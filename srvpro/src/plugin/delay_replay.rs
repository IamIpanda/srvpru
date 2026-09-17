use linkme::distributed_slice;

use ygopro_derive::*;
use ygopro_data::message::stoc;

use crate::plugin::base::replays::Replays;
use crate::room::Room;
use crate::room::ServerToClientHandler;
use crate::room::STOC_HANDLERS;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[after(stoc::Replay)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_replay() -> &'static str {
    "cancel"
}

#[handler(stoc::DuelEnd)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_end(room: &mut Room, replays: &mut Replays) {
    for replay in replays.replays.drain(..) {
        room.broadcast(stoc::Message::Replay(replay));
    }
}
