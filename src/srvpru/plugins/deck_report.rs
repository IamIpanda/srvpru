// ============================================================
// deck_report
// ------------------------------------------------------------
//! Report deck to target endpoint when duel finish.
//! 
//! Dependency:
//! - [deck_recorder](super::deck_recorder)
//! - [position_recorder](super::position_recorder)
// ============================================================

use crate::srvpru::Handler;
use crate::ygopro::data::Deck;
use crate::ygopro::message::stoc;

set_configuration! {
    endpoint: String,
    access_key: String,
    arena: String
}

depend_on! {
    "deck_recorder",
    "position_recorder"
}

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_dependency()?;
    register_handlers();
    Ok(())
}

fn register_handlers() {
    Handler::follow_message::<stoc::DuelStart, _>(100, "deck_reporter", |context, _| Box::pin(async move {
        let decks = context.get_deck().ok_or(anyhow!("Can't get player used deck"))?;
        let deck = decks.history_decks.last().ok_or(anyhow!("Can't get player last deck"))?;
        let configuration = get_configuration();
        let report = DeckReport {
            access_key: configuration.access_key.clone(),
            deck: deck.clone(),
            player_name: context.get_player().ok_or(anyhow!("Can't get player"))?.lock().name.clone(),
            arena: configuration.arena.clone()
        };
        let client = reqwest::Client::new();
        client.post(&configuration.endpoint).form(&report.to_form()).send().await?;
        Ok(false)
    })).register();

    Handler::register_handlers("deck_reporter", crate::ygopro::message::Direction::STOC, vec!["deck_reporter"]);
}

struct DeckReport {
    access_key: String,
    deck: Deck,
    player_name: String,
    arena: String
}

impl DeckReport {
    fn to_form(self) -> Vec<(&'static str, String)> {
        vec! [
            ("accesskey", self.access_key),
            ("deck", self.deck.to_string()),
            ("playername", self.player_name),
            ("arena", self.arena)
        ]
    }
}