#[macro_use] extern crate downcast_rs;
#[macro_use] extern crate erased_serde;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate thiserror;
#[macro_use] extern crate bitflags;
#[macro_use] extern crate anyhow;
#[macro_use] extern crate scanner;
#[macro_use] extern crate log;
extern crate pretty_env_logger;

#[macro_use] pub mod ygopro;
#[macro_use] pub mod srvpru;
pub mod srvpro;

use crate::srvpru::Player;
use crate::srvpru::Room;
use crate::srvpru::i18n;
use crate::srvpru::Server;
use crate::srvpru::get_server;

#[tokio::main]
async fn main() {
    init().await;
    register();
    start().await;
}

async fn init() {
    pretty_env_logger::init();
    srvpro::generate_srvpru_configuration().await;
    crate::srvpru::load_configuration().expect("Failed to load srvpru configuration.");
}

fn register() {
    i18n::init().expect("Failed to load i18n");
    Player::init().expect("Failed to load base plugin: player");
    Room::init().expect("Failed to load plugin: room");
    crate::srvpru::plugins::init().expect("Init plugins failed");
    Server::init().expect("Failed to init socket server");
}

async fn start() {
    get_server().start().await.expect("Failed to start socket server");
    error!("Terminated server. Srvpru is going to down.");
}
