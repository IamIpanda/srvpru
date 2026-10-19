//! Record the first attacker.

use ygopro_data::constants::CorePlayer;
use ygopro_data::constants::Netplayer;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::mode::TagFlag;
use crate::mode::opponent;
use crate::plugin::register_dependencies;
use crate::room::CTOS_HANDLERS;
use crate::room::ClientToServerHandler;
use crate::room::Room;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;

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
fn on_tp_result(room: &mut Room, index: usize, first_attack: &mut FirstAttack, tag: TagFlag, tp_result: &ctos::TpResult) {
    let Some(pos) = room.players.get(index).and_then(|player| player.states.get::<Netplayer>()).copied() else { return };
    let first_attacker = if tp_result.result == CorePlayer::FirstAttackPlayer { pos } else { opponent(tag.0, pos) };
    first_attack.first_attack.push(first_attacker);
    let teammate = tag.0.then(|| opponent(true, first_attacker));
    for (_, member) in room.players.iter_mut() {
        let Some(netplayer) = member.states.get::<Netplayer>().copied() else { continue };
        member.states.insert(if netplayer == first_attacker || teammate == Some(netplayer) { CorePlayer::FirstAttackPlayer } else { CorePlayer::SecondAttackPlayer });
    }
}

#[handler(stoc::DuelEnd)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_end(room: &mut Room) {
    for (_, member) in room.players.iter_mut() {
        member.states.remove::<CorePlayer>();
    }
}
