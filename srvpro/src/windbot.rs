use std::process::Stdio;
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;

use hashbrown::HashMap;
use parking_lot::Mutex;
use parking_lot::RwLock;
use rand::Rng;
use rand::seq::SliceRandom;
use serde::Deserialize;
use tokio::process::Command;
use tokio::task::JoinHandle;
use ygopro_data::constants::Color;
use ygopro_data::constants::Rule;
use ygopro_data::message::ctos;
use ygopro_derive::Configuration;

use crate::GlobalHandler;
use crate::message as srvpro;
use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::Normal;
use crate::mode::RoomConfiguration;
use crate::plugin::chat_command::CHAT_COMMANDS;
use crate::plugin::chat_command::ChatCommandHandler;
use crate::room::*;

pub static NAME: &'static str = module_path!();
static SPAWNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static PROCESS: LazyLock<Mutex<Option<JoinHandle<()>>>> = LazyLock::new(|| Mutex::new(None));
static PENDING_BOTS: LazyLock<RwLock<HashMap<String, Bot>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    /// Host of the WindBot ServerMode HTTP endpoint.
    #[config(default = "String::from(\"127.0.0.1\")")]
    pub server_ip: String,
    /// Port of the WindBot ServerMode HTTP endpoint.
    /// It must match the `ServerPort` the windbot process listens on.
    #[config(default = "2399")]
    pub port: u16,
    /// Address WindBot uses to connect back to this server.
    /// Must be reachable from the machine running WindBot.
    #[config(default = "String::from(\"127.0.0.1\")")]
    pub my_ip: String,
    /// Port WindBot uses to connect back.
    /// `0` falls back to the main server port.
    #[config(default = "0")]
    pub my_port: u16,
    /// Script that starts the WindBot service.
    /// An empty value leaves the process to the operator.
    #[config(default = "String::new()")]
    pub spawn: String,
    /// Bots WindBot may play, as a JSON array of `name`, `deck`, `dialog` and `hidden` objects.
    /// An empty list uses a single default bot with a random deck.
    #[config(default = "BotList::default()")]
    pub bots: BotList,
}

impl Configuration {
    fn game_port(&self) -> u16 {
        if self.my_port != 0 { return self.my_port }
        crate::configuration::get_configuration::<crate::server::Configuration>().map(|configuration| configuration.port).unwrap_or(7911)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Bot {
    pub name: String,
    #[serde(default)]
    pub deck: String,
    #[serde(default)]
    pub dialog: String,
    #[serde(default)]
    pub hidden: bool,
}

impl Default for Bot {
    fn default() -> Self {
        Self { name: "AI".to_string(), deck: String::new(), dialog: String::new(), hidden: false }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BotList(Vec<Bot>);

impl FromStr for BotList {
    type Err = serde_json::Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(serde_json::from_str(value)?))
    }
}

impl BotList {
    pub fn choose(&self, query: &str) -> Option<Bot> {
        let query = query.trim();
        if self.0.is_empty() { return Some(Bot::default()) }
        let mut candidates: Vec<&Bot> = self.0.iter().filter(|bot| {
            if query.is_empty() { !bot.hidden } else { bot.name == query || bot.deck == query }
        }).collect();
        candidates.shuffle(&mut rand::thread_rng());
        candidates.first().map(|bot| (*bot).clone())
    }
}

pub async fn invite(configuration: &Configuration, room_name: &str, bot: &Bot) -> Result<(), String> {
    let query = serde_urlencoded::to_string(&[
        ("name", bot.name.clone()),
        ("deck", bot.deck.clone()),
        ("host", configuration.my_ip.clone()),
        ("port", configuration.game_port().to_string()),
        ("dialog", bot.dialog.clone()),
        ("version", (*ygopro::PRO_VERSION).to_string()),
        ("password", room_name.to_string()),
    ]).unwrap_or_default();
    let url = format!("http://{}:{}/?{}", configuration.server_ip, configuration.port, query);
    tokio::task::spawn_blocking(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).build().into();
        agent.get(&url).call().map(|_| ()).map_err(|error| error.to_string())
    }).await.map_err(|error| error.to_string())?
}

#[handler(srvpro::PluginEnabled)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn on_plugin_enabled(event: &srvpro::PluginEnabled) {
    if event.module_name != NAME { return }
    let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return };
    if configuration.spawn.is_empty() { return }
    if SPAWNED.swap(true, std::sync::atomic::Ordering::SeqCst) { return }
    let handle = tokio::spawn(async move { spawn_windbot(configuration).await });
    *PROCESS.lock() = Some(handle);
}

#[handler(srvpro::PluginDisabled)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn on_plugin_disabled(event: &srvpro::PluginDisabled) {
    if event.module_name != NAME { return }
    SPAWNED.store(false, std::sync::atomic::Ordering::SeqCst);
    if let Some(handle) = PROCESS.lock().take() { handle.abort() }
}

async fn spawn_windbot(configuration: Configuration) {
    let mut command = Command::new(&configuration.spawn);
    command
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    match command.spawn() {
        Ok(mut child) => {
            log::info!("windbot process started");
            match child.wait().await {
                Ok(status) => log::warn!("windbot process exited: {status}"),
                Err(error) => log::warn!("windbot process wait failed: {error}"),
            }
        }
        Err(error) => log::warn!("windbot process failed to start: {error}"),
    }
}

struct AiRoom {
    bot: Option<Bot>,
}

struct AiSummoned;

#[before(ctos::JoinGame)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
async fn before_join_game(player: &mut Player, join_game: &ctos::JoinGame) -> &'static str {
    if player.states.get::<srvpro::PlayerMove>().is_some() { return "continue" }
    let pass = join_game.pass.trim();
    if !pass.as_bytes().get(..2).is_some_and(|head| head.eq_ignore_ascii_case(b"AI")) { return "continue" }
    if ROOMS.read().contains_key(pass) { return "continue" }
    let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return "continue" };
    let query = pass.split_once('#').map(|(_, query)| query).unwrap_or_default();
    let Some(bot) = configuration.bots.choose(query) else {
        player.send_message("找不到指定的 windbot。", Color::Red);
        return "kick"
    };
    let room_name = format!("AI#{}", rand::thread_rng().gen_range(0..100000));
    PENDING_BOTS.write().insert(room_name.clone(), bot);
    RoomHost::new(&room_name).await;
    player.states.insert(srvpro::PlayerMove { room_name });
    "continue"
}

#[handler(Normal)]
#[register_to(MODES)]
fn ai_room(config: &mut RoomConfiguration, part: &str) -> bool {
    if !part.eq_ignore_ascii_case("AI") { return false }
    let bot = PENDING_BOTS.write().remove(&config.name);
    config.hostinfo.rule = Rule::All;
    config.hostinfo.lflist = u32::MAX;
    config.hostinfo.time_limit = 999;
    config.states.insert(AiRoom { bot });
    true
}

#[after(ctos::JoinGame)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn on_join_game(room: &mut Room, states: &mut Anymap) {
    if states.get::<AiSummoned>().is_some() { return }
    let Some(ai_room) = states.get::<AiRoom>() else { return };
    let bot = ai_room.bot.clone();
    states.insert(AiSummoned);
    let Some(bot) = bot else {
        if let Some((_, player)) = room.players.iter().next() {
            player.send_message("没有可用的 windbot 配置。", Color::Red);
        }
        return
    };
    let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return };
    let room_name = room.name.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        if let Err(error) = invite(&configuration, &room_name, &bot).await {
            log::warn!("Failed to invite windbot to room {room_name}: {error}");
        }
    });
}

#[command]
#[register_to(CHAT_COMMANDS as ChatCommandHandler with &'static str)]
fn ai(room: &mut Room, _states: &mut Anymap, index: usize, args: &str) {
    let room_name = room.name.clone();
    let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return };
    let Some(player) = room.players.get_mut(index) else { return };
    if !player.states.get::<bool>().copied().unwrap_or(false) {
        player.send_message("只有房主可以召唤 AI。", Color::Red);
        return;
    }
    let Some(bot) = configuration.bots.choose(args) else {
        player.send_message("找不到指定的 AI。", Color::Red);
        return;
    };
    player.send_message(&format!("正在邀请 AI: {}", bot.name), Color::Babyblue);
    tokio::spawn(async move {
        if let Err(error) = invite(&configuration, &room_name, &bot).await {
            log::warn!("Failed to invite windbot to room {room_name}: {error}");
        }
    });
}
