use std::str::FromStr;

use hashbrown::HashMap;
use ygopro_data::constants::DuelStage;
use ygopro_data::constants::Netplayer;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::plugin::base::stage::Stage;
use crate::plugin::register_dependencies;
use crate::room::Room;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;

pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::stage::NAME
);

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    pub mode: HideNameMode,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum HideNameMode {
    #[default]
    Disabled,
    Start,
    Always,
}

impl FromStr for HideNameMode {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "start" => HideNameMode::Start,
            "always" => HideNameMode::Always,
            _ => HideNameMode::Disabled,
        })
    }
}

impl Configuration {
    fn should_mask(&self, room: &Room, index: usize, player_enter: &stoc::HsPlayerEnter, stage: &Stage) -> bool {
        if self.mode == HideNameMode::Disabled { return false }
        if !is_random_room(&room.name) { return false }
        if stage.stage != DuelStage::Begin { return false }
        let pos = u8::from(player_enter.pos);
        if pos >= 4 { return false }
        if pos as usize == index { return false }
        true
    }
}

#[derive(Attachment)]
struct HideNameState {
    names: HashMap<u8, String>,
    revealed: bool,
}

#[handler(stoc::HsPlayerEnter)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_hs_player_enter(room: &mut Room, index: usize, player_enter: &stoc::HsPlayerEnter, stage: &mut Stage, state: &mut HideNameState, configuration: Configuration) -> Option<stoc::HsPlayerEnter> {
    if let Netplayer::Player(pos) = player_enter.pos {
        state.names.insert(pos, player_enter.name.to_string());
    }
    if configuration.should_mask(room, index, player_enter, stage) {
        let masked = format!("Player {}", u8::from(player_enter.pos) + 1);
        Some(stoc::HsPlayerEnter { name: masked.into(), pos: player_enter.pos })
    } else {
        None
    }
}

#[handler(stoc::DuelStart)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_start(room: &mut Room, stage: &mut Stage, state: &mut HideNameState, configuration: Configuration) {
    stage.stage = DuelStage::Dueling;
    if state.revealed { return }
    state.revealed = true;
    if configuration.mode != HideNameMode::Start { return }
    for (pos, name) in &state.names {
        room.broadcast(stoc::HsPlayerEnter { name: name.clone().into(), pos: Netplayer::Player(*pos) }.into());
    }
}

fn is_random_room(name: &str) -> bool {
    name.contains("RANDOM")
}
