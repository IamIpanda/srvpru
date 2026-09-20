use linkme::distributed_slice;

use ygopro_data::constants::Color;
use ygopro_data::message::ctos;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::room::ClientToServerHandler;
use crate::room::CTOS_HANDLERS;
use crate::room::Player;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "String::from(\"欢迎来到srvpru服务器。\\n如遇Bug请联系作者，感谢。\")")]
    pub message: String,
}

#[handler(ctos::JoinGame)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn on_join_game(player: &mut Player) {
    let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return };
    for line in configuration.message.lines() {
        player.send_message(line, Color::Green);
    }
}
