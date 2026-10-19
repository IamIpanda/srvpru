//! Count the turns of the duel.

use ygopro_data::message::gm;
use ygopro_derive::Attachment;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::plugin::register_dependencies;
use crate::room::GM_HANDLERS;
use crate::room::GameMessageHandler;
use crate::room::Player;

/// Name for activitating this module in the plugin system.
pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::position::NAME
);

#[derive(Attachment)]
pub struct Turn {
    pub turn: u32,
}

#[handler(gm::NewTurn)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_new_turn(player: &mut Player, turn: &mut Turn) {
    if player.states.get::<bool>() != Some(&true) { return }
    turn.turn += 1;
}
