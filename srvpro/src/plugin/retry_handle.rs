//! Handle MSG_RETRY from the core: count retries and kick players who retry
//! too often. The retry itself is forwarded to the client as-is.

use ygopro_data::constants::Color;
use ygopro_data::message::gm;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;
use ygopro_handler::extract::Response;

use crate::room::GameMessageHandler;
use crate::room::GM_HANDLERS;
use crate::room::Player;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "5")]
    pub max_retry_count: u8,
}

#[derive(Default)]
struct RetryState {
    count: u8,
}

#[handler(gm::Retry)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_retry(player: &mut Player, _message: &gm::Retry, configuration: Configuration) -> Response<gm::Message> {
    let state = player.states.entry::<RetryState>().or_default();
    state.count += 1;
    let count = state.count;
    if configuration.max_retry_count > 0 && count >= configuration.max_retry_count {
        player.send_message(&format!("重试次数过多（{}），已被踢出", configuration.max_retry_count), Color::Red);
        return Response::Kick;
    }
    player.send_message(&format!("重试 {} 次", count), Color::Red);
    Response::Continue
}
