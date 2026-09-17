use std::net::IpAddr;
use std::net::SocketAddr;
use std::sync::LazyLock;
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::Extension;
use axum::extract::Query;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::Message as WsMessage;
use axum::extract::ws::WebSocket;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use futures::future::join_all;
use linkme::distributed_slice;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio::time::timeout;
use ygopro_data::constants::DuelStage;
use ygopro_data::constants::Netplayer;
use ygopro_data::message::stoc;
use ygopro_derive::Configuration;
use ygopro_derive::after;
use ygopro_derive::register_to;

use crate::auth;
use crate::auth::Identity;
use srvpro::configuration;
use srvpro::message as srvpro_message;
use srvpro::mode::RoomProviderConfiguration;
use srvpro::mode::Tag;
use srvpro::plugin::base::lp::Lp;
use srvpro::plugin::base::score::Score;
use srvpro::plugin::base::stage::Stage;
use srvpro::plugin::register_dependencies;
use srvpro::room::Anymap;
use srvpro::room::Player;
use srvpro::room::ROOMS;
use srvpro::room::Room;
use srvpro::room::SRVPRO_HANDLERS;
use srvpro::room::STOC_HANDLERS;
use srvpro::room::ServerToClientHandler;
use srvpro::room::SrvproMessageHandler;

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_dependencies!(
    srvpro::plugin::base::name::NAME,
    srvpro::plugin::base::position::NAME,
    srvpro::plugin::base::stage::NAME,
    srvpro::plugin::base::lp::NAME,
    srvpro::plugin::base::score::NAME
);

crate::register_router!(ROOMLIST_ROUTER, build_roomlist_router);

const PERMISSION: &str = "get_rooms";

#[derive(Clone, Configuration)]
#[config(sync, prefix = "http", register_to = "srvpro::plugin::CONFIGURATIONS")]
pub struct Configuration {
    #[config(default = "true")]
    pub show_ip: bool,
    #[config(default = "true")]
    pub show_info: bool,
    pub public_roomlist: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        Self { show_ip: true, show_info: true, public_roomlist: false }
    }
}

const QUERY_TIMEOUT: Duration = Duration::from_secs(1);
const BROADCAST_CAPACITY: usize = 64;

/// What the caller is allowed to see; fixed once per request.
#[derive(Clone, Copy)]
struct View {
    authenticated: bool,
    show_ip: bool,
    show_info: bool,
}

#[derive(Clone)]
struct RoomSnapshot {
    name: String,
    mode: Option<u8>,
    users: Vec<UserSnapshot>,
    started: bool,
}

#[derive(Clone)]
struct UserSnapshot {
    name: String,
    pos: u8,
    ip: Option<IpAddr>,
    status: Option<StatusSnapshot>,
}

#[derive(Clone)]
struct StatusSnapshot {
    score: u32,
    lp: i32,
    cards: Option<u8>,
}

#[derive(Serialize)]
struct RoomSummaries {
    rooms: Vec<RoomSummary>,
}

#[derive(Serialize)]
struct RoomSummary {
    roomid: Option<String>,
    roomname: String,
    roommode: Option<u8>,
    needpass: String,
    users: Vec<UserSummary>,
    istart: String,
}

#[derive(Serialize)]
struct UserSummary {
    id: String,
    name: String,
    ip: Option<String>,
    status: Option<StatusSummary>,
    pos: u8,
}

#[derive(Serialize)]
struct StatusSummary {
    score: u32,
    lp: i32,
    cards: Option<u8>,
}

#[derive(Clone)]
enum Event {
    Init(Vec<RoomSnapshot>),
    Create(RoomSnapshot),
    Update(RoomSnapshot),
    Delete { name: String, started: bool },
}

#[derive(Serialize)]
#[serde(tag = "event", content = "data", rename_all = "kebab-case")]
enum EventSummary {
    Init(Vec<RoomSummary>),
    Create(RoomSummary),
    Update(RoomSummary),
    Delete { name: String, started: bool },
}

impl RoomSnapshot {
    fn summarize(&self, view: View) -> RoomSummary {
        RoomSummary {
            roomid: None,
            roomname: if view.authenticated { self.name.clone() } else { self.name.split('$').next().unwrap_or_default().to_string() },
            roommode: self.mode,
            needpass: self.name.contains('$').to_string(),
            users: self.users.iter().map(|user| user.summarize(view)).collect(),
            istart: if self.started { "start" } else { "wait" }.to_string(),
        }
    }
}

impl UserSnapshot {
    fn summarize(&self, view: View) -> UserSummary {
        UserSummary {
            id: "-1".to_string(),
            name: self.name.clone(),
            ip: if view.show_ip && view.authenticated { self.ip.map(|ip| ip.to_string()) } else { None },
            status: if view.show_info { self.status.as_ref().map(StatusSnapshot::summarize) } else { None },
            pos: self.pos,
        }
    }
}

impl StatusSnapshot {
    fn summarize(&self) -> StatusSummary {
        StatusSummary { score: self.score, lp: self.lp, cards: self.cards }
    }
}

fn summarize(event: &Event, view: View) -> EventSummary {
    match event {
        Event::Init(rooms) => EventSummary::Init(rooms.iter().map(|room| room.summarize(view)).collect()),
        Event::Create(room) => EventSummary::Create(room.summarize(view)),
        Event::Update(room) => EventSummary::Update(room.summarize(view)),
        Event::Delete { name, started } => EventSummary::Delete { name: name.split('$').next().unwrap_or_default().to_string(), started: *started },
    }
}

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Filter {
    #[default]
    Waiting,
    Started,
}

impl Filter {
    fn holds(self, started: bool) -> bool {
        (self == Filter::Started) == started
    }
}

#[derive(Deserialize)]
struct RoomlistQuery {
    #[serde(default)]
    filter: Filter,
}

/// Rooms publish their lifecycle changes here; the subscribers are the WebSocket clients.
/// The HTTP endpoint queries the rooms directly, so this stream never becomes a second
/// source of truth that could drift from the rooms themselves.
static EVENTS: LazyLock<broadcast::Sender<Event>> = LazyLock::new(|| broadcast::channel(BROADCAST_CAPACITY).0);

fn publish(event: Event) {
    EVENTS.send(event).ok();
}

fn passes(event: &Event, filter: Filter) -> bool {
    match event {
        Event::Init(_) => true,
        Event::Create(room) | Event::Update(room) => filter.holds(room.started),
        Event::Delete { started, .. } => filter.holds(*started),
    }
}

fn filtered(rooms: Vec<RoomSnapshot>, filter: Filter) -> Vec<RoomSnapshot> {
    rooms.into_iter().filter(|room| filter.holds(room.started)).collect()
}

fn build_roomlist_router() -> Router {
    auth::optional(PERMISSION, Router::new()
        .route("/api/getrooms", get(get_rooms))
        .route("/roomlist/ws", get(room_list_ws)))
}

async fn get_rooms(identity: Option<Extension<Identity>>) -> Result<Json<RoomSummaries>, StatusCode> {
    let view = view(identity.as_deref());
    if !view.authenticated && !public_roomlist() { return Err(StatusCode::FORBIDDEN) }
    Ok(Json(RoomSummaries { rooms: collect().await.iter().map(|room| room.summarize(view)).collect() }))
}

async fn room_list_ws(upgrade: WebSocketUpgrade, Query(query): Query<RoomlistQuery>, identity: Option<Extension<Identity>>) -> Result<Response, StatusCode> {
    let view = view(identity.as_deref());
    if !view.authenticated && !public_roomlist() { return Err(StatusCode::FORBIDDEN) }
    Ok(upgrade.on_upgrade(move |socket| serve(socket, query.filter, view)))
}

fn view(identity: Option<&Identity>) -> View {
    let configuration = configuration::get().configurations.get::<Configuration>().cloned().unwrap_or_default();
    View {
        authenticated: identity.is_some(),
        show_ip: configuration.show_ip,
        show_info: configuration.show_info,
    }
}

fn public_roomlist() -> bool {
    configuration::get().configurations.get::<Configuration>().cloned().unwrap_or_default().public_roomlist
}

async fn serve(mut socket: WebSocket, filter: Filter, view: View) {
    // Subscribing before the snapshot is what keeps the stream gap-free: any change that
    // lands while the rooms are being queried is already queued and will follow the
    // snapshot. It may repeat what the snapshot shows, which is harmless — an `Update`
    // carries the whole room, so re-applying it is idempotent.
    let mut events = EVENTS.subscribe();
    let init = summarize(&Event::Init(filtered(collect().await, filter)), view);
    if send(&mut socket, &init).await.is_err() { return }
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(event) => if passes(&event, filter) && send(&mut socket, &summarize(&event, view)).await.is_err() { break },
                // A lagging subscriber lost events it cannot replay, so re-send the state instead.
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    let init = summarize(&Event::Init(filtered(collect().await, filter)), view);
                    if send(&mut socket, &init).await.is_err() { break }
                },
                Err(broadcast::error::RecvError::Closed) => break,
            },
            incoming = socket.recv() => match incoming {
                Some(Ok(WsMessage::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(_)) => (),
            },
        }
    }
}

async fn send(socket: &mut WebSocket, event: &EventSummary) -> Result<(), axum::Error> {
    let Ok(payload) = serde_json::to_string(event) else { return Ok(()) };
    socket.send(WsMessage::text(payload)).await
}

fn snapshot_user(player: &Player, in_duel: bool, status: StatusContext<'_>) -> Option<UserSnapshot> {
    let name = player.states.get::<String>()?.clone();
    let pos = u8::from(*player.states.get::<Netplayer>()?);
    let ip = player.states.get::<SocketAddr>().map(|addr| addr.ip());
    let status = (in_duel && pos != 7).then(|| status.of(pos));
    Some(UserSnapshot { name, pos, ip, status })
}

/// The per-room values a user's status falls back to before the duel reports its own.
#[derive(Clone, Copy)]
struct StatusContext<'a> {
    start_lp: i32,
    start_hand: u8,
    tag: bool,
    score: Option<&'a Score>,
    lp: Option<&'a Lp>,
}

impl StatusContext<'_> {
    fn of(&self, pos: u8) -> StatusSnapshot {
        // A tag duel splits the two core players across positions 0/2 and 1/3, so the
        // core index is the position parity rather than the position itself.
        let core_player = pos as usize % 2;
        StatusSnapshot {
            score: self.score.map(|score| score.winners.iter().filter(|winner| **winner == Some(Netplayer::Player(pos))).count() as u32).unwrap_or(0),
            lp: self.lp.map(|lp| lp.lp[core_player]).unwrap_or(self.start_lp),
            cards: (!self.tag).then_some(self.start_hand),
        }
    }
}

fn room_snapshot(room: &Room, states: &Anymap) -> RoomSnapshot {
    let hostinfo = states.get::<RoomProviderConfiguration>().map(|provider_configuration| provider_configuration.hostinfo.clone()).unwrap_or_default();
    let in_duel = started(states);
    let status = StatusContext {
        start_lp: hostinfo.start_lp as i32,
        start_hand: hostinfo.start_hand,
        tag: states.get::<Tag>().is_some(),
        score: states.get::<Score>(),
        lp: states.get::<Lp>(),
    };
    RoomSnapshot {
        name: room.name.clone(),
        mode: Some(u8::from(hostinfo.mode)),
        users: room.players.iter().filter_map(|(_, player)| snapshot_user(player, in_duel, status)).collect(),
        started: in_duel,
    }
}

fn started(states: &Anymap) -> bool {
    states.get::<Stage>().is_some_and(|stage| stage.stage != DuelStage::Begin)
}

async fn collect() -> Vec<RoomSnapshot> {
    let queries: Vec<_> = ROOMS.read().values().map(|host| timeout(QUERY_TIMEOUT, host.query(room_snapshot))).collect();
    join_all(queries).await.into_iter().flatten().flatten().collect()
}

#[after(srvpro_message::CreateRoom)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_create_room(room: &mut Room, states: &mut Anymap) {
    publish(Event::Create(room_snapshot(room, states)));
}

#[after(srvpro_message::PlayerJoin)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_player_join(room: &mut Room, states: &mut Anymap) {
    publish(Event::Update(room_snapshot(room, states)));
}

#[after(srvpro_message::ClientLeave)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_client_leave(room: &mut Room, states: &mut Anymap) {
    publish(Event::Update(room_snapshot(room, states)));
}

#[after(srvpro_message::Terminate)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_terminate(room: &mut Room, states: &mut Anymap) {
    publish(Event::Delete { name: room.name.clone(), started: started(states) });
}

#[after(stoc::TypeChange)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_type_change(room: &mut Room, states: &mut Anymap) {
    publish(Event::Update(room_snapshot(room, states)));
}

#[after(stoc::SelectHand)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_select_hand(room: &mut Room, states: &mut Anymap) {
    publish(Event::Update(room_snapshot(room, states)));
}

#[after(stoc::SelectTp)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_select_tp(room: &mut Room, states: &mut Anymap) {
    publish(Event::Update(room_snapshot(room, states)));
}

#[after(stoc::DuelStart)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_start(room: &mut Room, states: &mut Anymap) {
    publish(Event::Update(room_snapshot(room, states)));
}

#[after(stoc::ChangeSide)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_change_side(room: &mut Room, states: &mut Anymap) {
    publish(Event::Update(room_snapshot(room, states)));
}
