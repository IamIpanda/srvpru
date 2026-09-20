use std::str::FromStr;
use std::sync::LazyLock;

use hashbrown::HashMap;
use parking_lot::RwLock;
use rand::Rng;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Configuration;

use crate::message as srvpro;
use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::Normal;
use crate::mode::RoomConfiguration;
use crate::room::*;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

type Anymap = anymap3::Map<dyn std::any::Any + Send>;

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "String::from(\"S\")")]
    pub default: String,
    pub modes: Modes,
}

#[derive(Clone, Default)]
pub struct Modes(Vec<String>);

impl FromStr for Modes {
    type Err = serde_json::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Modes(serde_json::from_str(value)?))
    }
}

pub struct RandomRoom;

struct RandomRoomState {
    random_type: String,
    players: usize,
}
static RANDOM_ROOMS: LazyLock<RwLock<HashMap<String, RandomRoomState>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

fn random_type_of(pass: &str, configuration: &Configuration) -> Option<String> {
    let upper = pass.trim().to_uppercase();
    if upper.is_empty() {
        return Some(configuration.default.clone());
    }
    configuration.modes.0.iter().find(|mode| mode.as_str() == upper).cloned()
}

fn max_players_of(random_type: &str) -> usize {
    if random_type == "T" { 4 } else { 2 }
}

async fn allocate_random_room(random_type: &str) -> String {
    let available = RANDOM_ROOMS.read().iter()
        .find(|(_, room)| room.random_type == random_type && room.players < max_players_of(random_type))
        .map(|(room_name, _)| room_name.clone());
    if let Some(room_name) = available { return room_name }
    let room_name = format!("{},RANDOM#{}", random_type, rand::thread_rng().gen_range(0..100000));
    RANDOM_ROOMS.write().insert(room_name.clone(), RandomRoomState { random_type: random_type.to_string(), players: 0 });
    RoomHost::new(&room_name).await;
    room_name
}

#[handler(Normal)]
#[register_to(MODES)]
fn random_room(config: &mut RoomConfiguration, part: &str) -> bool {
    if part != "RANDOM" { return false }
    config.states.insert(RandomRoom);
    true
}

#[before(ctos::JoinGame)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
async fn before_join_game(states: &mut Anymap, join_game: &ctos::JoinGame) {
    if states.get::<srvpro::PlayerMove>().is_some() { return; }
    let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return };
    let Some(random_type) = random_type_of(join_game.pass.trim(), &configuration) else { return };
    let room_name = allocate_random_room(&random_type).await;
    states.insert(srvpro::PlayerMove { room_name });
}

#[after(srvpro::PlayerJoin)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_player_join(room: &mut Room) {
    if let Some(random_room) = RANDOM_ROOMS.write().get_mut(&room.name) {
        random_room.players += 1;
    }
}

#[after(srvpro::ClientLeave)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_client_leave(room: &mut Room) {
    if let Some(random_room) = RANDOM_ROOMS.write().get_mut(&room.name) {
        random_room.players = random_room.players.saturating_sub(1);
    }
}

#[handler(stoc::DuelStart)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_start(room: &mut Room) {
    RANDOM_ROOMS.write().remove(&room.name);
}

#[handler(srvpro::Terminate)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_terminate(room: &mut Room) {
    RANDOM_ROOMS.write().remove(&room.name);
}
