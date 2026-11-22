//! Record whether a player is the host and its
//! [`Netplayer`](ygopro_data::constants::Netplayer) from the type change message.

use std::mem::ManuallyDrop;

use ygopro_data::complex::Complex;
use ygopro_data::message::stoc;
use ygopro_handler::Bundle;
use ygopro_handler::FromRequest;
use ygopro_handler::extract::Request;

use crate::room::Player;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;
use crate::room::State;

/// Name for activitating this module in the plugin system.
pub static NAME: &'static str = module_path!();

#[handler(stoc::TypeChange)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_type_change(player: &mut Player, type_change: &stoc::TypeChange) {
    player.states.insert(type_change.host);
    player.states.insert(type_change.player);
}

pub struct Host;

impl<Res, Message> FromRequest<Request<Complex<Message>, usize>, State, Res> for Host where Message: Send, Res: Send {
    fn from_request(bundle: &mut Bundle<Request<Complex<Message>, usize>, State, Res>) -> Option<Self> {
        let player = bundle.state.room.players.get(bundle.request.extra)?;
        (player.states.get::<bool>() == Some(&true)).then_some(Host)
    }
}

impl<Res, Message> FromRequest<Request<Complex<Message>, usize>, ManuallyDrop<State>, Res> for Host where Message: Send, Res: Send {
    fn from_request(bundle: &mut Bundle<Request<Complex<Message>, usize>, ManuallyDrop<State>, Res>) -> Option<Self> {
        let player = bundle.state.room.players.get(bundle.request.extra)?;
        (player.states.get::<bool>() == Some(&true)).then_some(Host)
    }
}
