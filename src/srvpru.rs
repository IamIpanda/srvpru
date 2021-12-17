#[macro_use] mod room;
#[macro_use] mod player;
#[macro_use] mod utils;
mod processor;
mod server;

#[macro_use] pub mod plugins;
#[macro_use] pub mod message;
pub mod i18n;

pub use processor::*;
pub use server::*;
pub use room::*;
pub use player::*;
pub use utils::*;

#[doc(hidden)] fn default_ygopro_cwd() -> String{ "./ygopro".to_string() }
#[doc(hidden)] fn default_ygopro_address() -> String { "127.0.0.1".to_string() }
#[doc(hidden)] fn default_ygopro_binary() -> String { "./ygopro".to_string() }
#[doc(hidden)] fn default_ygopro_database() -> String { "./cards.cdb".to_string() }
#[doc(hidden)] fn default_ygopro_lflist_conf() -> String { "./ygopro/lflist.conf".to_string() }
#[doc(hidden)] fn default_host_info() -> crate::ygopro::message::HostInfo { crate::ygopro::message::HostInfo::default() }
#[doc(hidden)] fn default_plugins() -> Vec<String> { vec!["player".to_string(), "room".to_string()] }
#[doc(hidden)] fn default_empty_vec_owned() -> Vec<String> { Vec::new() }

/// Save ygopro server information.
#[derive(serde::Serialize, serde::Deserialize, Debug, serde_default)]
pub struct YgoproConfiguration {
    /// Working directory if ygopro server.
    #[serde(default = "default_ygopro_cwd")]
    pub cwd: String,
    /// Cards.cdb position. Don't needed if you don't 
    #[serde(default = "default_ygopro_database")]
    pub database: String,
    /// Ygopro server path.
    #[serde(default = "default_ygopro_binary")]
    binary: String,
    /// `lflist.conf` path.
    #[serde(default = "default_ygopro_lflist_conf")]
    pub lflist_conf: String,
    /// After ygopro server start, which address it's listening for.
    #[serde(default = "default_ygopro_address")]
    address: String,
    /// After ygopro server start, sleep for that milliseconds. \
    /// Set to `0` to disable.
    /// 
    /// Need to set to some value if start ygopro server by docker or K8s.
    #[serde(default)]
    wait_start: u64,
    /// Default host info.
    #[serde(default = "default_host_info")]
    pub host_info: crate::ygopro::message::HostInfo
}

fn default_port() -> u16 { 7911 }
fn default_timeout() -> u64 { 90 }

set_configuration! {
    /// Srvpru main port listening for.
    #[serde(default = "default_port")]
    port: u16,
    /// After that milliseconds without any message, socket will close.
    #[serde(default = "default_timeout")]
    timeout: u64,
    /// Ygopro server configuration.
    #[serde(default)]
    ygopro: YgoproConfiguration,
    /// Enabled plugins. \
    /// Srvpru will compile and load all plugins when start, and load plugins enabled in this configuration.
    #[serde(default = "default_plugins")]
    plugins: Vec<String>,
    /// Additional handlers on `stoc` after plugins loaded.
    #[serde(default = "default_empty_vec_owned")]
    stoc_handlers: Vec<String>,
    /// Additional handlers on `ctos` after plugins loaded.
    #[serde(default = "default_empty_vec_owned")]
    ctos_handlers: Vec<String>,
    /// Additional handlers on `srvpru` after plugins loaded.
    #[serde(default = "default_empty_vec_owned")]
    internal_handlers: Vec<String>
}

pub fn configuration_path() -> String {
    std::env::var("SRVPRU_CONFIG_PATH").unwrap_or("./srvpru_config".to_string())
}
