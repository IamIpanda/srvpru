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
use futures::channel::mpsc;
use tokio::sync::broadcast;

use ygopro::plugin::version_check::PRO_VERSION;
use ygopro_data::complex::Complex;
use ygopro_data::constants::DuelStage;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::after;
use ygopro_derive::before;
use ygopro_derive::handler;
use ygopro_derive::register_to;
use ygopro_handler::All;
use ygopro_handler::StopFlag;

use crate::message as srvpro;
use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::common::NoWatch;
use crate::mode::RemoteProvider;
use crate::mode::RoomConfiguration;
use crate::plugin::base::stage::Stage;
use crate::room::ClientToServerPrecursorHandler;
use crate::room::CTOS_PREHANDLERS;
use crate::room::Player;
use crate::room::Request;
use crate::room::Room;
use crate::room::SRVPRO_HANDLERS;
use crate::room::SrvproMessageHandler;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

const BIG_BROTHER: &str = "the Big Brother";
const BROADCAST_CAPACITY: usize = 1024;

#[derive(Attachment)]
#[attachment(no_default)]
struct BigBrother {
    _position: usize,
    client_to_server_sink: mpsc::UnboundedSender<Complex<ctos::Message>>,
    recorder: Recorder,
}

struct Recorder {
    join: mpsc::UnboundedSender<mpsc::UnboundedSender<Complex<stoc::Message>>>,
}

#[handler(All)]
#[register_to(MODES)]
fn enable_embedded_soumatou(config: &mut RoomConfiguration) {
    config.ygopru_configuration.enable_plugin(ygopro::plugin::soumatou::NAME);
}

#[before(ctos::JoinGame)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
fn refuse_recorder_impersonation(join_game: &ctos::JoinGame) -> &'static str {
    let pass = &*join_game.pass;
    if pass == BIG_BROTHER { "kick" } else { "continue" }
}

#[after(srvpro::CreateProvider)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn spawn_big_brother(room: &mut Room, states: &mut crate::room::Anymap, _: RemoteProvider) {
    if states.get::<NoWatch>().is_some() { return }
    if states.get::<BigBrother>().is_some() { return }
    let mut player = Player::allocate();
    let Some(mut server_to_client_stream) = player.server_to_client_stream_towards_network.take() else { return };
    let client_to_server_sink = player.client_to_server_sink_towards_provider.clone();
    let position = room.players.insert(player);
    room.request_sender.unbounded_send(Request::CTOS(Complex::from_message(ctos::Message::PlayerInfo(ctos::PlayerInfo { name: BIG_BROTHER.into() })), position)).ok();
    room.request_sender.unbounded_send(Request::CTOS(Complex::from_message(ctos::Message::JoinGame(ctos::JoinGame { version: *PRO_VERSION, gameid: 0, pass: BIG_BROTHER.into() })), position)).ok();
    room.request_sender.unbounded_send(Request::CTOS(Complex::from_message(ctos::Message::HsToObserver(ctos::HsToObserver)), position)).ok();
    let (join, mut joining) = mpsc::unbounded();
    states.insert(BigBrother { _position: position, client_to_server_sink, recorder: Recorder { join } });
    tokio::spawn(async move {
        let (broadcast, _) = broadcast::channel(BROADCAST_CAPACITY);
        let mut history: Vec<Complex<stoc::Message>> = Vec::new();
        loop { tokio::select! {
            message = server_to_client_stream.next() => {
                let Some(message) = message else { return };
                broadcast.send(message.clone()).ok();
                history.push(message);
            }
            watcher = joining.next() => {
                let Some(watcher) = watcher else { return };
                let mut receiver = broadcast.subscribe();
                for message in &history {
                    watcher.unbounded_send(message.clone()).ok();
                }
                tokio::spawn(async move {
                    while let Ok(message) = receiver.recv().await {
                        watcher.unbounded_send(message).ok();
                    }
                });
            }
        }}
    });
}

#[before(srvpro::PlayerJoin)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn join_remote_watcher(player_join: &mut srvpro::PlayerJoin, big_brother: &mut BigBrother, stage: &mut Stage, _: RemoteProvider, stop: &mut StopFlag) -> Option<&'static str> {
    if stage.stage == DuelStage::Begin { return None }
    let mut player = player_join.player.take()?;
    let position_sender = player_join.position_sender.take()?;
    big_brother.recorder.join.unbounded_send(player.server_to_client_sink_towards_network.clone()).ok()?;
    if let Some(mut client_to_server_stream) = player.client_to_server_stream_towards_provider.take() {
        let client_to_server_sink = big_brother.client_to_server_sink.clone();
        tokio::spawn(async move {
            while let Some(message) = client_to_server_stream.next().await {
                if matches!(&*message, ctos::Message::Chat(_)) {
                    client_to_server_sink.unbounded_send(message).ok();
                }
            }
        });
    }
    position_sender.send(usize::MAX).ok();
    stop.0 = true;
    Some("cancel")
}
