use ygopro_data::message::gm;
use ygopro_derive::Attachment;
use ygopro_derive::register_to;

use crate::plugin::register_dependencies;
use crate::room::Player;
use crate::room::GM_HANDLERS;
use crate::room::GameMessageHandler;

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
fn on_duel_start(player: &mut Player, count: &mut Count) {
    if player.states.get::<bool>() == Some(&true) {
        count.count += 1;
    }
}
