use std::str::FromStr;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;

use parking_lot::RwLock;
use rand::seq::SliceRandom;
use ygopro_data::constants::Color;
use ygopro_data::message::stoc;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::message as srvpro;
use crate::plugin::chat_command::CHAT_COMMANDS;
use crate::plugin::chat_command::ChatCommandHandler;
use crate::GlobalHandler;
use crate::room::Request;
use crate::room::Room;
use crate::room::ROOMS;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "true")]
    pub enabled: bool,
    #[config(default = "120000")]
    pub interval: u64,
    #[config(default = "String::from(\"Tip: \")")]
    pub prefix: String,
    pub tips: Tips
}

#[derive(Clone, Default)]
pub struct Tips(Arc<Vec<String>>);

impl FromStr for Tips {
    type Err = serde_json::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Tips(Arc::new(serde_json::from_str(value)?)))
    }
}

static TIMER: LazyLock<RwLock<Option<tokio::task::JoinHandle<()>>>> = LazyLock::new(|| RwLock::new(None));

#[handler(srvpro::PluginEnabled)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn on_plugin_enabled(event: &srvpro::PluginEnabled) {
    if event.module_name != NAME { return }
    start();
}

#[handler(srvpro::PluginDisabled)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn on_plugin_disabled(event: &srvpro::PluginDisabled) {
    if event.module_name != NAME { return }
    stop();
}

fn start() {
    let configuration = crate::configuration::get();
    let Some(configuration) = configuration.configurations.get::<Configuration>().cloned() else { return };
    if !configuration.enabled { return }
    let mut timer = TIMER.write();
    if timer.is_some() { return }
    *timer = Some(tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(configuration.interval));
        loop {
            interval.tick().await;
            let Some(message) = random_tip(&configuration) else { continue };
            let chat = stoc::Chat { player: Color::Babyblue.into(), msg: message.into() };
            let rooms = ROOMS.read();
            for host in rooms.values() {
                host.sender.unbounded_send(Request::Ex(srvpro::DirectSTOC { message: chat.clone().into(), target: None }.into(), 0)).ok();
            }
        }
    }));
}

fn stop() {
    if let Some(handle) = TIMER.write().take() {
        handle.abort();
    }
}

fn random_tip(configuration: &Configuration) -> Option<String> {
    let tip = configuration.tips.0.choose(&mut rand::thread_rng())?;
    Some(configuration.prefix.clone() + tip)
}

#[command]
#[register_to(CHAT_COMMANDS as ChatCommandHandler with &'static str)]
fn tip(room: &mut Room, index: usize, _args: &str) {
    let Some(tip_configuration) = room.configuration.configurations.get::<Configuration>() else { return };
    if let Some(message) = random_tip(tip_configuration) {
        room.players[index].send_message(&message, Color::Babyblue);
    }
}
