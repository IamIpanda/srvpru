// ============================================================
// deplayed_replay
// ------------------------------------------------------------
//! Stop replay event after each game.  
//! Send all the replays when a duel is end instead.
// ============================================================

use tokio::io::AsyncWriteExt;

use crate::ygopro::message;
use crate::ygopro::message::STOCMessageType;
use crate::srvpru::processor::Handler;

player_attach! {
    replays: Vec<Vec<u8>>
}

fn register_handlers() {
    srvpru_handler!(STOCMessageType::Replay, get_player_attachment_sure, |context| {
        if let Some(room) = context.get_room() {
            let room = room.lock();
            if room.host_info.mode != crate::ygopro::constants::Mode::Match {
                return Ok(false);
            }
        }
        attachment.replays.push(context.request_buffer.to_vec());
        context.block_message()
    }).register_as("replay_interceptor");

    srvpru_handler!(STOCMessageType::DuelEnd, get_player_attachment_sure, |context| {
        for (index, data) in attachment.replays.iter().enumerate() {
            context.send_chat(&format!("{{replay_hint_part1}} {} {{replay_hint_part2}}", index + 1), crate::ygopro::constants::Colors::Babyblue).await?;
            if let Some(socket) = context.socket { socket.write_all(&data).await?; }
        };
    }).register_as("replay_sender");

    register_player_attachment_dropper();
    Handler::register_handlers("delayed_replay", message::Direction::STOC, vec!("replay_interceptor", "replay_sender"));
    Handler::register_handlers("delayed_replay", message::Direction::SRVPRU, vec!("delayed_replay_player_attachment_dropper"));
}

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}