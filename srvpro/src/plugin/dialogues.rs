//! Summon lines: send a quote when a monster is summoned.

use std::str::FromStr;
use std::sync::Arc;

use hashbrown::HashMap;
use rand::seq::SliceRandom;
use ygopro_data::constants::Color;
use ygopro_data::constants::Location;
use ygopro_data::constants::Position;
use ygopro_data::message::gm;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::plugin::base::position::Host;
use crate::plugin::register_dependencies;
use crate::room::GM_HANDLERS;
use crate::room::GameMessageHandler;
use crate::room::Room;

/// Name for activitating this module in the plugin system.
#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::position::NAME
);

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    /// Summon lines keyed by card id.
    pub dialogues: Dialogues
}

#[derive(Clone, Default)]
pub struct Dialogues(Arc<HashMap<u32, Vec<String>>>);

impl FromStr for Dialogues {
    type Err = serde_json::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Dialogues(Arc::new(serde_json::from_str(value)?)))
    }
}

#[derive(Default)]
struct ReadyTrap(bool);

#[handler(gm::Summoning)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_summoning(room: &mut Room, _: Host, summoning: &gm::Summoning, configuration: Configuration) {
    if let Some(dialogue) = random_dialogue(&configuration, summoning.code) {
        send_dialogue(room, &dialogue);
    }
}

#[handler(gm::SpecialSummoning)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_special_summoning(room: &mut Room, _: Host, special_summoning: &gm::SpecialSummoning, configuration: Configuration) {
    if let Some(dialogue) = random_dialogue(&configuration, special_summoning.code) {
        send_dialogue(room, &dialogue);
    }
}

#[handler(gm::Chaining)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_chaining(room: &mut Room, index: usize, _: Host, chaining: &gm::Chaining, configuration: Configuration) {
    let dialogue = {
        let Some(player) = room.players.get_mut(index) else { return };
        let ready_trap = player.states.entry::<ReadyTrap>().or_default();
        let dialogue = if chaining.current.location.contains(Location::SZone) && ready_trap.0 {
            random_dialogue(&configuration, chaining.card)
        } else { None };
        ready_trap.0 = false;
        dialogue
    };
    if let Some(dialogue) = dialogue {
        send_dialogue(room, &dialogue);
    }
}

#[handler(gm::PositionChange)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_position_change(room: &mut Room, index: usize, position_change: &gm::PositionChange) {
    let Some(player) = room.players.get_mut(index) else { return };
    let ready_trap = player.states.entry::<ReadyTrap>().or_default();
    ready_trap.0 = position_change.location.contains(Location::SZone)
        && position_change.previous_position.intersects(Position::Facedown)
        && position_change.current_position.intersects(Position::Faceup);
}

fn send_dialogue(room: &Room, dialogue: &str) {
    for line in dialogue.lines() {
        room.broadcast_message(Color::Pink, line);
    }
}

fn random_dialogue(configuration: &Configuration, code: u32) -> Option<String> {
    let dialogue = configuration.dialogues.0.get(&code)?.choose(&mut rand::thread_rng())?;
    Some(dialogue.clone())
}
