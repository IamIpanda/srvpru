// rustc will discard the whole api cargo without that.
// check https://github.com/dtolnay/linkme/issues/31
#[cfg(feature = "api")]
extern crate srvpro_api;

#[tokio::main]
async fn main() {
    env_logger::init();
    let response = srvpro::process(srvpro::message::Init.into_message()).await;
    if matches!(response, srvpro::Response::Terminate) {
        log::error!("A handler reported termination in Initialization event, so application dont starts.");
        std::process::exit(1);
    }
    tokio::signal::ctrl_c().await.ok();
}
