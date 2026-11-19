use chrono::DateTime;
use chrono::Local;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;

pub static NAME: &'static str = module_path!();

#[derive(Attachment)]
pub struct StartTime {
    pub start: Option<DateTime<Local>>,
}

#[handler(stoc::DuelStart)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_start(start_time: &mut StartTime) {
    start_time.start.get_or_insert_with(Local::now);
}
