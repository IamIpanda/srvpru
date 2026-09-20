//! Post each player's deck at duel start, mirroring the original srvpro's
//! deck_log module.

use std::time::Duration;

use ygopro_data::data::Deck;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::plugin::register_dependencies;
use crate::room::Room;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;

pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::deck::NAME,
    crate::plugin::base::name::NAME
);

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    pub url: String,
    pub access_key: String,
    pub arena: String,
}

#[derive(Attachment)]
pub struct DeckReported {
    reported: bool,
}

#[handler(stoc::DuelStart)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_start(room: &mut Room, deck_reported: &mut DeckReported, configuration: Configuration) {
    if deck_reported.reported { return }
    if configuration.url.is_empty() {
        log::warn!("deck report url is not configured, skip posting decks.");
        return;
    }
    let mut forms = vec![];
    for (_index, player) in room.players.iter() {
        let Some(decks) = player.states.get::<Vec<Deck>>() else { continue };
        let Some(deck) = decks.first() else { continue };
        let Some(player_name) = player.states.get::<String>() else { continue };
        forms.push(build_form(deck, player_name, &configuration));
    }
    deck_reported.reported = true;
    let url = configuration.url;
    tokio::task::spawn_blocking(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).build().into();
        for form in forms {
            match agent.post(&url).send_form(form) {
                Ok(response) => log::info!("deck report posted: {}", response.status()),
                Err(error) => log::warn!("deck report post failed: {}", error),
            }
        }
    });
}

fn build_form(deck: &Deck, playername: &str, configuration: &Configuration) -> Vec<(String, String)> {
    let mut form: Vec<(String, String)> = Vec::new();
    form.push(("accesskey".to_string(), configuration.access_key.clone()));
    form.push(("deck".to_string(), deck.to_string()));
    form.push(("playername".to_string(), playername.to_string()));
    form.push(("arena".to_string(), configuration.arena.clone()));
    form
}
