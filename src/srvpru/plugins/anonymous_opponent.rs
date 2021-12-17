// ============================================================
// anonymous_opponent
// ------------------------------------------------------------
//! Player can't see opponent's name.
//! 
//! Dependency:
//! - [position_recorder](super::recorder::position_recorder)
// ============================================================

use std::collections::HashMap;

use crate::srvpru::CommonError;
use crate::ygopro::Netplayer;
use crate::ygopro::message::ctos::HsToDuelist;
use crate::ygopro::message::ctos::PlayerInfo;
use crate::ygopro::message::stoc::DuelStart;
use crate::ygopro::message::stoc::HsPlayerEnter;
use crate::ygopro::message::string::cast_to_fix_length_array;

use crate::srvpru::Handler;

player_attach! {}

room_attach! {
    actual_names: HashMap<Netplayer, String>
}

depend_on! {
    "position_recorder"
}

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    register_dependency()?;
    Ok(())
}

fn register_handlers() {
    // Anonymous opponent work as what newspeak do on Silent mode.
    // Srvpru record the marked name, ygopro server get the correct name.
    // In HsPlayerEnter, which still has correct name, record the name and block it.
    // When Duelstart, send HsPlayerEnter to mark all players normal.
    Handler::before_message::<PlayerInfo, _>(10, "anonymous_opponent_marker", |context, message| Box::pin(async move {
        let name = context.get_string(&message.name, "name")?;
        *name = "******".to_string();
        Ok(false)
    })).register_for_plugin("anonymous_opponent");

    Handler::before_message::<HsToDuelist, _>(101, "anonymous_opponent_to_duelist", |context, _| Box::pin(async move {
        insert_player_attachment(context);
        Ok(false)
    })).register_for_plugin("anonymous_opponent");

    Handler::before_message::<HsPlayerEnter, _>(101, "anonymous_opponent_revealer", |context, message| Box::pin(async move {
        let position = context.get_position();
        let mut attachment = get_room_attachment_sure(context)?;
        let exist_player_attachment = get_player_attachment(context).is_some();
        let name = context.get_string(&message.name, "name")?;
        // User it self, no mask.
        if message.pos == position { return Ok(false) }
        // move from observer to duelist, in this message, HsPlayerEnter is before TypeChange. so must manually process it.
        if position == Netplayer::Observer && exist_player_attachment {
            let name = name.clone();
            PLAYER_ATTACHMENTS.write().remove(&context.addr);
            attachment.actual_names.insert(message.pos, name);
            return Ok(false);
        }
        // mask user.
        attachment.actual_names.entry(message.pos).or_insert_with(|| name.clone());
        *name = "******".to_string();
        message.name = cast_to_fix_length_array("******");
        context.reserialize = true;
        Ok(false)
    })).register_for_plugin("anonymous_opponent");

    // It seems ygopro will do some order magics after duel start.
    // So just send it before duel start confirm.
    Handler::before_message::<DuelStart, _>(200, "anonymous_opponent_recover", |context, _| Box::pin(async move {
        let attachment = get_room_attachment_sure(context)?;
        let players = context.get_room().ok_or(CommonError::RoomNotExist)?.lock().get_players_in_hashmap();
        for (&pos, name) in attachment.actual_names.iter() {
            if let Some(player) = players.get(&pos) {
                player.lock().name = name.clone();
            }
            context.send(&HsPlayerEnter {
                name: cast_to_fix_length_array(name),
                pos
            }).await.ok();
        }
        Ok(false)
    })).register_for_plugin("anonymous_opponent");

    register_room_attachement_dropper();
}