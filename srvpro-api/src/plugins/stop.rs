//! Turn the server on and off by toggling the stop plugin.

use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use serde::Deserialize;
use serde::Serialize;

use crate::auth;
use crate::register_router;

/// Kept separate from `srvpro::plugin::stop`: the panel has to answer while the server is running,
/// and that plugin is off in exactly that state.
#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_router!(STOP_ROUTER, build_stop_router);

const PERMISSION: &str = "stop";
const MESSAGE_KEY: &str = "stop_message";

#[derive(Deserialize)]
struct StopQuery {
    stop: String,
}

#[derive(Serialize)]
struct StopResult {
    message: String,
    payload: String,
}

#[derive(Serialize)]
struct StopState {
    stopped: bool,
}

fn build_stop_router() -> Router {
    auth::required(PERMISSION, Router::new()
        .route("/api/stop", get(stop))
        .route("/api/getstop", get(get_stop)))
}

async fn get_stop() -> Json<StopState> {
    Json(StopState { stopped: srvpro::configuration::get().enable_plugins.contains(srvpro::plugin::stop::NAME) })
}

async fn stop(Query(query): Query<StopQuery>) -> Result<Json<StopResult>, StatusCode> {
    if query.stop == "false" {
        srvpro::configuration::disable(srvpro::plugin::stop::NAME).await;
        crate::config::set_plugin_enabled(srvpro::plugin::stop::NAME, false);
        Ok(Json(StopResult { message: "start ok".to_string(), payload: query.stop }))
    } else {
        srvpro::configuration::enable_with_configuration(srvpro::plugin::stop::NAME, srvpro::plugin::stop::Configuration { message: query.stop.clone() }).await;
        if let Err(error) = crate::config::set(MESSAGE_KEY, &query.stop) { log::error!("cannot write config table: {error}"); }
        crate::config::set_plugin_enabled(srvpro::plugin::stop::NAME, true);
        Ok(Json(StopResult { message: "stop ok".to_string(), payload: query.stop }))
    }
}
