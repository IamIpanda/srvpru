use ygopro_data::message::ctos;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::room::Player;
use crate::room::CTOS_HANDLERS;
use crate::room::ClientToServerHandler;

pub static NAME: &'static str = module_path!();

#[handler(ctos::ExternalAddress)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn on_external_address(player: &mut Player, external_address: &ctos::ExternalAddress) {
    if external_address.real_ip.is_unspecified() { return }
    player.states.insert(external_address.real_ip);
}

pub fn get_ip(player: &Player) -> std::net::Ipv4Addr {
    player.states.get::<std::net::Ipv4Addr>().copied()
        .or(player.states.get::<std::net::SocketAddr>().and_then(|socket_addr| match socket_addr.ip() {
            std::net::IpAddr::V4(ip) => Some(ip),
            std::net::IpAddr::V6(_) => None,
        }))
        .unwrap_or("127.0.0.1".parse().unwrap())
}
