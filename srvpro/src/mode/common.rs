use linkme::distributed_slice;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::mode::MODES;
use crate::mode::Normal;
use crate::mode::RoomConfiguration;
use crate::mode::ModeHandler as Handler;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[handler(Normal)]
#[register_to(MODES)]
fn genesys(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "G" || part == "GENESYS" {
        let deck_manager = ygopro::managers::deck_manager::load();
        if let Some(lflist) = deck_manager.lflists.iter().find(|lflist| lflist.name.starts_with("genesys")) {
            config.hostinfo.lflist = lflist.hash;
        }
        true
    } else { false }
}
