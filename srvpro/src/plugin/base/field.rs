//! Track the duel field from game message.

use ygopro_data::complex::Complex;
use ygopro_data::constants::CorePlayer;
use ygopro_data::constants::Netplayer;
use ygopro_data::data::QueryData;
use ygopro_data::data::UpdateCardInfo;
use ygopro_data::field;
use ygopro_data::message::gm;
use ygopro_derive::Attachment;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::plugin::register_dependencies;
use crate::room::GM_HANDLERS;
use crate::room::GameMessageHandler;
use crate::room::Player;

/// Name for activitating this module in the plugin system.
pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::position::NAME,
    crate::plugin::base::first_attack::NAME
);

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "CorePlayer::All", not_from_env)]
    pub perspective: CorePlayer,
}

#[derive(Attachment)]
pub struct Field {
    pub field: field::Field,
}

#[handler(ygopro_handler::All)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_game_message(player: &mut Player, field: &mut Field, message: &Complex<gm::Message>, configuration: Configuration) {
    let Some(netplayer) = player.states.get::<Netplayer>().copied() else { return };
    if matches!(netplayer, Netplayer::Observer(_)) { return }
    let message = &**message;
    let masked = is_masked(message);
    match configuration.perspective {
        CorePlayer::FirstAttackPlayer | CorePlayer::SecondAttackPlayer => {
            if player.states.get::<CorePlayer>() != Some(&configuration.perspective) { return }
        }
        CorePlayer::All | CorePlayer::Rule => if masked == Some(true) { return },
        CorePlayer::None => if masked == Some(false) { return },
    }
    if masked.is_none() && player.states.get::<bool>() != Some(&true) { return }
    field.field += message;
}

fn is_wiped(data: &UpdateCardInfo) -> bool {
    match data {
        UpdateCardInfo::Empty => true,
        UpdateCardInfo::Data(queries) => queries.iter().all(|query| matches!(query, QueryData::Clear)),
    }
}

fn is_masked(message: &gm::Message) -> Option<bool> {
    match message {
        gm::Message::UpdateData(message) => Some(message.data.iter().all(is_wiped)),
        gm::Message::UpdateCard(message) => Some(is_wiped(&message.data)),
        gm::Message::SelectCard(message) => Some(message.positions.iter().all(|(code, _)| code.id() == 0)),
        gm::Message::SelectTribute(message) => Some(message.tributes.iter().all(|(position, _)| position.code.id() == 0)),
        gm::Message::SelectUnselectCard(message) => Some(message.positions1.iter().chain(message.positions2.iter()).all(|(code, _)| code.id() == 0)),
        gm::Message::Move(message) => Some(message.code == 0),
        gm::Message::Set(message) => Some(message.code == 0),
        gm::Message::SpecialSummoning(message) => Some(message.code == 0),
        gm::Message::Draw(message) => Some(message.codes.iter().all(|code| code.id() == 0)),
        gm::Message::ShuffleHand(message) => Some(message.codes.iter().all(|code| *code == 0)),
        gm::Message::ShuffleExtra(message) => Some(message.codes.iter().all(|code| *code == 0)),
        gm::Message::TagSwap(message) => Some(message.hand_codes.iter().chain(message.extra_codes.iter()).all(|code| code.id() == 0)),
        _ => None,
    }
}
