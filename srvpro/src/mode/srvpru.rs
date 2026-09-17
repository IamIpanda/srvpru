use linkme::distributed_slice;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::Normal;
use crate::mode::ProviderType;
use crate::mode::RoomConfiguration;
use crate::mode::slice_with_prefix;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[handler(Normal)]
#[register_to(MODES)]
fn engine(config: &mut RoomConfiguration, part: &str) -> bool {
    if let Some(engine) = slice_with_prefix::<u8>(part, "ENGINE").or(slice_with_prefix(part, "E")) {
        match engine {
            1 => config.provider = ProviderType::Embedded,
            2 => config.provider = ProviderType::Remote,
            _ => ()
        };
        engine <= 2
    } else { false }
}

#[handler(Normal)]
#[register_to(MODES)]
fn no_mask(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "NM" || part == "NOMASK" {
        config.ygopru_configuration.no_mask = true;
        true
    } else { false }
}

#[handler(Normal)]
#[register_to(MODES)]
fn bo(config: &mut RoomConfiguration, part: &str) -> bool {
    if let Some(bo) = slice_with_prefix::<u8>(part, "BO") {
        config.ygopru_configuration.enable_plugin_with_configuration(
            ygopro::plugin::bo::NAME,
            ygopro::plugin::bo::Configuration { override_best_of: bo }
        );
        true
    } else { false }
}
