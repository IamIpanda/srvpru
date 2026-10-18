use std::str::FromStr;
use std::sync::Arc;
use std::sync::LazyLock;

use hashbrown::HashMap;
use parking_lot::RwLock;
use rand::Rng;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::Configuration;
use ygopro_derive::before;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::message as srvpro;
use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::Normal;
use crate::mode::RoomConfiguration;
use crate::room::*;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "String::from(\"S\")")]
    pub default: String,
    pub modes: Modes,
    #[config(default = "90")]
    pub ready_timeout: u64,
    #[config(default = "20")]
    pub start_timeout: u64,
    #[config(default = "3")]
    pub min_surrender_turn: u32,
    #[config(default = "Modes::from_str(\"[\\\"S\\\",\\\"T\\\"]\").unwrap()")]
    pub surrender_limited_modes: Modes,
}

#[derive(Clone, Default)]
pub struct Modes(Arc<[String]>);

impl FromStr for Modes {
    type Err = serde_json::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Modes(serde_json::from_str::<Vec<String>>(value)?.into()))
    }
}

#[derive(Attachment)]
#[attachment(no_default)]
pub struct RandomRoom {
    pub random_type: String,
    pub max_player: usize,
}

struct WaitingRoom {
    random_type: String,
    players: usize,
}
static WAITING_ROOMS: LazyLock<RwLock<HashMap<String, WaitingRoom>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

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
    allocate_room(random_type, |random| format!("{},RANDOM#{}", random_type, random)).await
}

pub async fn allocate_room(random_type: &str, room_name_of: impl Fn(u32) -> String + Send) -> String {
    let available = WAITING_ROOMS.read().iter()
        .find(|(_, waiting_room)| waiting_room.random_type == random_type && waiting_room.players < max_players_of(random_type))
        .map(|(room_name, _)| room_name.clone());
    if let Some(room_name) = available { return room_name }
    let room_name = room_name_of(rand::thread_rng().gen_range(0..100000));
    WAITING_ROOMS.write().insert(room_name.clone(), WaitingRoom { random_type: random_type.to_string(), players: 0 });
    RoomHost::new(&room_name).await;
    room_name
}

#[handler(Normal)]
#[register_to(MODES)]
fn random_room(config: &mut RoomConfiguration, part: &str) -> bool {
    if part != "RANDOM" { return false }
    let random_type = config.name.split(',').next().unwrap_or_default().to_uppercase();
    config.states.insert(RandomRoom { random_type: random_type.clone(), max_player: max_players_of(&random_type) });
    true
}

#[before(ctos::JoinGame)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
async fn before_join_game(player: &mut Player, join_game: &ctos::JoinGame) {
    if player.states.get::<srvpro::PlayerMove>().is_some() { return; }
    let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return };
    let Some(random_type) = random_type_of(join_game.pass.trim(), &configuration) else { return };
    let room_name = allocate_random_room(&random_type).await;
    player.states.insert(srvpro::PlayerMove { room_name });
}

#[after(srvpro::PlayerJoin)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_player_join(room: &mut Room) {
    if let Some(waiting_room) = WAITING_ROOMS.write().get_mut(&room.name) {
        waiting_room.players += 1;
    }
}

#[after(srvpro::ClientLeave)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_client_leave(room: &mut Room) {
    if let Some(waiting_room) = WAITING_ROOMS.write().get_mut(&room.name) {
        waiting_room.players = waiting_room.players.saturating_sub(1);
    }
}

#[handler(stoc::DuelStart)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_start(room: &mut Room) {
    WAITING_ROOMS.write().remove(&room.name);
}

#[handler(srvpro::Terminate)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_terminate(room: &mut Room) {
    WAITING_ROOMS.write().remove(&room.name);
}

mod ready_timeout {
    use std::time::Duration;

    use futures::channel::mpsc;
    use tokio::task::JoinHandle;
    use ygopro_data::constants::Color;
    use ygopro_data::constants::Netplayer;
    use ygopro_data::constants::PlayerChangeState;
    use ygopro_data::message::stoc;
    use ygopro_derive::before;
    use ygopro_derive::handler;
    use ygopro_derive::register_to;

    use crate::message as srvpro;
    use crate::plugin::random_match::Configuration;
    use crate::plugin::random_match::RandomRoom;
    use crate::room::Player;
    use crate::room::Request;
    use crate::room::Room;
    use crate::room::SRVPRO_HANDLERS;
    use crate::room::STOC_HANDLERS;
    use crate::room::Sender;
    use crate::room::ServerToClientHandler;
    use crate::room::SrvproMessageHandler;

    #[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
    pub static NAME: &'static str = module_path!();

    struct ReadyTimeout(JoinHandle<()>);

    fn set_ready_timeout(sender: mpsc::UnboundedSender<Request>, player: &mut Player, position: usize, timeout: u64) {
        if let Some(timeout) = player.states.remove::<ReadyTimeout>() { timeout.0.abort() }
        if timeout == 0 { return }
        let name = player.states.get::<String>().cloned().unwrap_or_default();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(timeout)).await;
            sender.unbounded_send(Request::Ex(srvpro::ServerMessage { color: Color::Red, message: format!("{}被系统请出了房间", name), target: None }.into(), 0)).ok();
            sender.unbounded_send(Request::Ex(srvpro::ClientLeave { position }.into(), position)).ok();
        });
        player.states.insert(ReadyTimeout(handle));
    }

    #[handler(stoc::TypeChange)]
    #[register_to(STOC_HANDLERS as ServerToClientHandler)]
    fn on_type_change(_: &mut RandomRoom, player: &mut Player, index: usize, type_change: &stoc::TypeChange, sender: Sender, configuration: Configuration) {
        if !matches!(type_change.player, Netplayer::Player(_)) {
            if let Some(timeout) = player.states.remove::<ReadyTimeout>() { timeout.0.abort() }
            return
        }
        set_ready_timeout(sender.0, player, index, configuration.ready_timeout);
    }

    #[handler(stoc::HsPlayerChange)]
    #[register_to(STOC_HANDLERS as ServerToClientHandler)]
    fn on_player_change(room: &mut Room, _: &mut RandomRoom, change: &stoc::HsPlayerChange, configuration: Configuration) {
        let seat: Netplayer = change.status.player();
        let Some((position, player)) = room.players.iter_mut().find(|(_, player)| player.states.get::<Netplayer>() == Some(&seat)) else { return };
        if change.status.state() == PlayerChangeState::Notready && matches!(seat, Netplayer::Player(_)) {
            set_ready_timeout(room.request_sender.clone(), player, position, configuration.ready_timeout);
        } else if let Some(timeout) = player.states.remove::<ReadyTimeout>() {
            timeout.0.abort()
        }
    }

    #[before(srvpro::ClientLeave, priority = 2)]
    #[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
    fn on_client_leave(room: &mut Room, client_leave: &srvpro::ClientLeave) {
        if let Some(player) = room.players.get_mut(client_leave.position) && 
           let Some(timeout) = player.states.remove::<ReadyTimeout>() { timeout.0.abort() }
    }
}

mod start_timeout {
    use std::time::Duration;

    use tokio::task::JoinHandle;
    use ygopro_data::constants::Color;
    use ygopro_data::constants::Netplayer;
    use ygopro_data::constants::PlayerChangeState;
    use ygopro_data::message::stoc;
    use ygopro_derive::Attachment;
    use ygopro_derive::before;
    use ygopro_derive::handler;
    use ygopro_derive::register_to;

    use crate::message as srvpro;
    use crate::plugin::random_match::Configuration;
    use crate::plugin::random_match::RandomRoom;
    use crate::room::Request;
    use crate::room::Room;
    use crate::room::SRVPRO_HANDLERS;
    use crate::room::STOC_HANDLERS;
    use crate::room::ServerToClientHandler;
    use crate::room::SrvproMessageHandler;

    #[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
    pub static NAME: &'static str = module_path!();

    #[derive(Attachment)]
    struct StartTimeout {
        handle: Option<JoinHandle<()>>,
    }

    struct Ready;

    fn ready_players(room: &Room, max_player: usize) -> usize {
        room.players.iter().filter(|(_, player)| {
            let Some(&Netplayer::Player(position)) = player.states.get::<Netplayer>() else { return false };
            (position as usize) < max_player && player.states.get::<bool>() != Some(&true) && player.states.get::<Ready>().is_some()
        }).count()
    }

    fn sync_start_timeout(room: &Room, random_room: &RandomRoom, start_timeout: &mut StartTimeout, timeout: u64) {
        if timeout == 0 || ready_players(room, random_room.max_player) + 1 < random_room.max_player {
            if let Some(handle) = start_timeout.handle.take() { handle.abort() }
            return
        }
        set_start_timeout(room, start_timeout, timeout);
    }

    fn set_start_timeout(room: &Room, start_timeout: &mut StartTimeout, timeout: u64) {
        if start_timeout.handle.is_some() { return }
        let Some(position) = room.players.iter().find_map(|(position, player)| (player.states.get::<bool>() == Some(&true)).then_some(position)) else { return };
        let name = room.players[position].states.get::<String>().cloned().unwrap_or_default();
        let sender = room.request_sender.clone();
        let handle = tokio::spawn(async move {
            for remaining in (1..timeout).rev() {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if remaining % 5 != 0 { continue }
                let color = if remaining <= 9 { Color::Red } else { Color::Lightblue };
                sender.unbounded_send(Request::Ex(srvpro::ServerMessage { color, message: format!("{}秒后房主若不开始游戏将被请出房间", remaining), target: None }.into(), 0)).ok();
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            sender.unbounded_send(Request::Ex(srvpro::ServerMessage { color: Color::Red, message: format!("{}未开始游戏，已被请出房间", name), target: None }.into(), 0)).ok();
            sender.unbounded_send(Request::Ex(srvpro::ClientLeave { position }.into(), position)).ok();
        });
        start_timeout.handle = Some(handle);
    }

    #[handler(stoc::HsPlayerChange)]
    #[register_to(STOC_HANDLERS as ServerToClientHandler)]
    fn on_player_change(room: &mut Room, random_room: &mut RandomRoom, start_timeout: &mut StartTimeout, change: &stoc::HsPlayerChange, configuration: Configuration) {
        let seat: Netplayer = change.status.player();
        let Some((_, player)) = room.players.iter_mut().find(|(_, player)| player.states.get::<Netplayer>() == Some(&seat)) else { return };
        if change.status.state() == PlayerChangeState::Ready {
            player.states.insert(Ready);
        } else {
            player.states.remove::<Ready>();
        }
        sync_start_timeout(room, random_room, start_timeout, configuration.start_timeout);
    }

    #[before(srvpro::ClientLeave, priority = 2)]
    #[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
    fn on_client_leave(room: &mut Room, _: &mut RandomRoom, start_timeout: &mut StartTimeout, client_leave: &srvpro::ClientLeave) {
        let seated = room.players.get(client_leave.position).is_some_and(|player| matches!(player.states.get::<Netplayer>(), Some(Netplayer::Player(_))));
        if seated && let Some(handle) = start_timeout.handle.take() { handle.abort() }
    }

    #[handler(stoc::DuelStart)]
    #[register_to(STOC_HANDLERS as ServerToClientHandler)]
    fn on_duel_start(_: &mut RandomRoom, start_timeout: &mut StartTimeout) {
        if let Some(handle) = start_timeout.handle.take() { handle.abort() }
    }
}

mod no_early_surrender {
    use ygopro_data::constants::Color;
    use ygopro_data::message::ctos;
    use ygopro_derive::before;
    use ygopro_derive::register_to;
    use ygopro_handler::StopFlag;

    use crate::plugin::base::turn::Turn;
    use crate::plugin::random_match::Configuration;
    use crate::plugin::random_match::RandomRoom;
    use crate::plugin::register_dependencies;
    use crate::room::Player;
    use crate::room::CTOS_HANDLERS;
    use crate::room::ClientToServerHandler;

    #[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
    pub static NAME: &'static str = module_path!();

    register_dependencies!(crate::plugin::base::turn::NAME);

    #[before(ctos::Surrender)]
    #[register_to(CTOS_HANDLERS as ClientToServerHandler)]
    fn on_surrender(random_room: &mut RandomRoom, player: &mut Player, turn: &mut Turn, stop: &mut StopFlag, configuration: Configuration) -> &'static str {
        if configuration.min_surrender_turn == 0 || turn.turn >= configuration.min_surrender_turn { return "continue" }
        if !configuration.surrender_limited_modes.0.iter().any(|mode| mode == &random_room.random_type) { return "continue" }
        player.send_message(&format!("决斗开始后 {} 回合内不能投降。", configuration.min_surrender_turn), Color::Babyblue);
        stop.0 = true;
        "cancel"
    }
}
