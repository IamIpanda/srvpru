//! Record whether a player is the host and its
//! [`Netplayer`](ygopro_data::constants::Netplayer) from the type change message.

use ygopro_data::message::stoc;

use crate::room::Player;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;

/// Name for activitating this module in the plugin system.
pub static NAME: &'static str = module_path!();

#[handler(stoc::TypeChange)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_type_change(player: &mut Player, type_change: &stoc::TypeChange) {
    player.states.insert(type_change.host);
    player.states.insert(type_change.player);
}
