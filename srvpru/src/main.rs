#[macro_use] extern crate anyhow;
#[macro_use] extern crate log;
#[macro_use] extern crate srvpru_proc_macros;

pub mod srvpro;

use srvpro::*;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    set_processors();
    serve().await;
}
