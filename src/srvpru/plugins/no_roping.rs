// ============================================================
// no_roping
// ------------------------------------------------------------
//! Limit player rope.
// ============================================================

use once_cell::sync::OnceCell;
use tokio::time::Duration;
use tokio::task::JoinHandle;
use chrono::offset::Local;
 
use crate::ygopro::Netplayer;
use crate::ygopro::Colors;
use crate::srvpru::room::ROOMS;
use crate::srvpru::Handler;
use crate::srvpru::generate_chat;
use crate::srvpru::HandlerCondition;
use crate::srvpru::HandlerOccasion;
use crate::ygopro::message::Direction;

set_configuration! {
    #[serde(default="default_max_roping_time")]
    max_roping_time: i64,
    #[serde(default="default_roping_warn_time")]
    roping_warn_time: i64,
    #[serde(default="default_scan_interval")]
    scan_interval: u64
}

fn default_max_roping_time() -> i64 { 90000 }
fn default_roping_warn_time() -> i64 { 70000 }
fn default_scan_interval() -> u64 { 4000 }

player_attach! {
    last_action_time: i64
}

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    start_scanner();
    Ok(())
}

fn register_handlers() {
    Handler::new(100, "roping_recorder", HandlerOccasion::After, HandlerCondition::Always, |context| Box::pin(async move {
        get_player_attachment_sure(context).last_action_time = Local::now().timestamp_millis();
        Ok(false)
    })).register();

    register_player_attachment_dropper();
    register_player_attachment_mover();
    Handler::register_handlers("no_roping", Direction::CTOS, vec!["roping_recorder"]);
}

static SCANNER: OnceCell<JoinHandle<()>> = OnceCell::new();

fn start_scanner() {
    let configuration = get_configuration();
    SCANNER.set(tokio::spawn(async move {
        loop {
            scan_rooms().await;
            tokio::time::sleep(Duration::from_millis(configuration.scan_interval)).await;
        }
    })).expect("scanner cell already set.");
}

async fn scan_rooms() {
    let now = Local::now().timestamp_millis();
    let configuration = get_configuration();
    let rooms = ROOMS.read();
    for room in rooms.values() {
        let _room = room.lock();
        for player in _room.players.iter() {
            let mut _player = player.lock();
            if let Some(attachment) = PLAYER_ATTACHMENTS.read().get(&_player.client_addr) {
                if _player.get_position() != Netplayer::Observer && !_player.timeout_exempt {
                    let spare_time = now - attachment.last_action_time;
                    if spare_time > configuration.max_roping_time {
                        _player.expel();
                    }
                    else if spare_time > configuration.roping_warn_time && spare_time <= configuration.roping_warn_time + configuration.scan_interval as i64 {
                        let rest_time = (configuration.max_roping_time - spare_time) / 1000;
                        let message = generate_chat(&format!("{}{{afk_warn_part1}}{}{{afk_warn_part2}}", _player.name, rest_time), Colors::Red, _player.region);
                        _player.send_to_client(&message).await.ok();
                    }
                }
            }
        }
    }
}