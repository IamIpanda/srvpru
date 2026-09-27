use std::net::IpAddr;
use std::net::SocketAddr;
use std::sync::LazyLock;
use std::time::Duration;

use hashbrown::HashMap;
use futures::channel::mpsc;
use linkme::distributed_slice;
use parking_lot::Mutex;
use ygopro_data::complex::Complex;
use ygopro_data::constants::Color;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::configuration;
use crate::GlobalHandler;
use crate::message as srvpro;
use crate::room::SrvproMessageHandler;
use crate::room::SRVPRO_HANDLERS;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "5")]
    pub max_connections_per_ip: usize,
    #[config(default = "5")]
    pub join_timeout: u64,
}

static IP_CONNECTIONS: LazyLock<Mutex<HashMap<IpAddr, usize>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

struct ConnectionGuard(SocketAddr);

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let ip = self.0.ip();
        let mut connections = IP_CONNECTIONS.lock();
        let remaining = connections.get_mut(&ip).map(|count| { *count = count.saturating_sub(1); *count }).unwrap_or_default();
        if remaining == 0 { connections.remove(&ip); }
    }
}

#[handler(srvpro::ClientConnect)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn limit_connections_per_ip(client_connect: &mut srvpro::ClientConnect) -> &'static str {
    let player = &mut client_connect.player;
    let Some(configuration) = configuration::get_configuration::<Configuration>() else { return "continue" };
    let Some(address) = player.states.get::<SocketAddr>().copied() else { return "continue" };
    let connections = {
        let mut connections = IP_CONNECTIONS.lock();
        let count = connections.entry(address.ip()).or_insert(0);
        *count += 1;
        *count
    };
    player.states.insert(ConnectionGuard(address));
    if configuration.max_connections_per_ip == 0 || connections <= configuration.max_connections_per_ip { return "continue" }
    player.send_message("同一 IP 连接数过多。", Color::Red);
    "kick"
}

struct JoinTimeoutTask(tokio::task::JoinHandle<()>);

#[handler(srvpro::ClientConnect)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn set_join_timeout(client_connect: &mut srvpro::ClientConnect) {
    let player = &mut client_connect.player;
    let Some(configuration) = configuration::get_configuration::<Configuration>() else { return };
    if configuration.join_timeout == 0 { return }
    let Some(client_to_server_sink_towards_preprocessor) = player.states.get::<mpsc::UnboundedSender<Complex<ctos::Message>>>().cloned() else { return };
    let server_to_client_sink_towards_network = player.server_to_client_sink_towards_network.clone();
    let timeout = Duration::from_secs(configuration.join_timeout);
    let handle = tokio::spawn(async move {
        tokio::time::sleep(timeout).await;
        server_to_client_sink_towards_network.unbounded_send(Complex::from_message(stoc::Chat { player: Color::Red.into(), msg: "[Server]: 加入房间超时。".into() }.into())).ok();
        client_to_server_sink_towards_preprocessor.unbounded_send(Complex::from_message(ctos::Message::LeaveGame(ctos::LeaveGame))).ok();
    });
    player.states.insert(JoinTimeoutTask(handle));
}

#[before(srvpro::PlayerJoin, priority = 120)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn reset_join_timeout(player_join: &mut srvpro::PlayerJoin) {
    let Some(player) = player_join.player.as_mut() else { return };
    if let Some(task) = player.states.remove::<JoinTimeoutTask>() { task.0.abort(); }
}
