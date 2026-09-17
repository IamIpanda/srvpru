//! Turn the server on and off by toggling the stop plugin.

use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use linkme::distributed_slice;
use serde::Deserialize;
use serde::Serialize;

use crate::auth;
use crate::register_router;

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_router!(STOP_ROUTER, build_stop_router);

const PERMISSION: &str = "stop";

#[derive(Deserialize)]
struct StopQuery {
    stop: String,
}

#[derive(Serialize)]
struct StopResult {
    message: String,
    payload: String,
}

fn build_stop_router() -> Router {
    auth::required(PERMISSION, Router::new().route("/api/stop", get(stop)))
}

async fn stop(Query(query): Query<StopQuery>) -> Result<Json<StopResult>, StatusCode> {
    if query.stop == "false" {
        srvpro::configuration::disable(srvpro::plugin::stop::NAME).await;
        return Ok(Json(StopResult { message: "start ok".to_string(), payload: query.stop }));
    }
    srvpro::configuration::enable_with_configuration(srvpro::plugin::stop::NAME, srvpro::plugin::stop::Configuration { message: query.stop.clone() }).await;
    Ok(Json(StopResult { message: "stop ok".to_string(), payload: query.stop }))
}
