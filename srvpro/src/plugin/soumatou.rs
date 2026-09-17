//! Keep every remote room recorded and watchable halfway through a duel.
//!
//! The external ygopro binary treats joins carrying the password
//! "the Big Brother" as its cache recorder and feeds them the full stream,
//! but it never replays history to spectators arriving after the duel
//! started. This plugin parks such a client in every remote room, records
//! its game messages and replays them to late observers, mirroring what the
//! original srvpro did for halfway watching. Embedded duels already replay
//! history through the in-process soumatou plugin, so for them this plugin
//! only makes sure that plugin stays enabled.

use futures::StreamExt;

use ygopro::plugin::version_check::PRO_VERSION;
use ygopro_data::complex::Complex;
use ygopro_data::constants::Netplayer;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::after;
use ygopro_derive::before;
use ygopro_derive::handler;
use ygopro_derive::register_to;
use ygopro_handler::All;
use ygopro_handler::RoomProvider;
use ygopro_handler::extract::Response;

use crate::message as srvpro;
use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::Provider;
use crate::mode::RemoteProvider;
use crate::mode::RoomConfiguration;
use crate::room::ClientToServerPrecursorHandler;
use crate::room::CTOS_PREHANDLERS;
use crate::room::Player;
use crate::room::Request;
use crate::room::Room;
use crate::room::SRVPRO_HANDLERS;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;
use crate::room::SrvproMessageHandler;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

const BIG_BROTHER: &str = "the Big Brother";

#[derive(Default, Attachment)]
struct BigBrother {
    position: Option<usize>,
    started: bool,
    history: Vec<Complex<stoc::Message>>,
}
#[handler(All)]
#[register_to(MODES)]
fn enable_embedded_soumatou(config: &mut RoomConfiguration) {
    config.ygopru_configuration.enable_plugin(ygopro::plugin::soumatou::NAME);
}

#[before(ctos::JoinGame)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
fn refuse_recorder_impersonation(join_game: &ctos::JoinGame) -> Response<ctos::Message> {
    let pass = &*join_game.pass;
    if pass == BIG_BROTHER { Response::Kick } else { Response::Continue }
}

#[after(srvpro::CreateProvider)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn spawn_big_brother(room: &mut Room, _: RemoteProvider, big_brother: &mut BigBrother) {
    if big_brother.position.is_some() { return }
    if !matches!(room.provider.as_ref(), Some(Provider::Remote(_))) { return }
    let mut player = Player::allocate();
    drop(player.server_to_client_stream_towards_network.take());
    player.client_to_server_sink_towards_provider.unbounded_send(Complex::from_message(ctos::Message::PlayerInfo(ctos::PlayerInfo { name: BIG_BROTHER.into() }))).ok();
    player.client_to_server_sink_towards_provider.unbounded_send(Complex::from_message(ctos::Message::JoinGame(ctos::JoinGame { version: *PRO_VERSION, gameid: 0, pass: BIG_BROTHER.into() }))).ok();
    player.client_to_server_sink_towards_provider.unbounded_send(Complex::from_message(ctos::Message::HsToObserver(ctos::HsToObserver))).ok();
    let Some(client_to_server_stream) = player.client_to_server_stream_towards_provider.take() else { return };
    let position = room.players.insert(player);
    big_brother.position = Some(position);
    let Some(provider) = room.provider.as_mut() else { return };
    let mut server_to_client_stream = provider.add(client_to_server_stream);
    let request_sender = room.request_sender.clone();
    tokio::spawn(async move {
        while let Some(message) = server_to_client_stream.next().await {
            request_sender.unbounded_send(Request::STOC(message, position)).ok();
        }
    });
}

#[handler(stoc::GameMessage)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn record_history(index: usize, message: &Complex<stoc::Message>, big_brother: &mut BigBrother) {
    if Some(index) != big_brother.position { return }
    big_brother.started = true;
    big_brother.history.push(message.clone());
}

#[handler(stoc::TypeChange)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn replay_history_to_late_observer(room: &mut Room, index: usize, type_change: &stoc::TypeChange, big_brother: &mut BigBrother) {
    if !big_brother.started { return }
    if !matches!(type_change.player, Netplayer::Observer(_)) { return }
    if Some(index) == big_brother.position { return }
    for message in &big_brother.history {
        room.send(Request::STOC(message.clone(), index));
    }
}
