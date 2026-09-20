use std::mem::ManuallyDrop;
use std::sync::LazyLock;

use arc_swap::ArcSwap;
use futures::Sink;
use futures::SinkExt;
use futures::Stream;
use futures::StreamExt;
use futures::channel::mpsc;
use hashbrown::HashMap;
use linkme::distributed_slice;
use log::warn;
use parking_lot::RwLock;
use slab::Slab;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use ygopro_data::complex::Complex;
use ygopro_data::constants::Color;
use ygopro_data::message::{ctos, stoc, gm};
use ygopro_handler::*;
use ygopro_handler::extract::*;

use crate::configuration::Configuration;
use crate::GlobalHandler;
use crate::message as srvpro;
use crate::mode::Provider;
use crate::mode::RoomConfiguration;

pub type Anymap = anymap3::Map<dyn std::any::Any + Send>;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static CORE_NAME: &'static str = "srvpro::core";

pub static ROOMS: LazyLock<RwLock<HashMap<String, RoomHost>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

pub enum Request {
    CTOS(Complex<ctos::Message>, usize),
    STOC(Complex<stoc::Message>, usize),
    Ex(srvpro::Message, usize),
    Command(&'static str, Box<dyn std::any::Any + Send>),
    Query(Box<dyn FnOnce(&Room, &Anymap) + Send>)
}

pub type ClientToServerHandler = TowerHandler<extract::Request<Complex<ctos::Message>, usize>, State, Response<ctos::Message>>;
pub type ClientToServerPrecursorHandler = TowerHandler<Complex<ctos::Message>, Player, Response<ctos::Message>>;
pub type ServerToClientHandler = TowerHandler<extract::Request<Complex<stoc::Message>, usize>, State, Response<stoc::Message>>;
pub type GameMessageHandler = TowerHandler<extract::Request<Complex<gm::Message>, usize>, ManuallyDrop<State>, Response<gm::Message>>;
pub type SrvproMessageHandler = TowerHandler<srvpro::Message, State, Response<srvpro::Message>>;
pub type CommandHandler = TowerHandler<extract::Request<Box<dyn std::any::Any + Send>, ()>, State, Response<()>>;

#[distributed_slice] pub static CTOS_HANDLERS: [fn() -> (u8, ClientToServerHandler)];
#[distributed_slice] pub static CTOS_PREHANDLERS: [fn() -> (u8, ClientToServerPrecursorHandler)];
#[distributed_slice] pub static STOC_HANDLERS: [fn() -> (u8, ServerToClientHandler)];
#[distributed_slice] pub static GM_HANDLERS: [fn() -> (u8, GameMessageHandler)];
#[distributed_slice] pub static SRVPRO_HANDLERS: [fn() -> (u8, SrvproMessageHandler)];
#[distributed_slice] pub static SRVPRO_COMMANDS: [fn() -> (&'static str, CommandHandler)];

type ClientToServerPrecursorProcessor = Processor<u8, Complex<ctos::Message>, Player, Response<ctos::Message>>;
type GameMessageProcessor = Processor<u8, extract::Request<Complex<gm::Message>, usize>, ManuallyDrop<State>, Response<gm::Message>, GameMessageHandler>;
type CommandProcessor = HashMap<&'static str, CommandHandler>;

static CTOS_PREPROCESSOR: LazyLock<ArcSwap<ClientToServerPrecursorProcessor>> = LazyLock::new(|| {
    let configuration = crate::configuration::get();
    ArcSwap::from_pointee(build_ctos_preprocessor(&configuration.enable_plugins))
});

fn build_ctos_preprocessor(enabled_plugins: &hashbrown::HashSet<String>) -> ClientToServerPrecursorProcessor {
    Processor::new_with_groups(&CTOS_PREHANDLERS, enabled_plugins, |h| { h.module_name }, |k| { *k == 0 })
}

#[handler(srvpro::ConfigurationChanged)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn on_configuration_changed() {
    let configuration = crate::configuration::get();
    CTOS_PREPROCESSOR.store(std::sync::Arc::new(build_ctos_preprocessor(&configuration.enable_plugins)));
}

/// Save connections in Room.
///
/// Player contains 4 channels with 8 endpoints. They're:
///          (A)        (B)
/// Network ----> Room ----> RoomProvider
///           (Processors)       |
/// Network <---- Room <---------┘
///          (D)        (C)
/// The Stream A is generated in server.rs and never saves in Player. It is fanned out to room in Room.add_player.
/// The Connection B is client_to_server_sink/stream_towards_provider in Player. It is fanned out to provider when join_game.
/// The Stream C is generated in provider.add(stream). It is executed in join_game and provider process then fanning out.
/// The Connection D is server_to_client_sink/stream_towards_network in Player. It is fanned out in Player::new.
#[derive(Debug)]
pub struct Player {
    pub client_to_server_sink_towards_provider: mpsc::UnboundedSender<Complex<ctos::Message>>,
    pub client_to_server_stream_towards_provider: Option<mpsc::UnboundedReceiver<Complex<ctos::Message>>>,
    pub server_to_client_sink_towards_network: mpsc::UnboundedSender<Complex<stoc::Message>>,
    pub server_to_client_stream_towards_network: Option<mpsc::UnboundedReceiver<Complex<stoc::Message>>>,
    pub states: Anymap,
}

impl Player {
    pub fn allocate() -> Self {
        let (ctos_sink, ctos_stream) = mpsc::unbounded();
        let (stoc_sink, stoc_stream) = mpsc::unbounded();
        Self {
            client_to_server_sink_towards_provider: ctos_sink,
            client_to_server_stream_towards_provider: Some(ctos_stream),
            server_to_client_sink_towards_network: stoc_sink,
            server_to_client_stream_towards_network: Some(stoc_stream),
            states: Anymap::new()
        }
    }

    pub fn new(mut network_sink: impl Sink<Complex<stoc::Message>> + Unpin + Send + 'static) -> Self {
        let mut player = Self::allocate();
        let mut stoc_stream = player.server_to_client_stream_towards_network.take().unwrap();
        tokio::spawn(async move { loop { tokio::select! {
            message = stoc_stream.next() => {
                if let Some(message) = message {
                    network_sink.send(message).await.ok();
                } else { break }
            }
        }}});
        player
    }

    pub fn run<S: Stream<Item = Complex<ctos::Message>> + Unpin + Send + 'static>(self, mut client_to_server_stream_from_network: S) {
        let processor = CTOS_PREPROCESSOR.load_full();
        let mut player = self;
        tokio::spawn(async move {
            while let Some(message) = client_to_server_stream_from_network.next().await {
                let key = message.message_key();
                let bundle = Bundle::new(message, player, Default::default());
                let Bundle { request, state, response, stop_flag: _ } = processor.process_bundle(bundle, key).await;
                player = state;
                match response {
                    Response::Continue => {player.client_to_server_sink_towards_provider.unbounded_send(request).ok();},
                    Response::Replace(message) => {player.client_to_server_sink_towards_provider.unbounded_send(Complex::from_message(message)).ok();} ,
                    Response::ReplaceMultiple(messages) => messages.into_iter().for_each(|message| { player.client_to_server_sink_towards_provider.unbounded_send(Complex::from_message(message)).ok(); }),
                    Response::Swallow => {},
                    Response::Terminate | Response::Kick => { break; }
                };
                if let Some(player_move) = player.states.get::<srvpro::PlayerMove>() {
                    let mut rooms  = ROOMS.write();
                    if let Some(room) = rooms.get_mut(&player_move.room_name) {
                        room.add_player(player, client_to_server_stream_from_network);
                    } else {
                        warn!("can't find room named {}, player will be dropped.", player_move.room_name)
                    };
                    break
                } else {
                    crate::process(srvpro::ClientRefused.into_message()).await;
                }
            }
        });
    }
      
    pub fn send_message(&self, message: &str, color: Color) {
        self.server_to_client_sink_towards_network.unbounded_send(Complex::from_message(stoc::Chat {
            player: color.into(),
            msg: ("[Server]: ".to_string() + message).into(),
        }.into())).ok();
    }
}

impl<Res, Message> FromRequest<extract::Request<Complex<Message>, usize>, State, Res> for &mut Player where Message: Send, Res: Send {
    fn from_request(bundle: &mut Bundle<extract::Request<Complex<Message>, usize>, State, Res>) -> Option<Self> {
        bundle.state.room.players.get_mut(bundle.request.extra).map(|p| unsafe { &mut *(p as *mut Player) })
    }
}

impl<Req, Res> FromRequest<Req, Player, Res> for &mut Player where Req: Send, Res: Send {
    fn from_request(bundle: &mut Bundle<Req, Player, Res>) -> Option<Self> {
        Some(unsafe { &mut *(&mut bundle.state as *mut Player) })
    }
}

impl ContainsMapMut for Player {
    fn get_map(&mut self) -> &mut anymap3::Map<dyn std::any::Any + Send> {
        &mut self.states
    }
}

pub struct Room {
    pub name: String,
    pub players: Slab<Player>,
    pub provider: Option<Provider>,
    pub configuration: crate::configuration::Configuration,
    pub request_sender: mpsc::UnboundedSender<Request>,
    pub request_receiver: Option<mpsc::UnboundedReceiver<Request>>
}

pub struct State {
    pub room: Room,
    pub states: Anymap
}

impl<Req, Res> FromRequest<Req, State, Res> for &mut Room where Req: Send, Res: Send {
    fn from_request(bundle: &mut Bundle<Req, State, Res>) -> Option<Self> {
        Some(unsafe { &mut *(&mut bundle.state.room as *mut Room) })
    }
}

impl ContainsMapMut for State {
    fn get_map(&mut self) -> &mut anymap3::Map<dyn std::any::Any + Send> {
        &mut self.states
    }
}

impl ContainsMap for State {
    fn get_map(&self) -> &anymap3::Map<dyn anymap3::CloneAny + Send> {
        // SAFETY: `Map<A>` wraps `HashMap<TypeId, Box<A>>`; `dyn CloneAny + Send`
        // and `dyn CloneAny + Send + Sync` share layout, and auto traits do not
        // change the vtable of `CloneAny` (anymap itself transmutes between them).
        // Dropping the `Sync` bound only widens the readable type.
        unsafe { &*(&self.room.configuration.configurations as *const anymap3::Map<dyn anymap3::CloneAny + Send + Sync> as *const anymap3::Map<dyn anymap3::CloneAny + Send>) }
    }
}

impl<Req, Res> FromRequest<Req, ManuallyDrop<State>, Res> for &mut Room where Req: Send, Res: Send {
    fn from_request(bundle: &mut Bundle<Req, ManuallyDrop<State>, Res>) -> Option<Self> {
        Some(unsafe { &mut *(&mut bundle.state.room as *mut Room) })
    }
}

impl<Res, Message> FromRequest<extract::Request<Complex<Message>, usize>, ManuallyDrop<State>, Res> for &mut Player where Message: Send, Res: Send {
    fn from_request(bundle: &mut Bundle<extract::Request<Complex<Message>, usize>, ManuallyDrop<State>, Res>) -> Option<Self> {
        bundle.state.room.players.get_mut(bundle.request.extra).map(|p| unsafe { &mut *(p as *mut Player) })
    }
}

pub struct Sender(pub mpsc::UnboundedSender<Request>);
impl<Req, Res> FromRequest<Req, State, Res> for Sender where Req: Send, Res: Send {
    fn from_request(bundle: &mut Bundle<Req, State, Res>) -> Option<Self> {
        Some(Sender(bundle.state.room.request_sender.clone()))
    }
}

impl Room {
    pub fn new(name: String) -> Self {
        let (request_sender, request_receiver) = mpsc::unbounded();
        Self {
            name,
            provider: None,
            players: Slab::new(),
            configuration: Configuration::empty(),
            request_sender,
            request_receiver: Some(request_receiver)
        }
    }

    pub fn run(self, configuration: Configuration, states: Anymap) -> JoinHandle<()> {
        let mut room = self;
        // Freeze the room's configuration here: the processors below are wired
        // from this snapshot, and later global changes never affect this room.
        room.configuration = configuration;
        let enabled = &room.configuration.enable_plugins;
        let ctos_processor = Processor::new_with_groups(&CTOS_HANDLERS, enabled, |h| { h.module_name }, |k| { *k == 0 });
        let stoc_processor = Processor::new_with_groups(&STOC_HANDLERS, enabled, |h| { h.module_name }, |k| { *k == 0 });
        let gm_processor = Processor::new_with_groups(&GM_HANDLERS, enabled, |h| { h.module_name }, |k| { *k == 0 });
        let srvpro_processor = Processor::new_with_groups(&SRVPRO_HANDLERS, enabled, |h| { h.module_name }, |k| { *k == 0 });
        let command_processor: CommandProcessor = SRVPRO_COMMANDS.iter().map(|build| build())
            .filter(|(_, handler)| enabled.contains(handler.module_name))
            .collect();
        let mut states: Anymap = states;
        states.insert(std::sync::Arc::new(gm_processor));
        tokio::spawn(async move {
            let Some(mut receiver) = room.request_receiver.take() else { return };
            while let Some(request) = receiver.next().await {
                match request {
                    Request::CTOS(complex, index) => {
                        let (returned_room, request, returned_states, response) = room.run_processor(&ctos_processor, extract::Request { message: complex, extra: index }, states).await;
                        room = returned_room;
                        states = returned_states;
                        match response {
                            Response::Continue => room.send(Request::CTOS(request.message, request.extra)),
                            Response::Replace(message) => room.send(Request::CTOS(Complex::from_message(message), request.extra)),
                            Response::ReplaceMultiple(messages) => messages.into_iter().map(|m| Request::CTOS(Complex::from_message(m), request.extra)).for_each(|m| room.send(m)),
                            Response::Swallow => {},
                            Response::Terminate => break,
                            Response::Kick => { room.players.try_remove(request.extra); }
                        }
                    },
                    Request::STOC(complex, index) => {
                        let (returned_room, request, returned_states, response) = room.run_processor(&stoc_processor, extract::Request { message: complex, extra: index }, states).await;
                        room = returned_room;
                        states = returned_states;
                        match response {
                            Response::Continue => room.send(Request::STOC(request.message, request.extra)),
                            Response::Replace(message) => room.send(Request::STOC(Complex::from_message(message), request.extra)),
                            Response::ReplaceMultiple(messages) => messages.into_iter().map(|m| Request::STOC(Complex::from_message(m), request.extra)).for_each(|m| room.send(m)),
                            Response::Swallow => {},
                            Response::Terminate => break,
                            Response::Kick => { room.players.try_remove(request.extra); }
                        }
                    },
                    Request::Ex(message, index) => {
                        let (returned_room, message, returned_states, response) = room.run_processor(&srvpro_processor, message, states).await;
                        room = returned_room;
                        states = returned_states;
                        match response {
                            Response::Terminate => break,
                            Response::Kick => { room.players.try_remove(index); },
                            _ => (),
                        }
                    },
                    Request::Command(key, message) => {
                        if let Some(handler) = command_processor.get(key) {
                            let state = State { room, states };
                            let bundle = Bundle::new(extract::Request { message, extra: () }, state, Default::default());
                            let bundle = handler.call(bundle).await;
                            let Bundle {
                                request: _,
                                state: State { room: returned_room, states: returned_states },
                                response,
                                stop_flag: _
                            } = bundle;
                            room = returned_room;
                            states = returned_states;
                            match response {
                                Response::Terminate => break,
                                _ => (),
                            }
                        } else {
                            warn!("Can't find command handler {}", key)
                        }
                    },
                    Request::Query(query) => query(&room, &states),
                }
            }
            room.run_processor(&srvpro_processor, srvpro::Terminate.into(), states).await;
        })
    }

    async fn run_processor<Req, Res, Handler>(self, processor: &ygopro_handler::Processor<u8, Req, State, Res, Handler>, request: Req, states: Anymap) -> (Self, Req, Anymap, Res)
    where Res: Default,
            Req: MessageKey<u8>,
            Handler: Call<Req, State, Res>
    {
        let state = State { room: self, states };
        let key = request.message_key();
        let bundle = Bundle::new(request, state, Default::default());
        let Bundle {
            request,
            state: State { room: returned_room, states: returned_states },
            response,
            stop_flag: _
        } = processor.process_bundle(bundle, key).await;
        (returned_room, request, returned_states, response)
    }

    pub fn send(&self, request: Request) {
        match request {
            Request::CTOS(message, index) => if let Some(p) = self.players.get(index) {
                p.client_to_server_sink_towards_provider.unbounded_send(message).ok();
            },
            Request::STOC(message, index) => if let Some(p) = self.players.get(index) {
                p.server_to_client_sink_towards_network.unbounded_send(message).ok();
            }
            Request::Ex(_, _) => warn!("Try to send an ex message."),
            Request::Command(_, _) => warn!("Try to send a command message"),
            Request::Query(_) => warn!("Try to send a query message")
        }
    }

    pub fn broadcast(&self, message: stoc::Message) {
        let complex = Complex::from_message(message);
        for player in self.players.iter() {
            player.1.server_to_client_sink_towards_network.unbounded_send(complex.clone()).ok();
        }
    }

    pub fn broadcast_message(&self, color: Color, message: &str) {
        self.broadcast(stoc::Chat {
            player: color.into(),
            msg: ("[Server]: ".to_string() + message).into(),
        }.into());
    }
}

#[handler(srvpro::DirectCTOS, module = "srvpro::core")]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_direct_ctos(room: &mut Room, direct_ctos: &srvpro::DirectCTOS) {
    let complex = Complex::from_message(direct_ctos.message.clone());
    match direct_ctos.target {
        Some(index) => room.send(Request::CTOS(complex, index)),
        None => for (_index, player) in room.players.iter() {
            player.client_to_server_sink_towards_provider.unbounded_send(complex.clone()).ok();
        },
    }
}

#[handler(srvpro::DirectSTOC, module = "srvpro::core")]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_direct_stoc(room: &mut Room, direct_stoc: &srvpro::DirectSTOC) {
    match direct_stoc.target {
        Some(index) => room.send(Request::STOC(Complex::from_message(direct_stoc.message.clone()), index)),
        None => room.broadcast(direct_stoc.message.clone())
    }
}

#[handler(stoc::GameMessage, module = "srvpro::core")]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
async fn game_message_handler(bundle: &mut Bundle<extract::Request<Complex<stoc::Message>, usize>, State, Response<stoc::Message>>) -> Response<stoc::Message> {
    let inner_bytes = bundle.request.message.data.slice(1..);
    let inner_request = extract::Request { message: Complex::new(inner_bytes), extra: bundle.request.extra };
    let inner_message_key = inner_request.message_key();
    let processor = bundle.state.states.get::<std::sync::Arc<GameMessageProcessor>>().cloned();
    let Some(processor) = processor else {
        warn!("no game message processor found for room {}", bundle.state.room.name);
        return Response::Continue;
    };
    let state = ManuallyDrop::new(unsafe { std::ptr::read(&bundle.state) });
    let inner_bundle: Bundle<extract::Request<Complex<gm::Message>, usize>, ManuallyDrop<State>, Response<gm::Message>> = Bundle::new(inner_request, state, Default::default());
    let mut result_bundle = processor.process_bundle(inner_bundle, inner_message_key).await;
    let state = unsafe { ManuallyDrop::take(&mut result_bundle.state) };
    unsafe { std::ptr::write(&mut bundle.state, state) };
    result_bundle.response.map(|m| m.into())
}

pub struct RoomHost {
    pub name: String,
    pub sender: mpsc::UnboundedSender<Request>,
    pub handler: JoinHandle<()>
}

impl RoomHost {
    pub(crate) fn allocate(name: String, configuration: Configuration, states: Anymap) -> Self {
        let room = Room::new(name.clone());
        let sender = room.request_sender.clone();
        let handler: JoinHandle<()> = room.run(configuration, states);
        Self { name, sender: sender.clone(), handler }
    }

    pub async fn new(pass: &str) -> mpsc::UnboundedSender<Request> {
        let room_configuration = RoomConfiguration::new(pass).await;
        let origin_name = room_configuration.origin_name;
        let mut states = room_configuration.states;
        states.insert(room_configuration.provider_configuration);
        let mut guard = ROOMS.write();
        if let Some(existing) = guard.get(&origin_name) {
            return existing.sender.clone();
        }
        let room: RoomHost = Self::allocate(room_configuration.name, room_configuration.srvpro_configuration, states);
        let sender = room.sender.clone();
        guard.insert(origin_name, room);
        sender.unbounded_send(Request::Ex(srvpro::CreateRoom.into(), 0)).ok();
        sender
    }

    pub fn query<T: Send + 'static>(&self, query: impl FnOnce(&Room, &Anymap) -> T + Send + 'static) -> oneshot::Receiver<T> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.sender.unbounded_send(Request::Query(Box::new(move |room, states| { response_sender.send(query(room, states)).ok(); }))).ok();
        response_receiver
    }

    pub fn add_player<S: Stream<Item = Complex<ctos::Message>> + Unpin + Send + 'static>(&mut self, player: Player, mut client_to_server_stream_from_network: S) {
        let room_sender = self.sender.clone();
        let (position_sender, position_receiver) = oneshot::channel();
        room_sender.unbounded_send(Request::Ex(srvpro::PlayerJoin { player: Some(player), position_sender: Some(position_sender) }.into(), 0)).ok();
        tokio::spawn(async move {
            let my_position = match position_receiver.await {
                Ok(position) => position,
                Err(err) => {
                    log::warn!("Failed to get position for player: {:?}", err);
                    return
                },
            };
            while let Some(message) = client_to_server_stream_from_network.next().await {
                room_sender.unbounded_send(Request::CTOS(message, my_position)).ok();
            }
            room_sender.unbounded_send(Request::Ex(srvpro::ClientLeave { position: my_position }.into(), my_position)).ok();
        });
    }
}
