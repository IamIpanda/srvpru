// ============================================================
// must_start
// ------------------------------------------------------------
//! Force Room host start. Force change side finish in time.
//! 
//! Dependency:
//! - [position_recorder](super::position_recorder)
// ============================================================

use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio::time::Duration;

use crate::srvpru::Handler;
use crate::srvpru::generate_chat;

use crate::ygopro::Colors;
use crate::ygopro::message::Direction;
use crate::ygopro::message::ctos;
use crate::ygopro::message::stoc;

set_configuration! {
    start_game: Vec<u64>,
    change_side: u64
}

room_attach! {
    start_game_watcher: Option<JoinHandle<()>>
}

player_attach! {
    ready: bool,
    change_side_watcher: Option<JoinHandle<()>>
}

depend_on! {
    "position_recorder"
}

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_dependency()?;
    register_handlers();

    Ok(())
}

fn register_handlers() {
    Handler::follow_message(100, "must_start_game", |context, _: &ctos::HsReady| Box::pin(async move {
        get_player_attachment_sure(context).ready = true;
        let mut room_attachment = get_room_attachment_sure(context);
        let room = context.get_room().ok_or(anyhow!("Cannot get room"))?;
        // room is full check
        {
            let _room = room.lock();
            if !(_room.is_full() && _room.is_all_ready()) { return Ok(false) }
        }
        if room_attachment.start_game_watcher.is_some() { return Ok(false) }
        // get the host
        let host = room.lock().get_host().ok_or(anyhow!("Can't decide room host"))?;
        let region = host.lock().region.clone();
        room_attachment.start_game_watcher = Some(tokio::spawn(async move {
            let configuration = get_configuration();
            let start_game = &configuration.start_game;
            let mut position = 0usize;
            while position <= start_game.len() {
                if position == start_game.len() - 1 {
                    let rest_time = start_game[position];
                    room.lock().send(&generate_chat(&format!("{}{{kick_count_down}}", rest_time), Colors::Red, region)).await; 
                    sleep(Duration::from_secs(start_game[position])).await;
                    break;
                }
                else {
                    let rest_time = start_game[position];
                    room.lock().send(&generate_chat(&format!("{}{{kick_count_down}}", rest_time), Colors::Babyblue, region)).await;
                    sleep(Duration::from_secs(rest_time - start_game[position + 1])).await;
                    position = position + 1;
                }
            }
            let host_name = host.lock().name.clone();
            host.lock().expel();
            let _room = room.lock();
            _room.send(&generate_chat(&format!("{:} {{kicked_by_system}}", host_name), Colors::Red, region)).await;
            ROOM_ATTACHMENTS.write().get_mut(&_room.name).map(|attachment| attachment.start_game_watcher = None);
        }));  
        Ok(false)
    })).register();

    Handler::follow_message(100, "must_start_game_not_ready", |context, _: &ctos::HsNotReady| Box::pin(async move {
        get_player_attachment_sure(context).ready = false;
        let mut room_attachment = get_room_attachment_sure(context);
        if let Some(handle) = room_attachment.start_game_watcher.take() {
            handle.abort();
        }
        Ok(false)
    })).register();

    Handler::follow_message(254, "must_start_game_started", |context, _: &ctos::HsStart| Box::pin(async move {
        let mut room_attachment = get_room_attachment_sure(context);
        if let Some(handler) = room_attachment.start_game_watcher.take() {
            handler.abort();
        }
        Ok(false)
    })).register();

    Handler::follow_message(100, "must_change_side", |context, _: &stoc::ChangeSide| Box::pin(async move {
        let mut attachment = get_player_attachment_sure(context);
        let player = context.get_player().ok_or(anyhow!("Cannot get player"))?;
        let configuration = get_configuration();
        context.send(&generate_chat(&format!("{{side_timeout_part1}}{}{{side_timeout_part2}}", configuration.change_side), Colors::Babyblue, context.get_region())).await.ok();
        attachment.change_side_watcher = Some(tokio::spawn(async move {
            sleep(Duration::from_secs(configuration.change_side * 60)).await;
            let mut _player = player.lock();
            let region = _player.region.clone();
            _player.send_to_client(&generate_chat("{{side_overtime}}", Colors::Red, region)).await.ok();
            _player.expel();
        }));
        Ok(false)
    })).register();

    Handler::follow_message(100, "must_change_side_finished", |context, _: &stoc::DuelStart| Box::pin(async move {
        let mut attachment = get_player_attachment_sure(context);
        if let Some(handler) = attachment.change_side_watcher.as_mut() {
            handler.abort();
        }
        Ok(false)
    })).register();

    register_room_attachement_dropper();
    register_player_attachment_dropper();
    register_player_attachment_mover();
    Handler::register_handlers("must_start", Direction::CTOS, vec!["must_start_game", "must_start_game_not_ready", "must_start_game_started"]);
    Handler::register_handlers("must_start", Direction::STOC, vec!["must_change_side_finished", "must_change_side"]);
}

impl crate::srvpru::Room {
    pub fn is_all_ready(&self) -> bool {
        self.players.iter().all(|player| {
            let _player = player.lock(); 
            _player.is_host() || PLAYER_ATTACHMENTS.read().get(&_player.client_addr).map(|attachment| attachment.ready) == Some(true)
        })
    }
}