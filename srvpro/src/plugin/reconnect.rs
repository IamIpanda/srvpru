//! A player whose connection dropped may take its seat back.
//!
//! The reconnecting client is never a member of the room while it is being checked. It
//! arrives as a fresh connection, so it is a bare `Player` in its own task, and the only
//! way to reach the room is a question sent as `Request::Query`.
//!
//! 1. `before(ctos::JoinGame)` refuses the join: no `PlayerMove` is set, so the message
//!    stream stays in the client's own task and every later message goes through this chain
//!    instead of the room. The room answers with the seat to take over, plus a fake room
//!    state that is written straight to the new connection.
//! 2. That fake state exists only to drag the client's UI into the room screen, so that it
//!    offers the "pick your deck and ready up" screen again. `JOIN_GAME` carries the room
//!    rules, `HS_PLAYER_ENTER` fills the roster, and `TYPE_CHANGE` must claim a seat number
//!    below 2: the client enables the ready checkbox of that very seat, and a higher number
//!    would turn it into an observer with no way to ready up. Which of the two seats it
//!    claims does not matter; none of this is the client's real state.
//! 3. `before(ctos::UpdateDeck)` is the only real check. The submitted deck must equal the
//!    one the seat submitted for the current duel, kept by `base::deck`. Equal: the client
//!    is routed into the room, where `before(srvpro::PlayerJoin)` moves it into the seat and
//!    replays the messages of the current stage. Different: the client is told to use its
//!    original deck and stays outside. Seat already gone: the client is kicked.
//! 4. `before(srvpro::ClientLeave)` keeps the seat as a hole instead of dropping it:
//!    `Disconnected` records the player under its origin name and the timer that later turns
//!    this deferred leave into a real one. With `allow_kick_reconnect` a client of the same
//!    origin name may also take over a seat that is still occupied.

use std::net::SocketAddr;
use std::sync::LazyLock;
use std::time::Duration;

use futures::channel::mpsc;
use hashbrown::HashMap;
use parking_lot::Mutex;
use tokio::sync::oneshot;

use ygopro_data::complex::Complex;
use ygopro_data::constants::Color;
use ygopro_data::constants::DuelStage;
use ygopro_data::constants::ErrorMessage;
use ygopro_data::constants::Mode;
use ygopro_data::constants::Netplayer;
use ygopro_data::constants::PlayerChange;
use ygopro_data::constants::PlayerChangeState;
use ygopro_data::data::Deck;
use ygopro_data::data::DeckError;
use ygopro_data::data::DeckErrorType;
use ygopro_data::message::HostInfo;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::Configuration;
use ygopro_derive::before;
use ygopro_derive::register_to;
use ygopro_handler::StopFlag;

use crate::message as srvpro;
use crate::plugin::base::stage::Stage;
use crate::plugin::random_match::RandomRoom;
use crate::plugin::register_dependencies;
use crate::plugin::virtual_password::OriginName;
use crate::room::*;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::name::NAME,
    crate::plugin::base::position::NAME,
    crate::plugin::base::deck::NAME,
    crate::plugin::base::stage::NAME
);

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "60")]
    pub wait_time: u64,
    #[config(default = "false")]
    pub auto_surrender_after_disconnect: bool,
    #[config(default = "false")]
    pub allow_kick_reconnect: bool,
}

struct DisconnectInfo {
    name: String,
    timeout: tokio::task::JoinHandle<()>,
}

static RANDOM_ROOM_OF_CLIENT: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Attachment)]
struct Disconnected {
    slots: hashbrown::HashMap<usize, DisconnectInfo>,
}

#[derive(Clone)]
struct Reconnecting {
    room_name: String,
    origin_name: String,
    position: usize,
    kick: bool,
}

impl<Req, State, Res> ygopro_handler::FromRequest<Req, State, Res> for Reconnecting
where Req: Send, State: Send + ygopro_handler::extract::ContainsMapMut, Res: Send,
{
    fn from_request(bundle: &mut ygopro_handler::Bundle<Req, State, Res>) -> Option<Self> {
        ygopro_handler::extract::ContainsMapMut::get_map(&mut bundle.state).get::<Reconnecting>().cloned()
    }
}

#[derive(Clone, Copy)]
enum DeckCheck {
    Accepted,
    Incorrect,
    Lost,
}

type Anymap = anymap3::Map<dyn std::any::Any + Send>;

fn origin_name(player: &Player) -> Option<String> {
    player.states.get::<OriginName>().map(|name| name.0.clone()).or_else(|| player.states.get::<String>().cloned())
}

fn client_key(player: &Player) -> Option<String> {
    let address = player.states.get::<SocketAddr>()?;
    let name = origin_name(player)?;
    Some(format!("{}:{}", address.ip(), name))
}

fn random_room_of(player: &Player) -> Option<String> {
    let key = client_key(player)?;
    let mut random_rooms = RANDOM_ROOM_OF_CLIENT.lock();
    let room_name = random_rooms.get(&key).cloned()?;
    if !ROOMS.read().contains_key(&room_name) {
        random_rooms.remove(&key);
        return None
    }
    Some(room_name)
}

fn room_sender(room_name: &str) -> Option<mpsc::UnboundedSender<Request>> {
    ROOMS.read().get(room_name).map(|host| host.sender.clone())
}

async fn ask<T>(sender: &mpsc::UnboundedSender<Request>, question: impl FnOnce(&Room, &Anymap) -> T + Send + 'static) -> Option<T> where T: Send + 'static {
    let (answer_sender, answer_receiver) = oneshot::channel();
    sender.unbounded_send(Request::Query(Box::new(move |room, states| { answer_sender.send(question(room, states)).ok(); }))).ok()?;
    answer_receiver.await.ok()
}

fn find_seat(room: &Room, states: &Anymap, key: &str, allow_kick_reconnect: bool) -> Option<(usize, bool)> {
    let disconnected = states.get::<Disconnected>();
    if let Some((&position, _)) = disconnected.and_then(|disconnected| disconnected.slots.iter().find(|(_, info)| info.name == key)) {
        if room.players.contains(position) { return Some((position, false)) }
    }
    if !allow_kick_reconnect { return None }
    room.players.iter().find_map(|(position, player)| {
        let is_player = matches!(player.states.get::<Netplayer>(), Some(Netplayer::Player(_)));
        (is_player && origin_name(player).as_deref() == Some(key)).then_some((position, true))
    })
}

fn welcome_messages(room: &Room, states: &Anymap, position: usize) -> Vec<stoc::Message> {
    let hostinfo = states.get::<HostInfo>().cloned().unwrap_or_default();
    let seat = room.players.get(position);
    let netplayer = seat.and_then(|player| player.states.get::<Netplayer>()).copied().unwrap_or(Netplayer::Unknown);
    let host = seat.and_then(|player| player.states.get::<bool>()).copied().unwrap_or(false);
    let mut messages = vec![
        stoc::Message::from(stoc::Chat { player: Color::Babyblue.into(), msg: "你有未完成的对局，即将重新连接，请选择你在本局决斗中使用的卡组并准备。".into() }),
        stoc::Message::from(stoc::JoinGame { info: hostinfo }),
        stoc::Message::from(stoc::TypeChange { player: netplayer, host }),
    ];
    for (_, player) in room.players.iter() {
        let name = player.states.get::<String>().cloned().unwrap_or_default();
        let pos = player.states.get::<Netplayer>().copied().unwrap_or(Netplayer::Unknown);
        messages.push(stoc::Message::from(stoc::HsPlayerEnter { name: name.into(), pos }));
    }
    messages
}

fn check_deck(room: &Room, states: &Anymap, key: &str, position: usize, kick: bool, deck: &Deck) -> DeckCheck {
    let Some(player) = room.players.get(position) else { return DeckCheck::Lost };
    if origin_name(player).as_deref() != Some(key) { return DeckCheck::Lost }
    let Some(duel_deck) = player.states.get::<Vec<Deck>>().and_then(|decks| decks.first()) else { return DeckCheck::Lost };
    if !kick && !states.get::<Disconnected>().map(|disconnected| disconnected.slots.contains_key(&position)).unwrap_or(false) { return DeckCheck::Lost }
    if duel_deck != deck { return DeckCheck::Incorrect }
    DeckCheck::Accepted
}

fn send_deck_incorrect(player: &mut Player, position: usize) {
    let mut deck_error = DeckError::new();
    deck_error.set_error_type(DeckErrorType::NotAvailable);
    let status = PlayerChange::new().with_state(PlayerChangeState::Notready).with_player(Netplayer::Player(position as u8));
    let messages = vec![
        stoc::Message::from(stoc::Chat { player: Color::Red.into(), msg: "卡组与断线前不一致，请使用原卡组重连。".into() }),
        stoc::Message::from(stoc::HsPlayerChange { status }),
        stoc::Message::from(stoc::ErrorMessage { err: ErrorMessage::DeckError(deck_error) }),
    ];
    for message in messages {
        player.server_to_client_sink_towards_network.unbounded_send(Complex::from_message(message)).ok();
    }
}

// In ClientLeave, we:
// Delay that message to when timer ends.
// Set Flag and let us remember here is a hole.
#[before(srvpro::ClientLeave)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn before_client_leave(room: &mut Room, client_leave: &mut srvpro::ClientLeave, states: &mut Anymap, disconnected: &mut Disconnected, stage: &mut Stage, stop: &mut StopFlag, configuration: Configuration) {
    if stage.stage == DuelStage::Begin || stage.stage == DuelStage::End { return }
    let position = client_leave.position;
    let Some(player) = room.players.get(position) else { return };
    if !matches!(player.states.get::<Netplayer>(), Some(Netplayer::Player(_))) { return }
    let Some(name) = origin_name(player) else { return };
    let display_name = player.states.get::<String>().cloned().unwrap_or_default();
    let key = states.get::<RandomRoom>().is_some().then(|| client_key(player)).flatten();
    // Is this ClientLeave a real leave?
    if disconnected.slots.contains_key(&position) {
        if room.players[position].states.get::<Reconnecting>().is_none() {
            // That's a real leave created by timer
            disconnected.slots.remove(&position);
            if let Some(key) = &key { RANDOM_ROOM_OF_CLIENT.lock().remove(key); }
            return
        }
        // He is submitting deck, and leaves.
        // TODO: timeout when submitting deck.
        room.players[position].states.remove::<Reconnecting>();
    }
    if let Some(key) = key {
        if RANDOM_ROOM_OF_CLIENT.lock().values().any(|target| *target == room.name) { return }
        RANDOM_ROOM_OF_CLIENT.lock().insert(key, room.name.clone());
    }
    if configuration.auto_surrender_after_disconnect {
        if states.get::<HostInfo>().map(|hostinfo| hostinfo.mode) != Some(Mode::Match) { return }
        if stage.stage == DuelStage::Dueling {
            room.players[position].client_to_server_sink_towards_provider.unbounded_send(Complex::from_message(ctos::Surrender.into())).ok();
        }
    }
    let sender = room.request_sender.clone();
    let timeout = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(configuration.wait_time)).await;
        sender.unbounded_send(Request::Ex(srvpro::ClientLeave { position }.into(), position)).ok();
    });
    disconnected.slots.insert(position, DisconnectInfo { name, timeout });
    // we can't use `name` here because name is with the password. We get the display name recorded by `base::Name` by here.
    room.broadcast_message(Color::Babyblue, &format!("{} 与服务器断开连接，{} 秒内未重连将被移出房间。", display_name, configuration.wait_time));
    stop.0 = true;
}

#[before(ctos::JoinGame, priority = 16)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
async fn before_join_game(player: &mut Player, join_game: &ctos::JoinGame, stop: &mut StopFlag) -> &'static str {
    let Some(origin_name) = origin_name(player) else { return "continue" };
    let room_name = random_room_of(player).unwrap_or_else(|| join_game.pass.to_string());
    let Some(room_sender) = room_sender(&room_name) else { return "continue" };
    let allow_kick_reconnect = crate::configuration::get_configuration::<Configuration>().map(|configuration| configuration.allow_kick_reconnect).unwrap_or(false);
    let client = player.server_to_client_sink_towards_network.clone();
    let question = {
        let key = origin_name.clone();
        move |room: &Room, states: &Anymap| {
            let (position, kick) = find_seat(room, states, &key, allow_kick_reconnect)?;
            for message in welcome_messages(room, states, position) {
                client.unbounded_send(Complex::from_message(message)).ok();
            }
            Some((position, kick))
        }
    };
    let Some(Some((position, kick))) = ask(&room_sender, question).await else { return "continue" };
    player.states.insert(Reconnecting { room_name, origin_name, position, kick });
    stop.0 = true;
    "cancel"
}

#[handler(ctos::UpdateDeck)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
async fn before_update_deck(player: &mut Player, update_deck: &ctos::UpdateDeck, reconnecting: Reconnecting) -> &'static str {
    let Some(room_sender) = room_sender(&reconnecting.room_name) else { return "kick" };
    let deck = update_deck.deck.clone();
    let target = reconnecting.clone();
    let question = move |room: &Room, states: &Anymap| check_deck(room, states, &target.origin_name, target.position, target.kick, &deck);
    match ask(&room_sender, question).await {
        Some(DeckCheck::Accepted) => {
            player.states.insert(srvpro::PlayerMove { room_name: reconnecting.room_name });
            "cancel"
        },
        Some(DeckCheck::Incorrect) => {
            send_deck_incorrect(player, reconnecting.position);
            "cancel"
        },
        _ => {
            player.send_message("重连失败，请重新加入房间。", Color::Red);
            "kick"
        },
    }
}

#[before(srvpro::PlayerJoin, priority = 5)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn before_player_join(room: &mut Room, player_join: &mut srvpro::PlayerJoin, disconnected: &mut Disconnected, stage: &mut Stage, stop: &mut StopFlag) {
    let Some(Reconnecting { position, kick, .. }) = player_join.player.as_ref().and_then(|player| player.states.get::<Reconnecting>()) else { return };
    let position = *position;
    let kick = *kick;
    if !room.players.contains(position) { return }
    let Some(mut player) = player_join.player.take() else { return };
    let Some(position_sender) = player_join.position_sender.take() else { return };
    if let Some(key) = client_key(&room.players[position]) { RANDOM_ROOM_OF_CLIENT.lock().remove(&key); }
    if let Some(info) = disconnected.slots.remove(&position) {
        info.timeout.abort();
    } else if kick {
        room.send(Request::STOC(Complex::from_message(stoc::Chat { player: Color::Red.into(), msg: "你的对局已被同一玩家的新连接顶替。".into() }.into()), position));
    }
    let (seat_sink, old_states) = {
        let old_player = &mut room.players[position];
        (old_player.client_to_server_sink_towards_provider.clone(), std::mem::replace(&mut old_player.states, Default::default()))
    };
    let name = old_states.get::<String>().cloned().unwrap_or_default();
    player.client_to_server_sink_towards_provider = seat_sink;
    player.states = old_states;
    room.players[position] = player;
    if let Some(mut buffered) = room.players[position].client_to_server_stream_towards_provider.take() {
        while let Ok(message) = buffered.try_recv() {
            room.request_sender.unbounded_send(Request::CTOS(message, position)).ok();
        }
    }
    position_sender.send(position).ok();
    match stage.stage {
        DuelStage::Finger => {
            room.send(Request::STOC(Complex::from_message(stoc::DuelStart.into()), position));
            room.send(Request::STOC(Complex::from_message(stoc::SelectHand.into()), position));
        }
        DuelStage::Firstgo => {
            room.send(Request::STOC(Complex::from_message(stoc::DuelStart.into()), position));
            room.send(Request::STOC(Complex::from_message(stoc::SelectTp.into()), position));
        }
        DuelStage::Siding => {
            room.send(Request::STOC(Complex::from_message(stoc::DuelStart.into()), position));
            room.send(Request::STOC(Complex::from_message(stoc::ChangeSide.into()), position));
        }
        _ => {
            room.players[position].client_to_server_sink_towards_provider.unbounded_send(Complex::from_message(ctos::RequestField.into())).ok();
        }
    }
    room.broadcast_message(Color::Babyblue, &format!("{} 已重连。", name));
    stop.0 = true;
}
