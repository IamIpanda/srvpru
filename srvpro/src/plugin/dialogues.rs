use std::str::FromStr;
use std::sync::Arc;

use hashbrown::HashMap;
use linkme::distributed_slice;
use rand::seq::SliceRandom;
use ygopro_data::constants::Color;
use ygopro_data::constants::Location;
use ygopro_data::constants::Position;
use ygopro_data::message::gm;
use ygopro_data::message::stoc;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::room::GM_HANDLERS;
use crate::room::GameMessageHandler;
use crate::room::Room;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "true")]
    pub enabled: bool,
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

fn is_host(room: &Room, index: usize) -> bool {
    room.players.get(index).and_then(|player: &crate::room::Player| player.states.get::<bool>().copied()).unwrap_or(false)
}

#[handler(gm::Summoning)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_summoning(room: &mut Room, index: usize, summoning: &gm::Summoning, configuration: Configuration) {
    if !is_host(room, index) { return }
    if !configuration.enabled { return }
    if let Some(dialogue) = random_dialogue(&configuration, summoning.code) {
        send_dialogue(room, &dialogue);
    }
}

#[handler(gm::SpecialSummoning)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_special_summoning(room: &mut Room, index: usize, special_summoning: &gm::SpecialSummoning, configuration: Configuration) {
    if !is_host(room, index) { return }
    if !configuration.enabled { return }
    if let Some(dialogue) = random_dialogue(&configuration, special_summoning.code) {
        send_dialogue(room, &dialogue);
    }
}

#[handler(gm::Chaining)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_chaining(room: &mut Room, index: usize, chaining: &gm::Chaining, configuration: Configuration) {
    if !is_host(room, index) { return }
    if !configuration.enabled { return }
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
        room.broadcast(stoc::Chat {
            player: Color::Pink.into(),
            msg: line.into(),
        }.into());
    }
}

fn random_dialogue(configuration: &Configuration, code: u32) -> Option<String> {
    let dialogue = configuration.dialogues.0.get(&code)?.choose(&mut rand::thread_rng())?;
    Some(dialogue.clone())
}
