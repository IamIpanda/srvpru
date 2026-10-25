use ygopro_data::constants::MasterRule;
use ygopro_data::constants::Mode;
use ygopro_data::constants::Rule;
use ygopro_derive::handler;
use ygopro_derive::register_to;
use ygopro_handler::All;

use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::RoomConfiguration;
use crate::plugin::register_dependencies;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_dependencies!(crate::plugin::base::stage::NAME);

pub(crate) use no_watch::NoWatch;

pub mod genesys {
    use ygopro::duel::Duel;
    use ygopro::managers::deck_manager;
    use ygopro::plugin::preload_script;
    use ygopro::ygopro_handlers::HandlerEx;
    use ygopro::ygopro_handlers::YGOPRO_HANDLERS_EX;
    use ygopro_data::constants::Type;
    use ygopro_data::data::DeckError;
    use ygopro_data::data::DeckErrorType;
    use ygopro_derive::before;
    use ygopro_derive::handler;
    use ygopro_derive::register_to;

    use crate::mode::MODES;
    use crate::mode::ModeHandler as Handler;
    use crate::mode::Normal;
    use crate::mode::RoomConfiguration;

    pub static NAME: &'static str = module_path!();

    const SPECIAL_SCRIPT: &str = "./script/special_genesys.lua";

    #[handler(Normal, module = "srvpro::mode::common")]
    #[register_to(MODES)]
    fn genesys(config: &mut RoomConfiguration, part: &str) -> bool {
        if part != "G" && part != "GENESYS" { return false }
        let deck_manager = deck_manager::load();
        if let Some(lflist) = deck_manager.lflists.iter().find(|lflist| lflist.name.contains("genesys")) {
            config.hostinfo.lflist = lflist.hash;
        }
        config.ygopru_configuration.enable_plugin(NAME);
        config.ygopru_configuration.enable_plugin_with_inited::<preload_script::Configuration>(preload_script::NAME, |configuration| {
            let scripts = &mut configuration.into_mut().preloaded_scripts;
            if !scripts.iter().any(|script| script == SPECIAL_SCRIPT) { scripts.push(SPECIAL_SCRIPT.to_string()); }
        });
        true
    }

    #[before(ygopro::message::ValidateDeck, priority = 10)]
    #[register_to(YGOPRO_HANDLERS_EX as HandlerEx)]
    fn forbid_pendulum_and_link(duel: &mut Duel, validate_deck: &ygopro::message::ValidateDeck) {
        if duel.get(validate_deck.player).is_none_or(|player| player.deck_error.is_some()) { return }
        let data_manager = ygopro::managers::data_manager::load_full();
        let deck = &validate_deck.deck;
        if let Some(player) = duel.get_mut(validate_deck.player) {
            player.deck_error = deck.iter()
                .filter_map(|&code| data_manager.get_card(code))
                .find(|card| card.card_type.intersects(Type::Pendulum | Type::Link))
                .map(|card| DeckError::new().with_error_type(DeckErrorType::NotAvailable).with_code(card.code));
        }
    }
}

#[handler(All, module = "srvpro::mode::common")]
#[register_to(MODES)]
fn legacy(config: &mut RoomConfiguration, name: &str) -> bool {
    let Some((head, numbers)) = name.split_at_checked(5) else { return false };
    let mut head = head.chars();
    let (Some(rule), Some(mode), Some(duel_rule), Some(no_check_deck), Some(no_shuffle_deck)) = (head.next(), head.next(), head.next(), head.next(), head.next()) else { return false };
    let Some(rule) = rule.to_digit(10).and_then(|value| Rule::try_from(value as u8).ok()) else { return false };
    let Some(mode) = mode.to_digit(10).and_then(|value| Mode::try_from(value as u8).ok()) else { return false };
    let duel_rule = match duel_rule { 'T' => Some(3), 'F' => Some(5), other => other.to_digit(10) };
    let Some(duel_rule) = duel_rule.and_then(|value| MasterRule::try_from(value as u8).ok()) else { return false };
    if !matches!(no_check_deck, 'T' | 'F') || !matches!(no_shuffle_deck, 'T' | 'F') { return false }
    let Some((start_lp, rest)) = numbers.split_once(',') else { return false };
    let Some((start_hand, draw_count)) = rest.split_once(',') else { return false };
    let (Ok(start_lp), Ok(start_hand), Ok(draw_count)) = (start_lp.parse::<u32>(), start_hand.parse::<u8>(), draw_count.parse::<u8>()) else { return false };
    config.hostinfo.rule = rule;
    config.hostinfo.mode = mode;
    config.hostinfo.duel_rule = duel_rule;
    config.hostinfo.no_check_deck = no_check_deck == 'T';
    config.hostinfo.no_shuffle_deck = no_shuffle_deck == 'T';
    config.hostinfo.start_lp = start_lp;
    config.hostinfo.start_hand = start_hand;
    config.hostinfo.draw_count = draw_count;
    true
}

mod no_watch {
    use std::net::SocketAddr;

    use ygopro_data::complex::Complex;
    use ygopro_data::constants::Color;
    use ygopro_data::constants::DuelStage;
    use ygopro_data::constants::Mode;
    use ygopro_data::constants::Netplayer;
    use ygopro_data::message::ctos;
    use ygopro_derive::before;
    use ygopro_derive::handler;
    use ygopro_derive::register_to;
    use ygopro_handler::Bundle;
    use ygopro_handler::FromRequest;
    use ygopro_handler::StopFlag;
    use ygopro_handler::extract;
    use ygopro_handler::extract::Response;

    use crate::mode::MODES;
    use crate::mode::ModeHandler as Handler;
    use crate::mode::Normal;
    use crate::mode::RoomConfiguration;
    use crate::mode::RoomProviderConfiguration;
    use crate::room::Anymap;
    use crate::room::CTOS_HANDLERS;
    use crate::room::ClientToServerHandler;
    use crate::room::Room;
    use crate::room::State;

    pub struct NoWatch;

    impl<Res> FromRequest<extract::Request<Complex<ctos::Message>, usize>, State, Res> for NoWatch where Res: Send {
        fn from_request(bundle: &mut Bundle<extract::Request<Complex<ctos::Message>, usize>, State, Res>) -> Option<Self> {
            bundle.state.states.get::<NoWatch>().map(|_| NoWatch)
        }
    }

    #[handler(Normal, module = "srvpro::mode::common")]
    #[register_to(MODES)]
    fn no_watch(config: &mut RoomConfiguration, part: &str) -> bool {
        if part != "NW" && part != "NOWATCH" { return false }
        config.states.insert(NoWatch);
        true
    }

    #[before(ctos::JoinGame, priority = 1, module = "srvpro::mode::common")]
    #[register_to(CTOS_HANDLERS as ClientToServerHandler)]
    fn refuse_watcher_join(room: &mut Room, states: &mut Anymap, index: usize, stop: &mut StopFlag, _: NoWatch) -> Response<ctos::Message> {
        if room.players.get(index).is_some_and(|player| player.states.get::<Netplayer>().is_some()) { return Response::Continue }
        let stage = states.get::<crate::plugin::base::stage::Stage>().map(|stage| stage.stage).unwrap_or(DuelStage::Begin);
        let maximum_players = states.get::<RoomProviderConfiguration>().map(|configuration| if configuration.hostinfo.mode == Mode::Tag { 4 } else { 2 }).unwrap_or(2);
        let connected_players = room.players.iter().filter(|(_, player)| player.states.get::<SocketAddr>().is_some()).count();
        if stage == DuelStage::Begin && connected_players <= maximum_players { return Response::Continue }
        if let Some(player) = room.players.get(index) {
            player.send_message("该房间禁止观战。", Color::Red);
        }
        stop.0 = true;
        Response::Kick
    }

    #[before(ctos::HsToObserver, priority = 1, module = "srvpro::mode::common")]
    #[register_to(CTOS_HANDLERS as ClientToServerHandler)]
    fn refuse_to_observer(room: &mut Room, index: usize, stop: &mut StopFlag, _: NoWatch) -> Response<ctos::Message> {
        if let Some(player) = room.players.get(index) {
            player.send_message("该房间禁止观战。", Color::Red);
        }
        stop.0 = true;
        Response::Swallow
    }
}
