// ============================================================
// deck_recorder
// ------------------------------------------------------------
//! Record which deck each player used.
// ============================================================

use crate::ygopro::data::Deck;
use crate::ygopro::message::ctos;
use crate::srvpru::Handler;

player_attach! {
    start_deck: Deck,
    history_decks: Vec<Deck>
}

export_player_attach_as!(get_deck);

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}

fn register_handlers() {
    Handler::follow_message::<ctos::UpdateDeck, _>(100, "deck_recorder", |context, request| Box::pin(async move {
        if contains_player_attachment(context.addr) {
            let deck = Deck::from_data(request);
            insert_player_attachment(context, deck, Vec::new());
        }
        let mut attachment = get_player_attachment_sure(context);
        attachment.history_decks.push(Deck::from_data(request));
        Ok(false)
    })).register();
    register_player_attachment_dropper();
    register_player_attachment_mover();
    Handler::register_handlers("deck_recorder", crate::ygopro::message::Direction::CTOS, vec!["deck_recorder"]);
}
