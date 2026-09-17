use linkme::distributed_slice;
use rust_i18n::t;

use ygopro_derive::*;
use ygopro_data::constants::Color;
use ygopro_data::message::ctos;

use crate::room::ClientToServerHandler;
use crate::room::CTOS_HANDLERS;
use crate::room::Player;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[handler(ctos::JoinGame)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn on_join_game(player: &mut Player) {
    for line in t!("welcome").lines() {
        player.send_message(line, Color::Babyblue);
    }
}
