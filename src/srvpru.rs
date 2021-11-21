#[macro_use] mod room;
#[macro_use] mod player;
#[macro_use] mod utils;
mod processor;
mod server;

#[macro_use] pub mod plugins;
#[macro_use] pub mod message;
pub mod i18n;
pub mod config;

pub use processor::*;
pub use utils::*;
pub use room::*;
pub use player::*;
pub use server::*;

fn default_ygopro_address() -> String { "127.0.0.1".to_string() }
fn default_ygopro_binary() -> String { "./ygopro".to_string() }
fn default_ygopro_cwd() -> String{ "./ygopro".to_string() }
fn default_ygopro_lflist_conf() -> String { "./ygopro/lflist.conf".to_string() }
fn default_host_info() -> crate::ygopro::message::HostInfo { serde_json::from_str("{}").unwrap() }

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct YgoproConfiguration {
    #[serde(default = "default_ygopro_cwd")]
    cwd: String,
    #[serde(default = "default_ygopro_binary")]
    binary: String,
    #[serde(default = "default_ygopro_lflist_conf")]
    pub lflist_conf: String,
    #[serde(default = "default_ygopro_address")]
    address: String,
    #[serde(default)]
    wait_start: u64,
    #[serde(default = "default_host_info")]
    pub host_info: crate::ygopro::message::HostInfo
}

fn default_port() -> u16 { 7911 }
fn default_timeout() -> u64 { 90 }

set_configuration! {
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_timeout")]
    timeout: u64,
    ygopro: YgoproConfiguration,
    plugins: Vec<String>
}