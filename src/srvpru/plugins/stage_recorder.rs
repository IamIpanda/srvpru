// ============================================================
// stage_recorder
// ------------------------------------------------------------
//! Record which stage is duel in.
// ============================================================

use crate::srvpru::Context;
use crate::srvpru::Handler;
use crate::srvpru::structs;
use crate::ygopro::message::Direction;
use crate::ygopro::message::STOCMessageType;
use crate::ygopro::constants::GMMessageType;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum DuelStage {
    Void,
    Begin,
    Finger,
    Firstgo,
    Dueling,
    Siding,
    End
}

impl std::default::Default for DuelStage {
    fn default() -> DuelStage {
        return DuelStage::Begin;
    }
}
 
room_attach! {
    duel_stage: DuelStage 
}

pub fn init()  -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}

fn register_handlers() {
    srvpru_handler!(structs::RoomCreated,        get_room_attachment_sure, |context, _| Ok(false)).register_as("stage_recorder_1");
    srvpru_handler!(STOCMessageType::DuelStart,  get_room_attachment_sure, |context| { attachment.duel_stage = DuelStage::Finger;  }).register_as("stage_recorder_2");
    srvpru_handler!(STOCMessageType::SelectHand, get_room_attachment_sure, |context| { attachment.duel_stage = DuelStage::Finger;  }).register_as("stage_recorder_3");
    srvpru_handler!(STOCMessageType::SelectTp,   get_room_attachment_sure, |context| { attachment.duel_stage = DuelStage::Firstgo; }).register_as("stage_recorder_4");
    srvpru_handler!(STOCMessageType::ChangeSide, get_room_attachment_sure, |context| { attachment.duel_stage = DuelStage::Siding;  }).register_as("stage_recorder_5");
    srvpru_handler!(GMMessageType::Start,        get_room_attachment_sure, |context| { attachment.duel_stage = DuelStage::Dueling; }).register_as("stage_recorder_6");
    
    register_room_attachement_dropper();
    Handler::register_handlers("stage_recorder", Direction::SRVPRU, vec!["stage_recorder_1", "stage_recorder_room_attachment_dropper"]);
    Handler::register_handlers("stage_recorder", Direction::STOC, vec!["stage_recorder_2", "stage_recorder_3", "stage_recorder_4", "stage_recorder_5", "stage_recorder_6"]);
}

impl<'a> Context<'a> {
    pub fn get_stage(&self) -> DuelStage {
        let stage = get_room_attachment(self);
        if stage.is_none() { return DuelStage::Void; }
        else { return stage.unwrap().duel_stage; }
    }    
}