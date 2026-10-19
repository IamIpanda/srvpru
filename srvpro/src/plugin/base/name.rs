//! Remember the name sent in [`ctos::PlayerInfo`].

use ygopro_data::message::ctos;

use crate::room::Player;
use crate::room::CTOS_HANDLERS;
use crate::room::ClientToServerHandler;

/// Name for activitating this module in the plugin system.
pub static NAME: &'static str = module_path!();

#[handler(ctos::PlayerInfo)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn on_player_info(player: &mut Player, playe_info: &ctos::PlayerInfo) {
    player.states.insert(playe_info.name.to_string());
}
