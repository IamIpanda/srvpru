//! Count how many duels are played.

use ygopro_data::message::gm;
use ygopro_derive::Attachment;
use ygopro_derive::register_to;

use crate::plugin::base::position::Host;
use crate::plugin::register_dependencies;
use crate::room::GM_HANDLERS;
use crate::room::GameMessageHandler;

/// Name for activitating this module in the plugin system.
pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::position::NAME
);

#[derive(Attachment)]
pub struct Count {
    pub count: u8,
}

#[before(gm::Win)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_duel_start(_: Host, count: &mut Count) {
    count.count += 1;
}
