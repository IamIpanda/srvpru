use std::time::Duration;

use ygopro_data::complex::Complex;
use ygopro_data::constants::Color;
use ygopro_data::constants::DuelStage;
use ygopro_data::constants::Netplayer;
use ygopro_data::data::DeckError;
use ygopro_data::data::DeckErrorType;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::Configuration;
use ygopro_derive::before;
use ygopro_derive::handler;
use ygopro_derive::register_to;
use ygopro_handler::StopFlag;

use crate::message as srvpro;
use crate::plugin::base::stage::Stage;
use crate::plugin::register_dependencies;
use crate::room::Request;
use crate::room::Room;
use crate::room::CTOS_HANDLERS;
use crate::room::CTOS_PREHANDLERS;
use crate::room::SRVPRO_HANDLERS;
use crate::room::ClientToServerHandler;
use crate::room::ClientToServerPrecursorHandler;
use crate::room::SrvproMessageHandler;

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
}

struct DisconnectInfo {
    position: usize,
    timeout: tokio::task::JoinHandle<()>,
}

#[derive(Attachment)]
struct Disconnected {
    slots: hashbrown::HashMap<String, DisconnectInfo>,
}

struct Reconnecting(String);

type Anymap = anymap3::Map<dyn std::any::Any + Send>;

/// The precursor phase stashes the announced name here, so the join handler
/// can match the connection against disconnect slots before the room assigns
/// it a fresh slot.
struct PendingName(String);

#[handler(ctos::PlayerInfo, priority = 251)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
fn on_player_info(states: &mut Anymap, player_info: &ctos::PlayerInfo) {
    states.insert(PendingName(player_info.name.to_string()));
}

#[before(srvpro::ClientLeave)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn before_client_leave(room: &mut Room, client_leave: &mut srvpro::ClientLeave, disconnected: &mut Disconnected, stage: &mut Stage, stop: &mut StopFlag, configuration: Configuration) {
    let position = client_leave.position;
    if stage.stage == DuelStage::Begin { return }
    // Surrendering on disconnect means the seat is released at once: the core
    // sees the stream EOF and declares the opponent the winner.
    if configuration.auto_surrender_after_disconnect { return }
    let Some(player) = room.players.get(position) else { return };
    if !matches!(player.states.get::<Netplayer>(), Some(Netplayer::Player(_))) { return }
    let Some(player_name) = player.states.get::<String>().cloned() else { return };
    if let Some(info) = disconnected.slots.get(&player_name) {
        if info.position == position {
            if room.players[position].states.get::<Reconnecting>().is_some() {
                let sender = room.request_sender.clone();
                let timeout = tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(configuration.wait_time)).await;
                    sender.unbounded_send(Request::Ex(srvpro::ClientLeave { position }.into(), position)).ok();
                });
                disconnected.slots.insert(player_name.clone(), DisconnectInfo { position, timeout });
                stop.0 = true;
                return
            }
            disconnected.slots.remove(&player_name);
        }
        return
    }
    let sender = room.request_sender.clone();
    let timeout = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(configuration.wait_time)).await;
        sender.unbounded_send(Request::Ex(srvpro::ClientLeave { position }.into(), position)).ok();
    });
    disconnected.slots.insert(player_name.clone(), DisconnectInfo { position, timeout });
    room.broadcast_message(Color::Babyblue, &format!("{player_name} 与服务器断开连接，{} 秒内未重连将被移出房间。", configuration.wait_time));
    stop.0 = true;
}

/// Adopt the disconnected seat before the room assigns the joining connection
/// a fresh slot: the network forwarding task resolves its position through the
/// join's oneshot, so answering with the seat position keeps every later
/// client-to-server message routed to the seat, and the provider's
/// server-to-client task already targets the seat.
#[before(srvpro::PlayerJoin)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn before_player_join(room: &mut Room, player_join: &mut srvpro::PlayerJoin, disconnected: &mut Disconnected, stop: &mut StopFlag) {
    let Some(player_name) = player_join.player.as_ref().and_then(|player| player.states.get::<PendingName>()).map(|name| name.0.clone()) else { return };
    let Some(info) = disconnected.slots.get(&player_name) else { return };
    let position = info.position;
    if !room.players.contains(position) { return }
    if room.players[position].states.get::<Reconnecting>().is_some() { return }
    let Some(mut player) = player_join.player.take() else { return };
    let Some(position_sender) = player_join.position_sender.take() else { return };
    info.timeout.abort();
    let (seat_sink, old_states) = {
        let old_player = &mut room.players[position];
        (old_player.client_to_server_sink_towards_provider.clone(), std::mem::replace(&mut old_player.states, Default::default()))
    };
    player.client_to_server_sink_towards_provider = seat_sink;
    player.states = old_states;
    player.states.insert(Reconnecting(player_name));
    room.players[position] = player;
    if let Some(mut provider_stream) = room.players[position].client_to_server_stream_towards_provider.take() {
        while let Ok(message) = provider_stream.try_recv() {
            room.request_sender.unbounded_send(Request::CTOS(message, position)).ok();
        }
    }
    position_sender.send(position).ok();
    stop.0 = true;
}

#[before(ctos::JoinGame)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn on_join_game(room: &mut Room, index: usize, stop: &mut StopFlag) {
    let Some(player) = room.players.get(index) else { return };
    if player.states.get::<Reconnecting>().is_none() { return }
    room.send(Request::STOC(Complex::from_message(stoc::JoinGame { info: Default::default() }.into()), index));
    let host = player.states.get::<bool>().copied().unwrap_or(false);
    room.send(Request::STOC(Complex::from_message(stoc::TypeChange { player: Netplayer::Player(index as u8), host }.into()), index));
    for (_, player) in room.players.iter() {
        let name = player.states.get::<String>().cloned().unwrap_or_default();
        let pos = player.states.get::<Netplayer>().copied().unwrap_or(Netplayer::Unknown);
        room.send(Request::STOC(Complex::from_message(stoc::HsPlayerEnter { name: name.into(), pos }.into()), index));
    }
    stop.0 = true;
}

#[before(ctos::UpdateDeck)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn on_update_deck(room: &mut Room, index: usize, update_deck: &ctos::UpdateDeck, disconnected: &mut Disconnected, stage: &mut Stage, stop: &mut StopFlag) {
    let Some(player) = room.players.get(index) else { return };
    let Some(Reconnecting(player_name)) = player.states.get::<Reconnecting>() else { return };
    let player_name = player_name.clone();
    let Some(deckbuf) = player.states.get::<Vec<ygopro_data::data::Deck>>().and_then(|decks| decks.first()) else { return };
    if deckbuf.clone() != update_deck.deck {
        room.send(Request::STOC(Complex::from_message(stoc::Chat { player: Color::Red.into(), msg: "卡组与断线前不一致，请使用原卡组重连。".into() }.into()), index));
        let mut deck_error = DeckError::new();
        deck_error.set_error_type(DeckErrorType::NotAvailable);
        room.send(Request::STOC(Complex::from_message(stoc::ErrorMessage { err: ygopro_data::constants::ErrorMessage::DeckError(deck_error) }.into()), index));
        stop.0 = true;
        return;
    }
    if let Some(info) = disconnected.slots.remove(&player_name) {
        info.timeout.abort();
    }
    room.broadcast_message(Color::Babyblue, &format!("{player_name} 已重连。"));
    match stage.stage {
        DuelStage::Finger => {
            room.send(Request::STOC(Complex::from_message(stoc::DuelStart.into()), index));
            room.send(Request::STOC(Complex::from_message(stoc::SelectHand.into()), index));
        }
        DuelStage::Firstgo => {
            room.send(Request::STOC(Complex::from_message(stoc::DuelStart.into()), index));
            room.send(Request::STOC(Complex::from_message(stoc::SelectTp.into()), index));
        }
        DuelStage::Siding => {
            room.send(Request::STOC(Complex::from_message(stoc::DuelStart.into()), index));
            room.send(Request::STOC(Complex::from_message(stoc::ChangeSide.into()), index));
        }
        _ => {
            room.players[index].client_to_server_sink_towards_provider.unbounded_send(Complex::from_message(ctos::RequestField.into())).ok();
        }
    }
    room.players[index].states.remove::<Reconnecting>();
    stop.0 = true;
}
