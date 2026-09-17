use ygopro_data::constants::CorePlayer;
use ygopro_data::constants::Netplayer;
use ygopro_data::message::ctos;
use ygopro_derive::Attachment;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::mode::TagFlag;
use crate::mode::opponent;
use crate::plugin::register_dependencies;
use crate::room::CTOS_HANDLERS;
use crate::room::ClientToServerHandler;
use crate::room::Player;

pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::position::NAME
);

#[derive(Attachment)]
pub struct FirstAttack {
    pub first_attack: Vec<Netplayer>,
}

#[handler(ctos::TpResult)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn on_tp_result(player: &mut Player, first_attack: &mut FirstAttack, tag: TagFlag, tp_result: &ctos::TpResult) {
    let Some(pos) = player.states.get::<Netplayer>() else { return };
    first_attack.first_attack.push(
        if tp_result.result == CorePlayer::FirstAttackPlayer {
            *pos
        } else {
            opponent(tag.0, *pos)
        }
    );
}
