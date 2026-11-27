//! Duel with the embedded WindBot AI.
//!
//! Enabling this module registers the `embedded` bot provider, which runs
//! WindBot inside this process instead of asking a ServerMode host.

use std::sync::Once;

use srvpro::GlobalHandler;
use srvpro::SRVPRO_GLOBAL_HANDLERS;
use srvpro::plugin::register_dependencies;
use srvpro::room::Room;
use srvpro::windbot::BOT_PROVIDERS;
use srvpro::windbot::Bot;
use srvpro::windbot::BotList;
use srvpro::windbot::BotProvider;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;

/// Name for activitating this module in the plugin system.
#[linkme::distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_dependencies!(srvpro::windbot::NAME);

#[derive(Clone, Configuration)]
#[config(sync, register_to = "srvpro::plugin::SRVPRO_CONFIGURATIONS")]
pub struct Configuration {
    /// Bots a player may summon. Defaults to every deck AI bundled with WindBot.
    #[config(default = "embedded_bot_list()")]
    pub bots: BotList,
}

fn embedded_bot_list() -> BotList {
    let Ok(json) = crate::list_bots() else { return BotList::default() };
    let Ok(rows) = serde_json::from_str::<Vec<[String; 5]>>(&json) else { return BotList::default() };
    rows.into_iter()
        .map(|[name, deck, _embedded_deck, dialog, level]| Bot { 
            name, deck, dialog, hidden: level != "Normal" }
        )
        .collect::<Vec<_>>()
        .into()
}

struct Embedded;

impl BotProvider for Embedded {
    fn bots(&self) -> BotList {
        srvpro::configuration::get_configuration::<Configuration>().map(|configuration| configuration.bots).unwrap_or_default()
    }

    fn invite(&self, room: &Room, bot: &Bot) {
        let port = srvpro::configuration::get_configuration::<srvpro::server::Configuration>().map(|configuration| configuration.port).unwrap_or(7911);
        let options = serde_json::json!({
            "Name": bot.name,
            "Deck": bot.deck,
            "Dialog": bot.dialog,
            "Host": "127.0.0.1",
            "Port": port,
            "HostInfo": room.name,
            "Version": *ygopro::PRO_VERSION,
        });
        let Ok(json) = serde_json::to_string(&options) else { return };
        let room_name = room.name.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(error) = crate::run(Some(&json)) {
                log::warn!("Failed to run windbot in room {room_name}: {error}");
            }
        });
    }
}

#[linkme::distributed_slice(BOT_PROVIDERS)]
static EMBEDDED: (&'static str, fn() -> Box<dyn BotProvider>) = ("embedded", || Box::new(Embedded));

#[handler(srvpro::message::PluginEnabled)]
#[register_to(SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn on_plugin_enabled(event: &srvpro::message::PluginEnabled) {
    if event.module_name != NAME { return }
    static CARDS_LOADED: Once = Once::new();
    CARDS_LOADED.call_once(|| {
        let data_manager = ygopro::managers::data_manager::load();
        if let Err(error) = crate::load_cards(data_manager.cards.values()) {
            log::warn!("Failed to load WindBot card data: {error}");
        }
    });
}
