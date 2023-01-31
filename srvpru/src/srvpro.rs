mod context;
mod processor;
mod player;
mod room;
mod handler;
mod handler_extension;
mod attachment;
mod configuration;
mod server;
mod api;
mod plugin;
pub mod message;

pub use context::*;
pub use processor::*;
pub use player::*;
pub use room::*;
pub use handler::*;
pub use handler_extension::*;
pub use attachment::*;
pub use configuration::*;
pub use server::*;
pub use api::*;

#[derive(Debug, Configuration, serde::Deserialize)]
#[configuration(filename = "ygopro")]
#[serde(default)]
pub struct YgoproConfiguration {
    /// Working directory if ygopro server.
    pub cwd: String,
    /// Ygopro server path.
    binary: String,
    /// `lflist.conf` path.
    pub lflist_conf: String,
    /// After ygopro server start, which address it's listening for.
    address: String,
    /// After ygopro server start, sleep for that milliseconds. \
    /// Set to `0` to disable.
    /// 
    /// Need to set to some value if start ygopro server by docker or K8s.
    wait_start: u64,
    /// Default host info.
    pub host_info: ygopro::message::HostInfo
}

impl Default for YgoproConfiguration {
    fn default() -> Self {
        Self { 
            cwd        : "./ygopro2".to_string(), 
            binary     : "./ygopro".to_string(), 
            lflist_conf: "./ygopro/lflist.conf".to_string(), 
            address    : "127.0.0.1".to_string(), 
            wait_start : 1, 
            host_info  : Default::default() 
        }
    }
}

#[derive(Debug, Configuration, serde::Deserialize)]
#[configuration(filename = "srvpru")]
#[serde(default)]
pub struct SrvpruConfiguration {
    pub host: String,
    pub port: u16,
    pub plugins: Vec<String>
}

impl Default for SrvpruConfiguration {
    fn default() -> Self {
        Self { 
            host: "0.0.0.0".to_string(),
            port: 7911, 
            plugins: vec!() 
        }
    }
}
