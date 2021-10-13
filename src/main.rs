#[macro_use] extern crate downcast_rs;
#[macro_use] extern crate erased_serde;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate thiserror;
#[macro_use] extern crate anyhow;
#[macro_use] extern crate scanner;
#[macro_use] extern crate log;
extern crate pretty_env_logger;

mod ygopro;
mod srvpru;

use crate::srvpru::Player;
use crate::srvpru::Room;
use crate::srvpru::i18n;
use crate::srvpru::server::Server;
use crate::srvpru::server::SOCKET_SERVER;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init()?;
    register();
    setup();
    start().await;
    Ok(())
}

fn init() -> anyhow::Result<()> {
    pretty_env_logger::init();
    i18n::load_configuration()?;
    crate::srvpru::load_configuration().expect("Failed to load srvpru configuration.");
    Ok(())
}

fn register() {
    Player::register_handlers();
    Room::register_handlers();
    init_plugin_under_dir!("src/srvpru/plugins", process_plugin_result("#name", #name::init()));
}

fn setup() {
    let mut server = Server::new();
    let configuration = crate::srvpru::get_configuration();
    let plugins: Vec<&str> = configuration.plugins.iter().map(String::as_ref).collect();
    server.register_handlers(&plugins, &[], &[], &[]);
    info!("{:}", &server);
    SOCKET_SERVER.set(server).ok().expect("Failed to set socket server");
}

async fn start() {
    SOCKET_SERVER.get().unwrap().start().await.expect("Failed to start socket server");
    error!("Terminated server. Srvpru is going to down.");
}

fn process_plugin_result(name: &str, result: anyhow::Result<()>) {
    match result {
        Ok(_) => info!("Loaded plugin {}", name),
        Err(err) => error!("Load plugin {} failed: {}", name, err)
    };
}