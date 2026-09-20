//! Filter player names, chat messages and room names against configurable
//! bad-word groups, using Aho-Corasick for multi-pattern matching.

use std::ops::Mul;
use std::str::FromStr;
use std::sync::Arc;

use aho_corasick::AhoCorasick;
use serde::Deserialize;
use hashbrown::HashMap;

use ygopro_data::complex::Complex;
use ygopro_data::constants::Netplayer;
use ygopro_data::constants::Color;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Configuration;
use ygopro_handler::StopFlag;

use crate::plugin::register_dependencies;
use crate::room::CTOS_HANDLERS;
use crate::room::CTOS_PREHANDLERS;
use crate::room::ClientToServerHandler;
use crate::room::ClientToServerPrecursorHandler;
use crate::room::Player;
use crate::room::Room;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::position::NAME
);

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "true")]
    pub check_room_name: bool,
    #[config(default = "true")]
    pub check_player_name: bool,
    pub groups: Groups,
}

#[derive(Clone, Default)]
pub struct Groups(HashMap<String, Group>);

impl FromStr for Groups {
    type Err = serde_json::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let raw: HashMap<String, RawGroup> = serde_json::from_str(value)?;
        let groups = raw.into_iter().filter_map(|(name, raw)| {
            AhoCorasick::builder()
                .ascii_case_insensitive(true)
                .build(raw.words.iter().map(String::as_str))
                .ok()
                .map(|automaton| (name, Group { strategy: raw.strategy, words: Arc::new(automaton) }))
        }).collect();
        Ok(Groups(groups))
    }
}

#[derive(Clone)]
struct Group {
    strategy: Strategy,
    words: Arc<AhoCorasick>,
}

#[derive(Deserialize)]
struct RawGroup {
    strategy: Strategy,
    words: Vec<String>,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    #[default]
    Block,
    Warn,
    Echo,
    Kick,
}

impl Mul for Strategy {
    type Output = Strategy;

    fn mul(self, other: Strategy) -> Strategy {
        match (self, other) {
            (Strategy::Kick, _) | (_, Strategy::Kick) => Strategy::Kick,
            (Strategy::Warn, _) | (_, Strategy::Warn) => Strategy::Warn,
            (Strategy::Echo, _) | (_, Strategy::Echo) => Strategy::Echo,
            _ => Strategy::Block,
        }
    }
}

fn check(text: &str, groups: &Groups) -> Option<Strategy> {
    let mut worst = None;
    for group in groups.0.values() {
        if group.words.is_match(text) {
            worst = Some(match worst {
                Some(current) => current * group.strategy,
                None => group.strategy,
            });
        }
    }
    worst
}

#[handler(ctos::PlayerInfo, priority = 251)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
fn on_player_info(player: &mut Player, player_info: &ctos::PlayerInfo) -> Option<&'static str> {
    let configuration = crate::configuration::get_configuration::<Configuration>()?;
    if !configuration.check_player_name { return None }
    let strategy = check(&*player_info.name, &configuration.groups)?;
    match strategy {
        Strategy::Warn => {
            player.send_message("昵称包含不当词汇，请及时修改", Color::Red);
            None
        }
        _ => {
            player.send_message("昵称包含不当词汇，你已被踢出服务器", Color::Red);
            Some("kick")
        }
    }
}

#[before(ctos::JoinGame)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
fn before_join_game(join_game: &ctos::JoinGame, stop: &mut StopFlag) -> Option<&'static str> {
    let configuration = crate::configuration::get_configuration::<Configuration>()?;
    if !configuration.check_room_name { return None }
    check(&*join_game.pass, &configuration.groups)?;
    stop.0 = true;
    Some("kick")
}

#[before(ctos::Chat)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn before_chat(room: &mut Room, index: usize, chat: &ctos::Chat, stop: &mut StopFlag, configuration: Configuration) -> Option<&'static str> {
    let strategy = check(&*chat.msg, &configuration.groups)?;
    stop.0 = true;
    match strategy {
        Strategy::Block => Some("cancel"),
        Strategy::Warn => {
            if let Some(player) = room.players.get(index) {
                player.send_message("请勿发送不当言论", Color::Red);
            }
            Some("cancel")
        }
        Strategy::Echo => {
            if let Some(player) = room.players.get(index) && let Some(netplayer) = player.states.get::<Netplayer>() {
                player.server_to_client_sink_towards_network.unbounded_send(Complex::from_message(stoc::Chat {
                    player: (*netplayer).into(),
                    msg: chat.msg.clone(),
                }.into())).ok();
            }
            Some("cancel")
        }
        Strategy::Kick => {
            if let Some(player) = room.players.get(index) {
                player.send_message("发言包含不当词汇，你已被移出房间", Color::Red);
            }
            Some("kick")
        }
    }
}
