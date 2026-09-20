use linkme::distributed_slice;

use ygopro_data::message::ctos;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::Normal;
use crate::mode::RoomConfiguration;
use crate::room::ClientToServerPrecursorHandler;
use crate::room::Player;
use crate::room::CTOS_PREHANDLERS;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

pub struct OriginName(pub String);

#[handler(ctos::PlayerInfo, priority = 250)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
fn on_player_info(player: &mut Player, player_info: &ctos::PlayerInfo) -> Option<ctos::PlayerInfo> {
    player.states.insert(OriginName(player_info.name.to_string()));
    let index = player_info.name.find('$')?;
    let name = player_info.name[..index].to_string();
    Some(ctos::PlayerInfo { name: name.into() })
}

#[handler(Normal)]
#[register_to(MODES)]
fn virtual_password(config: &mut RoomConfiguration) {
    let Some(index) = config.origin_name.find('$') else { return };
    config.name = config.origin_name[..index].to_string();
}
