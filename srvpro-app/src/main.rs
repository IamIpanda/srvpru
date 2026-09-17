// rustc will discard the whole api cargo without that.
// check https://github.com/dtolnay/linkme/issues/31
#[cfg(feature = "api")]
extern crate srvpro_api;

#[tokio::main]
async fn main() {
    env_logger::init();
    srvpro::process(srvpro::message::Init.into_message()).await;
    tokio::signal::ctrl_c().await.ok();
}
