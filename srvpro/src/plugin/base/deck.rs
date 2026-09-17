use ygopro_data::data::Deck;
use ygopro_data::message::ctos;

use crate::plugin::base::count::Count;
use crate::plugin::register_dependencies;
use crate::room::Player;
use crate::room::CTOS_HANDLERS;
use crate::room::ClientToServerHandler;

pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::count::NAME
);

#[after(ctos::UpdateDeck)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn on_deck_record(player: &mut Player, update_deck: &ctos::UpdateDeck, count: &mut Count) {
    player.states.entry::<Vec<Deck>>().or_insert(vec![]).insert(count.count as usize, update_deck.deck.clone());
}
