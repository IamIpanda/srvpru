//! Hold the replay until the duel ends.

use ygopro_derive::*;
use ygopro_data::constants::Color;
use ygopro_data::message::stoc;

use crate::plugin::base::replays::Replays;
use crate::plugin::register_dependencies;
use crate::room::Room;
use crate::room::ServerToClientHandler;
use crate::room::STOC_HANDLERS;

/// Name for activitating this module in the plugin system.
#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::replays::NAME
);

#[after(stoc::Replay)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_replay() -> &'static str {
    "cancel"
}

#[before(stoc::DuelEnd)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_end(room: &mut Room, replays: &mut Replays) {
    for (index, replay) in replays.replays.drain(..).enumerate() {
        room.broadcast_message(Color::System, &format!("正在发送第 {} 局录像", index + 1));
        room.broadcast(stoc::Message::Replay(Box::new(replay)));
    }
}
