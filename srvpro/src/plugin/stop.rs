//! Refuse every join while this plugin is enabled.

use ygopro_data::complex::Complex;
use ygopro_data::constants::Color;
use ygopro_data::constants::ErrorMessage;
use ygopro_data::constants::JoinError;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Configuration;
use ygopro_derive::before;
use ygopro_derive::register_to;
use ygopro_handler::StopFlag;

use crate::room::CTOS_PREHANDLERS;
use crate::room::ClientToServerPrecursorHandler;
use crate::room::Player;

pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    pub message: String,
}

#[before(ctos::JoinGame)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
fn refuse_join(player: &mut Player, stop: &mut StopFlag) -> &'static str {
    let configuration = crate::configuration::get();
    let Some(configuration) = configuration.configurations.get::<Configuration>().cloned() else { return "continue" };
    player.server_to_client_sink_towards_network.unbounded_send(Complex::from_message(stoc::Chat {
        player: Color::Red.into(),
        msg: configuration.message.into(),
    }.into())).ok();
    player.server_to_client_sink_towards_network.unbounded_send(Complex::from_message(stoc::ErrorMessage {
        err: ErrorMessage::JoinError(JoinError::HostRefused),
    }.into())).ok();
    stop.0 = true;
    "kick"
}
