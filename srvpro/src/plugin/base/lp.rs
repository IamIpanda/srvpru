use ygopro_data::message::gm;
use ygopro_derive::Attachment;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::room::GM_HANDLERS;
use crate::room::GameMessageHandler;

pub static NAME: &'static str = module_path!();

#[derive(Attachment)]
pub struct Lp {
    pub lp: [i32; 2],
}

#[handler(gm::LPUpdate)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_lp_update(lp: &mut Lp, lp_update: &gm::LPUpdate) {
    if let Some(slot) = lp.lp.get_mut(lp_update.player as usize) {
        *slot = lp_update.lp;
    }
}
