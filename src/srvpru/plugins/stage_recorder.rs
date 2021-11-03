// ============================================================
// stage_recorder
// ------------------------------------------------------------
//! Record which stage is duel in.
// ============================================================

use crate::srvpru::Handler;
use crate::ygopro::message::Direction;
use crate::ygopro::message::stoc;
use crate::ygopro::message::srvpru;
use crate::ygopro::message::gm;

pub fn init()  -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}

room_attach! {
    duel_stage: DuelStage 
}
export_room_attach_as!(get_duel_stage, DuelStage, transformer);
export_room_attach_in_join_game_as!(get_duel_stage_in_join_game, DuelStage, transformer);

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

fn register_handlers() {
    srvpru_handler!(srvpru::RoomCreated,           get_room_attachment_sure, |context, _| Ok(false)).register_as("stage_recorder_1");
    srvpru_handler!(stoc::MessageType::DuelStart,  get_room_attachment_sure, |context| { attachment.duel_stage = DuelStage::Finger;  }).register_as("stage_recorder_2");
    srvpru_handler!(stoc::MessageType::SelectHand, get_room_attachment_sure, |context| { attachment.duel_stage = DuelStage::Finger;  }).register_as("stage_recorder_3");
    srvpru_handler!(stoc::MessageType::SelectTp,   get_room_attachment_sure, |context| { attachment.duel_stage = DuelStage::Firstgo; }).register_as("stage_recorder_4");
    srvpru_handler!(stoc::MessageType::ChangeSide, get_room_attachment_sure, |context| { attachment.duel_stage = DuelStage::Siding;  }).register_as("stage_recorder_5");
    srvpru_handler!(gm::MessageType::Start,        get_room_attachment_sure, |context| { attachment.duel_stage = DuelStage::Dueling; }).register_as("stage_recorder_6");
    
    register_room_attachement_dropper();
    Handler::register_handlers("stage_recorder", Direction::SRVPRU, vec!["stage_recorder_1"]);
    Handler::register_handlers("stage_recorder", Direction::STOC,   vec!["stage_recorder_2", "stage_recorder_3", "stage_recorder_4", "stage_recorder_5", "stage_recorder_6"]);
}

impl std::default::Default for DuelStage {
    fn default() -> DuelStage {
        return DuelStage::Begin;
    }
}

fn transformer<'b> (attachment: Option<parking_lot::MappedRwLockWriteGuard<'b, RoomAttachment>>) -> DuelStage {
    attachment.map(|attach| attach.duel_stage).unwrap_or(DuelStage::Void)
}
