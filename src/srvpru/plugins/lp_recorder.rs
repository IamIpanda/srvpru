// ============================================================
// lp_recorder
// ------------------------------------------------------------
//! Record each player lp.
//! 
//! Dependency:
//! - [position_recorder](super::position_recorder)
// ============================================================

use crate::srvpru;
use crate::srvpru::Handler;
use crate::srvpru::message::LpChange;
use crate::ygopro::message::Direction;
use crate::ygopro::message::stoc;
use crate::ygopro::message::gm;

player_attach! {
    lp: i32
}

export_player_attach_as!(get_lp, i32, transformer);

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())    
}

fn register_handlers() {
    Handler::follow_message::<stoc::DuelStart, _>(100, "lp_recorder_start", |context, _| Box::pin(async move {
        let room = context.get_room().ok_or(anyhow!("Can't get the room"))?;
        get_player_attachment_sure(context).lp = room.lock().host_info.start_lp as i32;
        Ok(false)
    })).register();

    srvpru_handler!(gm::Damage, get_player_attachment_sure, |context, request| {
        if context.get_position() != request.player { return Ok(false); }
        attachment.lp = attachment.lp - request.value;
        srvpru::server::trigger_internal(*context.addr, LpChange { player: context.get_player().ok_or(anyhow!("Can't get player"))?, lp: attachment.lp});
    }).register_as("lp_recorder_damage");

    srvpru_handler!(gm::Recover, get_player_attachment_sure, |context, request| {
        if context.get_position() != request.player { return Ok(false); }
        attachment.lp = attachment.lp + request.value;
    }).register_as("lp_recorder_recover");

    srvpru_handler!(gm::Lpupdate, get_player_attachment_sure, |context, request| {
        if context.get_position() != request.player { return Ok(false); }
        attachment.lp = request.lp;
    }).register_as("lp_recorder_lpupdate");

    srvpru_handler!(gm::PayLpcost, get_player_attachment_sure, |context, request| {
        if context.get_position() != request.player { return Ok(false); }
        attachment.lp = attachment.lp - request.cost;
    }).register_as("lp_recorder_paylp");

    register_player_attachment_dropper();
    register_player_attachment_mover();
    Handler::register_handlers("lp_recorder", Direction::STOC, vec!["lp_recorder_damage", "lp_recorder_recover", "lp_recorder_lpupdate", "lp_recorder_paylp", "lp_recorder_start"]);
}

fn transformer<'b> (attachment: Option<parking_lot::MappedRwLockWriteGuard<'b, PlayerAttachment>>) -> i32 {
    attachment.map(|attach| attach.lp).unwrap_or(8000)
}
