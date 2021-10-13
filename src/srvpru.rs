#[macro_use]
pub mod room;
pub mod processor;
pub mod structs;
pub mod server;
#[macro_use]
pub mod player;
#[macro_use]
pub mod plugins;
pub mod utils;
pub mod i18n;
pub mod config;

pub use processor::*;
pub use utils::*;
pub use room::Room;
pub use server::Server;
pub use player::Player;

fn default_ygopro_address() -> String { "127.0.0.1".to_string() }

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct YgoproConfiguration {
    binary: String,
    #[serde(default = "default_ygopro_address")]
    address: String,
    #[serde(default)]
    wait_start: u64,
}

fn default_port() -> u16 { 7911 }
fn default_timeout() -> u64 { 30 }

set_configuration! {
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_timeout")]
    timeout: u64,
    ygopro: YgoproConfiguration,
    plugins: Vec<String>
}