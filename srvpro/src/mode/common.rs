use linkme::distributed_slice;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::Normal;
use crate::mode::RoomConfiguration;
use crate::plugin::register_dependencies;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_dependencies!(crate::plugin::base::stage::NAME);

pub(crate) use no_watch::NoWatch;

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
