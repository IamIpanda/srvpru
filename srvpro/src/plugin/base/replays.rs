//! Record the replay messages sent to the host.

use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::plugin::base::position::Host;
use crate::plugin::register_dependencies;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;

/// Name for activitating this module in the plugin system.
pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::position::NAME
);

#[derive(Attachment)]
pub struct Replays {
    pub replays: Vec<stoc::Replay>,
}

#[handler(stoc::Replay)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_replay(_: Host, replays: &mut Replays, replay: &stoc::Replay) {
    replays.replays.push(replay.clone());
}
