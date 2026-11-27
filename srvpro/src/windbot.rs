use std::str::FromStr;
use std::sync::Arc;
use std::sync::LazyLock;

use arc_swap::ArcSwap;
use rand::Rng;
use rand::seq::SliceRandom;
use serde::Deserialize;
use ygopro::duel::SendTarget;
use ygopro_data::constants::Color;
use ygopro_data::constants::Netplayer;
use ygopro_data::constants::Rule;
use ygopro_data::message::ctos;

use crate::GlobalHandler;
use crate::message as srvpro;
use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::Normal;
use crate::mode::RoomConfiguration;
use crate::mode::slice_with_prefix;
use crate::plugin::chat_command::CHAT_COMMANDS;
use crate::plugin::chat_command::ChatCommandHandler;
use crate::room::*;

/// Name for activitating this module in the plugin system.
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    /// Name of the linked [`BotProvider`] that brings bots in.
    pub provider: String,
}

/// One way of bringing a bot into a room.
pub trait BotProvider: Send + Sync {
    /// Bots this host may play.
    fn bots(&self) -> BotList;
    /// Asks the host to make `bot` join `room`.
    fn invite(&self, room: &Room, bot: &Bot);
}

/// Hosts linked into this binary, selected by [`Configuration::provider`].
#[distributed_slice]
pub static BOT_PROVIDERS: [(&'static str, fn() -> Box<dyn BotProvider>)];

static BOT_PROVIDER: LazyLock<ArcSwap<Option<Box<dyn BotProvider>>>> = LazyLock::new(|| {
    let provider = crate::configuration::get_configuration::<Configuration>().map(|configuration| configuration.provider);
    ArcSwap::from_pointee(BOT_PROVIDERS.iter().find(|(name, _)| provider.as_deref() == Some(*name)).map(|(_, make)| make()))
});

#[after(srvpro::ConfigurationChanged)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn on_configuration_changed() {
    let provider = crate::configuration::get_configuration::<Configuration>().map(|configuration| configuration.provider);
    BOT_PROVIDER.store(Arc::new(BOT_PROVIDERS.iter().find(|(name, _)| provider.as_deref() == Some(*name)).map(|(_, make)| make())));
}

fn bots() -> BotList {
    let selected = BOT_PROVIDER.load_full();
    selected.as_ref().as_ref().map(|host| host.bots()).unwrap_or_default()
}

fn invite(room: &Room, bot: &Bot) {
    let selected = BOT_PROVIDER.load_full();
    let Some(host) = selected.as_ref().as_ref() else { return };
    host.invite(room, bot);
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

#[derive(Clone, Debug, Default)]
pub struct BotList(Arc<Vec<Bot>>);

impl FromStr for BotList {
    type Err = serde_json::Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(Arc::new(serde_json::from_str(value)?)))
    }
}

impl From<Vec<Bot>> for BotList {
    fn from(bots: Vec<Bot>) -> Self { Self(Arc::new(bots)) }
}

impl BotList {
    pub fn is_empty(&self) -> bool { self.0.is_empty() }

    pub fn choose(&self, query: &str) -> Option<&Bot> {
        let query = query.trim();
        if self.0.is_empty() { return None }
        let mut candidates: Vec<&Bot> = self.0.iter().filter(|bot| {
            if query.is_empty() { 
                !bot.hidden 
            } else { 
                bot.name.eq_ignore_ascii_case(query) || bot.deck.eq_ignore_ascii_case(query) 
            }
        }).collect();
        candidates.shuffle(&mut rand::thread_rng());
        candidates.first().copied()
    }
}

#[derive(Attachment)]
#[attachment(no_default)]
struct BotRoom {
    bot: String,
}

#[before(ctos::JoinGame)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
async fn before_join_game(player: &mut Player, join_game: &ctos::JoinGame) {
    if player.states.get::<srvpro::PlayerMove>().is_some() { return }
    let pass = join_game.pass.trim();
    if !pass.as_bytes().get(..2).is_some_and(|head| head.eq_ignore_ascii_case(b"AI")) { return }
    let bot_name = pass[2..].strip_prefix('#').unwrap_or_default();
    if !bot_name.is_empty() && bots().choose(bot_name).is_none() { return }
    let random = rand::thread_rng().gen_range(0..100000);
    let room_name = if bot_name.is_empty() { 
        format!("AI#{random}") 
    } else { 
        format!("BOT{bot_name},AI#{random}") 
    };
    RoomHost::new(&room_name).await;
    player.states.insert(srvpro::PlayerMove { room_name });
}

#[handler(Normal)]
#[register_to(MODES)]
fn bot_room(config: &mut RoomConfiguration, part: &str) -> bool {
    if let Some(bot) = slice_with_prefix::<String>(part, "BOT") {
        config.states.insert(BotRoom { bot });
        true
    } else { false }
}

#[handler(Normal)]
#[register_to(MODES)]
fn ai_room(config: &mut RoomConfiguration, part: &str) -> bool {
    if part != "AI" { return false }
    let bot = config.states.get::<BotRoom>().map(|bot_room| bot_room.bot.clone()).unwrap_or_default();
    config.hostinfo.rule = Rule::All;
    config.hostinfo.lflist = 0;
    config.hostinfo.time_limit = 999;
    config.srvpro_configuration.enable_plugins.remove(crate::plugin::reconnect::NAME);
    config.ygopru_configuration.enable_plugin_with_configuration(
        ygopro::plugin::terminate::NAME,
        ygopro::plugin::terminate::Configuration {
            terminate_when: SendTarget::Single(Netplayer::Player(0)),
            terminate_when_match_end: true
        }
    );
    config.states.insert(BotRoom { bot });
    true
}

#[after(srvpro::CreateProvider)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_create_provider(room: &mut Room, bot_room: &mut BotRoom) {
    let bots = bots();
    let Some(bot) = bots.choose(&bot_room.bot) else { return };
    invite(room, bot);
}

#[command]
#[register_to(CHAT_COMMANDS as ChatCommandHandler with &'static str)]
fn ai(room: &mut Room, _states: &mut Anymap, index: usize, args: &str) {
    let Some(player) = room.players.get_mut(index) else { return };
    if !player.states.get::<bool>().copied().unwrap_or(false) {
        player.send_message("只有房主可以召唤 AI。", Color::Red);
        return;
    }
    let bots = bots();
    let Some(bot) = bots.choose(args) else {
        player.send_message("找不到指定的 AI。", Color::Red);
        return;
    };
    invite(room, bot);
}

pub mod windbot_server {
    use std::process::Stdio;
    use std::sync::Arc;
    use std::sync::LazyLock;
    use std::time::Duration;

    use arc_swap::ArcSwapOption;
    use tokio::process::Command;
    use tokio::task::JoinHandle;
    use ygopro_data::constants::Color;
    use ygopro_derive::Configuration;

    use crate::GlobalHandler;
    use crate::message as srvpro;
    use crate::room::Request;
    use crate::room::Room;

    use super::Bot;
    use super::BotList;
    use super::BOT_PROVIDERS;
    use super::BotProvider;

    /// Name for activitating this module in the plugin system.
    pub static NAME: &'static str = module_path!();

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
        #[config(default = "BotList::default()")]
        pub bots: BotList,
    }

    struct Server;

    impl BotProvider for Server {
        fn bots(&self) -> BotList { 
            crate::configuration::get_configuration::<Configuration>().map(|configuration| configuration.bots).unwrap_or_default()
        }

        fn invite(&self, room: &Room, bot: &Bot) {
            let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return };
            let room_name = room.name.clone();
            let room_sender = room.request_sender.clone();
            let port = game_port(&configuration).to_string();
            let version = ygopro::PRO_VERSION.to_string();
            let query = serde_urlencoded::to_string(&[
                ("name", bot.name.as_str()),
                ("deck", bot.deck.as_str()),
                ("host", configuration.my_ip.as_str()),
                ("port", port.as_str()),
                ("dialog", bot.dialog.as_str()),
                ("version", version.as_str()),
                ("password", room_name.as_str()),
            ]).unwrap_or_default();
            let url = format!("http://{}:{}/?{}", configuration.server_ip, configuration.port, query);
            let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).build().into();
            tokio::task::spawn_blocking(move || {
                if let Err(error) = agent.get(&url).call() {
                    log::warn!("Failed to invite windbot to room {room_name}: {error}");
                    room_sender.unbounded_send(Request::Ex(srvpro::ServerMessage { color: Color::Red, message: "添加AI失败，可尝试输入 /ai 重新添加。".to_string(), target: None }.into(), 0)).ok();
                }
            });
        }
    }

    #[distributed_slice(BOT_PROVIDERS)]
    static SERVER: (&'static str, fn() -> Box<dyn BotProvider>) = ("server", || Box::new(Server));
    static PROCESS: LazyLock<ArcSwapOption<JoinHandle<()>>> = LazyLock::new(|| ArcSwapOption::empty());

    #[handler(srvpro::PluginEnabled)]
    #[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
    fn on_plugin_enabled(event: &srvpro::PluginEnabled) {
        if event.module_name != NAME { return }
        spawn();
    }

    #[handler(srvpro::PluginDisabled)]
    #[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
    fn on_plugin_disabled(event: &srvpro::PluginDisabled) {
        if event.module_name != NAME { return }
        abort();
    }

    pub fn spawn() {
        let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return };
        if configuration.spawn.is_empty() { return }
        if PROCESS.load().is_some() { return }
        PROCESS.store(Some(Arc::new(tokio::spawn(async move { run(configuration).await }))));
    }

    pub fn abort() {
        if let Some(handle) = PROCESS.swap(None) { handle.abort() }
    }

    fn game_port(configuration: &Configuration) -> u16 {
        if configuration.my_port != 0 { return configuration.my_port }
        crate::configuration::get_configuration::<crate::server::Configuration>().map(|main| main.port).unwrap_or(7911)
    }

    async fn run(configuration: Configuration) {
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
        PROCESS.store(None);
    }
}