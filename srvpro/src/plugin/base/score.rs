use ygopro_data::constants::CorePlayer;
use ygopro_data::constants::Netplayer;
use ygopro_data::message::gm;
use ygopro_derive::Attachment;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::mode::TagFlag;
use crate::mode::opponent;
use crate::plugin::base::first_attack::FirstAttack;
use crate::plugin::register_dependencies;
use crate::room::GM_HANDLERS;
use crate::room::GameMessageHandler;
use crate::room::Player;

pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::position::NAME,
    crate::plugin::base::first_attack::NAME
);

#[derive(Attachment)]
pub struct Score {
    pub winners: Vec<Option<Netplayer>>,
}

#[handler(gm::Win)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_win(player: &mut Player, score: &mut Score, first_attack: &mut FirstAttack, tag: TagFlag, win: &gm::Win) {
    if player.states.get::<bool>() != Some(&true) { return; }
    let Some(first_attacker) = first_attack.first_attack.last() else { return };
    let winner = if win.winner == CorePlayer::FirstAttackPlayer {
        Some(*first_attacker)
    } else if win.winner == CorePlayer::None {
        None
    } else {
        Some(opponent(tag.0, *first_attacker))
    };
    score.winners.push(winner);
}
