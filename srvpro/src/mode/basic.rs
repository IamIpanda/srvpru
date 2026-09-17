use linkme::distributed_slice;
use ygopro::duel::SendTarget;
use ygopro_data::constants::*;
use ygopro_derive::handler;
use ygopro_derive::register_to;
use ygopro_handler::All;

use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::Normal;
use crate::mode::RoomConfiguration;
use crate::mode::Tag;
use crate::mode::slice_with_prefix;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[handler(All, module="srvpro::core")]
#[register_to(MODES)]
fn basic(config: &mut RoomConfiguration) {
    config.ygopru_configuration.enable_plugin_with_configuration(
        ygopro::plugin::terminate::NAME,
        ygopro::plugin::terminate::Configuration {
            terminate_when: SendTarget::All,
            terminate_when_match_end: true
        }
    );
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn single(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "S" || part == "SINGLE" {
        config.hostinfo.mode = Mode::Single;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn _match(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "M" || part == "MATCH" {
        config.hostinfo.mode = Mode::Match;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn tag(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "T" || part == "TAG" {
        config.hostinfo.mode = Mode::Tag;
        config.hostinfo.start_lp = 16000;
        config.states.insert(Tag);
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn ocg(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "OCGONLY" || part == "OO" {
        config.hostinfo.rule = Rule::OCG;
        config.hostinfo.lflist = 0;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn ot(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "TCG" || part == "OT" {
        config.hostinfo.rule = Rule::All;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn tcg(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "TCGONLY" || part == "TO" {
        config.hostinfo.rule = Rule::TCG;
        let deck_manager = ygopro::managers::deck_manager::load();
        if let Some(lflist) = deck_manager.lflists.iter().find(|lflist| lflist.name.starts_with("TCG")) {
            config.hostinfo.lflist = lflist.hash;
        }
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn chinese(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "SC" || part == "CN" || part == "CCG" || part == "CHINESE" {
        config.hostinfo.rule = Rule::SC;
        config.hostinfo.lflist = 0;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn custom(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "CUSTOM" || part == "DIY" {
        config.hostinfo.rule = Rule::Custom;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn no_lflist(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "NF" || part == "NOLFLIST" {
        config.hostinfo.lflist = 0;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn no_unique(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "NU" || part == "NOUNIQUE" {
        config.hostinfo.rule = Rule::OCG_TCG;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn no_check(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "NC" || part == "NOCHECK" {
        config.hostinfo.no_check_deck = true;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn no_shuffle(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "NS" || part == "NOSHUFFLE" {
        config.hostinfo.no_shuffle_deck = true;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn lp(config: &mut RoomConfiguration, part: &str) -> bool {
    if let Some(lp) = slice_with_prefix(part, "LP") {
        config.hostinfo.start_lp = lp;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn time(config: &mut RoomConfiguration, part: &str) -> bool {
    if let Some(time) = slice_with_prefix(part, "TIME").or(slice_with_prefix(part, "TM")).or(slice_with_prefix(part, "TI")) {
        config.hostinfo.time_limit = time;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn start(config: &mut RoomConfiguration, part: &str) -> bool {
    if let Some(start) = slice_with_prefix(part, "START").or(slice_with_prefix(part, "ST")) {
        config.hostinfo.start_hand = start;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn draw(config: &mut RoomConfiguration, part: &str) -> bool {
    if let Some(draw) = slice_with_prefix(part, "DRAW").or(slice_with_prefix(part, "DR")) {
        config.hostinfo.draw_count = draw;
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn lflist(config: &mut RoomConfiguration, part: &str) -> bool {
    if let Some(lflist) = slice_with_prefix::<u32>(part, "LFLIST").or(slice_with_prefix(part, "LF")) {
        let deck_manager = ygopro::managers::deck_manager::load();
        if let Some(lflist) = deck_manager.get_lflist_by_index(lflist) {
            config.hostinfo.lflist = lflist.hash;
        }
        true
    } else { false }
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn duel_rule(config: &mut RoomConfiguration, part: &str) -> bool {
    let Some(rule) = slice_with_prefix::<u8>(part, "DUELRULE").or(slice_with_prefix(part, "MR")) else { return false };
    let Ok(rule) = MasterRule::try_from(rule) else { return false };
    config.hostinfo.duel_rule = rule;
    true
}

#[handler(Normal, module="srvpro::core")]
#[register_to(MODES)]
fn priority(config: &mut RoomConfiguration, part: &str) -> bool {
    if part == "IGPRIORITY" || part == "PR" {
        config.hostinfo.duel_rule = MasterRule::MasterRuleNew;
        true
    } else { false }
}
