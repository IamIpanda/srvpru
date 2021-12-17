// ============================================================
// windbot
// ------------------------------------------------------------
//! Offer simple ai to play with.
//! 
//! **Attention** \
//! Different from srvpro, `windbot` plugin make windbot directly
//! join to inner ygopro server, don't pass through srvpru
//! so that any other plugin won't influence windbot.
// ============================================================

use std::sync::Arc;

use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use rand::prelude::SliceRandom;
use tokio::process::Command;

use crate::ygopro::Colors;
use crate::ygopro::message::ctos::JoinGame;

use crate::srvpru::Room;
use crate::srvpru::Handler;
use crate::srvpru::generate_chat;
use crate::srvpru::CommonError;
use crate::srvpru::message::ServerStart;
use crate::srvpru::plugins::base::chat_command;
use crate::srvpru::plugins::version_checker;


set_configuration! {
    #[serde(default)]
    spawn: Option<String>,
    server: String,
    bots: Vec<Bot>
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Bot {
    name: String,
    deck: String,
    dialog: String,
    #[serde(default)]
    hidden: bool
}

use_http_client!();

pub fn init() -> anyhow::Result<()> {
    load_configuration()?; 
    register_handlers();
    Ok(())
}

fn register_handlers() {
    Handler::before_message::<ServerStart, _>(100, "windbot_spawner", |_, _| Box::pin(async move {
        spawn()?;
        Ok(false)
    })).register_for_plugin("windbot");

    chat_command::before_message("ai", |context, message| Box::pin(async move {
        let name = (&message[3..]).trim().to_string();
        if let Some(room) = context.get_room() {
            if let Some(message) = join_room(room, Some(name)).await {
                context.send_back(&generate_chat(message, Colors::Red, context.get_region())).await.ok();
            }
        }
    })).register_for_plugin("windbot");

    Handler::follow_message::<JoinGame, _>(100, "windbot_ai_joiner", |context, message| Box::pin(async move {
        let name = context.get_string(&message.pass, "pass")?.clone();
        let room = context.get_room().ok_or(CommonError::RoomNotExist)?;
        if name.starts_with("AI#") {
            if let Some(message) = join_room(&room, None).await {
                context.send_back(&generate_chat(message, Colors::Red, context.get_region())).await.ok();
            }
        }
        Ok(false)
    })).register_for_plugin("windbot");
}

async fn join_room(room: &Arc<Mutex<Room>>, bot_name: Option<String>) -> Option<&'static str> {
    let bot = match select_a_bot(bot_name) {
        Some(bot) => bot,
        None => return Some("{windbot_deck_not_found}")
    };
    let _room = room.lock();
    let room_port = _room.server_addr.unwrap().port();
    let base_url = &get_configuration().server;
    let server = crate::srvpru::get_configuration();
    let version = version_checker::get_configuration().version;
    let url = format!("{}?name={}&deck={}&host={}&port={}&version={}&password={}", base_url, bot.name, bot.deck, server.ygopro.address, room_port, version, _room.origin_name);
    match get_http_client().get(url).send().await {
        Ok(response) if response.status() == reqwest::StatusCode::OK => None,
        _ => Some("{add_windbot_failed}")
    }
}

static SPAWN_CHILD: OnceCell<tokio::process::Child> = OnceCell::new();

fn spawn() -> anyhow::Result<()> {
    let configuration = get_configuration();
    if let Some(spawn) = configuration.spawn.as_ref() {
        let child = Command::new(spawn.clone()).spawn()?;
        SPAWN_CHILD.set(child).map_err(|_| anyhow!("Windbot: spawn process already exist."))?;
    }
    Ok(())
}

fn select_a_bot(name: Option<String>) -> Option<Bot> {
    let bots = &get_configuration().bots;
    let bots: Vec<&Bot> = match name {
        Some(_name) if !_name.is_empty() => bots.iter().filter(|bot| bot.name == _name).collect(),
        _                                       => bots.iter().filter(|bot| !bot.hidden).collect()
    };
    bots.choose(&mut rand::thread_rng()).map(|bot| (*bot).clone())
}