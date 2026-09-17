use ygopro_data::message::stoc;

use crate::room::Player;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;

pub static NAME: &'static str = module_path!();

#[handler(stoc::TypeChange)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_type_change(player: &mut Player, type_change: &stoc::TypeChange) {
    player.states.insert(type_change.host);
    player.states.insert(type_change.player);
}
