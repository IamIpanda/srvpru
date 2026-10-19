//! Announce the low-life-point messages when a player is damaged or pays a cost.

use std::str::FromStr;
use std::sync::Arc;

use ygopro_data::constants::Color;
use ygopro_data::message::gm;
use ygopro_derive::Configuration;
use ygopro_derive::after;
use ygopro_derive::register_to;

use crate::plugin::base::lp::Lp;
use crate::plugin::register_dependencies;
use crate::room::GM_HANDLERS;
use crate::room::GameMessageHandler;
use crate::room::Room;

/// Name for activitating this module in the plugin system.
#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::lp::NAME
);

#[derive(Clone, Default)]
pub struct Message(Arc<str>);

impl FromStr for Message {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Message(value.into()))
    }
}

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    /// Line broadcast when a player's LP drops into the low range after damage.
    #[config(default = "Message::from_str(\"你的生命已经如风中残烛了！\").unwrap()")]
    pub lp_low_message_when_damaged: Message,
    /// Line broadcast when a player's LP drops into the low range after paying a cost.
    #[config(default = "Message::from_str(\"背水一战！\").unwrap()")]
    pub lp_low_message_when_paid_cost: Message,
}

#[after(gm::Damage)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_damage(room: &mut Room, index: usize, lp: &mut Lp, damage: &gm::Damage, configuration: Configuration) {
    if room.players.get(index).and_then(|player| player.states.get::<bool>()) != Some(&true) { return }
    let Some(remaining) = lp.lp.get(damage.player as usize) else { return };
    if *remaining <= 0 || *remaining > 100 { return }
    room.broadcast_message(Color::Pink, &configuration.lp_low_message_when_damaged.0);
}

#[after(gm::PayLPCost)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_pay_lp_cost(room: &mut Room, index: usize, lp: &mut Lp, pay_lp_cost: &gm::PayLPCost, configuration: Configuration) {
    if room.players.get(index).and_then(|player| player.states.get::<bool>()) != Some(&true) { return }
    let Some(remaining) = lp.lp.get(pay_lp_cost.player as usize) else { return };
    if *remaining <= 0 || *remaining > 100 { return }
    room.broadcast_message(Color::Pink, &configuration.lp_low_message_when_paid_cost.0);
}
