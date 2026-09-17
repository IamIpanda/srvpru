use ygopro_data::constants::DuelStage;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;

pub static NAME: &'static str = module_path!();

#[derive(Attachment)]
pub struct Stage {
    #[attachment(default = "DuelStage::Begin")]
    pub stage: DuelStage,
}

#[handler(stoc::SelectHand)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_select_hand(stage: &mut Stage) {
    stage.stage = DuelStage::Finger;
}

#[handler(stoc::SelectTp)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_select_tp(stage: &mut Stage) {
    stage.stage = DuelStage::Firstgo;
}

#[handler(stoc::DuelStart)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_start(stage: &mut Stage) {
    stage.stage = DuelStage::Dueling;
}

#[handler(stoc::ChangeSide)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_change_side(stage: &mut Stage) {
    stage.stage = DuelStage::Siding;
}

#[handler(stoc::DuelEnd)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_end(stage: &mut Stage) {
    stage.stage = DuelStage::End;
}
