use ygopro_data::message::gm;
use ygopro_derive::Attachment;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::message as srvpro;
use crate::room::Request;
use crate::room::Room;
use crate::room::GM_HANDLERS;
use crate::room::GameMessageHandler;

pub static NAME: &'static str = module_path!();

#[derive(Attachment)]
pub struct Lp {
    pub lp: [i32; 2],
}

#[handler(gm::Start)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_start(room: &mut Room, index: usize, lp: &mut Lp, start: &gm::Start) {
    if room.players.get(index).and_then(|player| player.states.get::<bool>()) != Some(&true) { return }
    lp.lp = [start.player1_lp, start.player2_lp];
    room.request_sender.unbounded_send(Request::Ex(srvpro::LPChanged.into(), index)).ok();
}

#[handler(gm::Damage)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_damage(room: &mut Room, index: usize, lp: &mut Lp, damage: &gm::Damage) {
    if room.players.get(index).and_then(|player| player.states.get::<bool>()) != Some(&true) { return }
    if let Some(slot) = lp.lp.get_mut(damage.player as usize) {
        *slot = slot.saturating_sub(damage.value).max(0);
        room.request_sender.unbounded_send(Request::Ex(srvpro::LPChanged.into(), index)).ok();
    }
}

#[handler(gm::Recover)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_recover(room: &mut Room, index: usize, lp: &mut Lp, recover: &gm::Recover) {
    if room.players.get(index).and_then(|player| player.states.get::<bool>()) != Some(&true) { return }
    if let Some(slot) = lp.lp.get_mut(recover.player as usize) {
        *slot = slot.saturating_add(recover.value);
        room.request_sender.unbounded_send(Request::Ex(srvpro::LPChanged.into(), index)).ok();
    }
}

#[handler(gm::LPUpdate)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_lp_update(room: &mut Room, index: usize, lp: &mut Lp, lp_update: &gm::LPUpdate) {
    if room.players.get(index).and_then(|player| player.states.get::<bool>()) != Some(&true) { return }
    if let Some(slot) = lp.lp.get_mut(lp_update.player as usize) {
        *slot = lp_update.lp;
        room.request_sender.unbounded_send(Request::Ex(srvpro::LPChanged.into(), index)).ok();
    }
}

#[handler(gm::PayLPCost)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_pay_lp_cost(room: &mut Room, index: usize, lp: &mut Lp, pay_lp_cost: &gm::PayLPCost) {
    if room.players.get(index).and_then(|player| player.states.get::<bool>()) != Some(&true) { return }
    if let Some(slot) = lp.lp.get_mut(pay_lp_cost.player as usize) {
        *slot = slot.saturating_sub(pay_lp_cost.cost).max(0);
        room.request_sender.unbounded_send(Request::Ex(srvpro::LPChanged.into(), index)).ok();
    }
}
