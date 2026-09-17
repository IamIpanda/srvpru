//! Limit the time players have to change their side deck between duels.

use std::time::Duration;

use ygopro_data::constants::Color;
use ygopro_data::message::gm;
use ygopro_data::message::stoc;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::message as srvpro;
use crate::room::GM_HANDLERS;
use crate::room::GameMessageHandler;
use crate::room::Player;
use crate::room::Request;
use crate::room::Room;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "3")]
    pub side_timeout: u64,
}

struct SideTimeoutTask(tokio::task::JoinHandle<()>);

#[handler(stoc::ChangeSide)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_change_side(room: &mut Room, index: usize, configuration: Configuration) {
    if configuration.side_timeout == 0 { return }
    let sender = room.request_sender.clone();
    let handle = tokio::spawn(async move {
        for remaining in (1..configuration.side_timeout).rev() {
            tokio::time::sleep(Duration::from_secs(60)).await;
            sender.unbounded_send(Request::Ex(srvpro::DirectSTOC { message: stoc::Chat { player: Color::Babyblue.into(), msg: format!("剩余 {} 分钟", remaining).into() }.into(), target: Some(index) }.into(), index)).ok();
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
        sender.unbounded_send(Request::Ex(srvpro::DirectSTOC { message: stoc::Chat { player: Color::Red.into(), msg: "换side超时".to_string().into() }.into(), target: Some(index) }.into(), index)).ok();
        sender.unbounded_send(Request::Ex(srvpro::ClientLeave { position: index }.into(), index)).ok();
    });
    if let Some(task) = room.players[index].states.remove::<SideTimeoutTask>() {
        task.0.abort();
    }
    room.players[index].states.insert(SideTimeoutTask(handle));
}

#[handler(gm::Start)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_duel_start(player: &mut Player) {
    if let Some(task) = player.states.get::<SideTimeoutTask>() {
        task.0.abort();
    }
}
