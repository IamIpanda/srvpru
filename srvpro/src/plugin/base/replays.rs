use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::room::Player;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;

pub static NAME: &'static str = module_path!();

#[derive(Attachment)]
pub struct Replays {
    pub replays: Vec<stoc::Replay>,
}

#[handler(stoc::Replay)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_replay(player: &mut Player, replays: &mut Replays, replay: &stoc::Replay) {
    if player.states.get::<bool>() != Some(&true) { return }
    replays.replays.push(replay.clone());
}
