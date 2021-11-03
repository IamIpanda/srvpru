// ============================================================
// deplayed_replay
// ------------------------------------------------------------
//! Stop replay event after each game.  
//! Send all the replays when a duel is end instead.
// ============================================================

use std::io::Cursor;

use tokio::io::AsyncWriteExt;

use crate::ygopro::message;
use crate::ygopro::message::stoc;
use crate::ygopro::Colors;
use crate::ygopro::data::Replay;

use crate::srvpru::processor::Handler;
use crate::srvpru::generate_chat;

player_attach! {
    replays: Vec<Vec<u8>>
}

fn register_handlers() {
    srvpru_handler!(stoc::MessageType::Replay, get_player_attachment_sure, |context| {
        let room = context.get_room().ok_or(anyhow!("Cannot get toom"))?;
        if room.lock().host_info.mode != crate::ygopro::Mode::Match {
            return Ok(false);
        }
        attachment.replays.push(context.request_buffer.to_vec());
        let vec = context.request_buffer.to_vec();
        let mut cursor = Cursor::new(vec);
        cursor.set_position(3);
        if let Err(e) = Replay::from_reader(&mut cursor) {
            error!("{:?}", e)
        }
        context.block_message()
    }).register_as("replay_interceptor");

    srvpru_handler!(stoc::MessageType::DuelEnd, get_player_attachment_sure, |context| {
        for (index, data) in attachment.replays.iter().enumerate() {
            context.send(&generate_chat(&format!("{{replay_hint_part1}} {} {{replay_hint_part2}}", index + 1), Colors::Babyblue, context.get_region())).await?;
            if let Some(socket) = context.socket { socket.write_all(&data).await?; }
        };
    }).register_as("replay_sender");
    register_player_attachment_dropper();
    register_player_attachment_mover();
    Handler::register_handlers("delayed_replay", message::Direction::STOC, vec!("replay_interceptor", "replay_sender"));
}

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}