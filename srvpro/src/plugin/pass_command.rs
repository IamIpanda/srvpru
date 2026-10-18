use std::mem::ManuallyDrop;

use ygopro_data::complex::Complex;
use ygopro_data::constants::ErrorMessage;
use ygopro_data::constants::JoinError;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_handler::Bundle;
use ygopro_handler::Call;
use ygopro_handler::FromRequest;
use ygopro_handler::TowerHandler;
use ygopro_handler::extract;

use crate::configuration;
use crate::room::CTOS_PREHANDLERS;
use crate::room::ClientToServerPrecursorHandler;
use crate::room::Player;

pub type PassCommandHandler = TowerHandler<PassCommandArguments, ManuallyDrop<Player>, extract::Response<()>>;

pub struct PassCommandArguments {
    pub pass: String,
}

impl<Res> FromRequest<PassCommandArguments, ManuallyDrop<Player>, Res> for &mut PassCommandArguments where Res: Send {
    fn from_request(bundle: &mut Bundle<PassCommandArguments, ManuallyDrop<Player>, Res>) -> Option<Self> {
        Some(unsafe { &mut *(&mut bundle.request as *mut PassCommandArguments) })
    }
}

#[distributed_slice]
pub static PASS_COMMANDS: [fn() -> (&'static str, PassCommandHandler)];

fn command_handler(command_name: &str) -> Option<PassCommandHandler> {
    let configuration = configuration::get();
    PASS_COMMANDS.iter().map(|build| build())
        .find(|(name, handler)| name.eq_ignore_ascii_case(command_name) && configuration.enable_plugins.contains(handler.module_name))
        .map(|(_, handler)| handler)
}

#[before(ctos::JoinGame, priority = 2, module = "srvpro::core")]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
async fn before_join_game(bundle: &mut Bundle<Complex<ctos::Message>, Player, extract::Response<ctos::Message>>) -> &'static str {
    let pass: &str = match &*bundle.request {
        ctos::Message::JoinGame(join_game) => join_game.pass.as_ref(),
        _ => return "continue",
    };
    let Some(handler) = command_handler(pass.trim()) else { return "continue" };
    let request = PassCommandArguments { pass: pass.to_string() };
    bundle.stop_flag.0 = true;
    let player = ManuallyDrop::new(unsafe { std::ptr::read(&bundle.state) });
    let inner_bundle = Bundle::new(request, player, Default::default());
    let mut result_bundle = handler.call(inner_bundle).await;
    let player = unsafe { ManuallyDrop::take(&mut result_bundle.state) };
    unsafe { std::ptr::write(&mut bundle.state, player) };
    bundle.state.server_to_client_sink_towards_network.unbounded_send(Complex::from_message(stoc::ErrorMessage {
        err: ErrorMessage::JoinError(JoinError::HostRefused),
    }.into())).ok();
    "kick"
}
