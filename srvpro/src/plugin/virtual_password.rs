use linkme::distributed_slice;

use ygopro_data::message::ctos;

use crate::room::ClientToServerPrecursorHandler;
use crate::room::CTOS_PREHANDLERS;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[handler(ctos::PlayerInfo, priority = 250)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
fn on_player_info(player_info: &ctos::PlayerInfo) -> Option<ctos::PlayerInfo> {
    let index = player_info.name.find('$')?;
    let name = player_info.name[..index].to_string();
    Some(ctos::PlayerInfo { name: name.into() })
}
