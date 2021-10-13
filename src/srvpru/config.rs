use serde::Serialize;
use serde::Deserialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerConfig {
    pub binary: String,
    pub address: String,
    pub wait_start: u32
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub port: u16,
    pub ygopro: ServerConfig,
    pub plugins: Vec<String>
}

