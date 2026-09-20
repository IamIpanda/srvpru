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
use srvpro::room::Request;
use srvpro::room::ROOMS;

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_router!(DEATH_ROUTER, build_death_router);

const PERMISSION: &str = "start_death";

#[derive(Deserialize)]
struct DeathQuery {
    death: String,
}

#[derive(Serialize)]
struct DeathResult {
    message: String,
    payload: String,
}

fn build_death_router() -> Router {
    auth::required(PERMISSION, Router::new()
        .route("/api/death", get(start_death))
        .route("/api/deathcancel", get(cancel_death)))
}

async fn start_death(Query(query): Query<DeathQuery>) -> Result<Json<DeathResult>, StatusCode> {
    send_command(&query.death, "start_death");
    Ok(Json(DeathResult { message: "death ok".to_string(), payload: query.death }))
}

async fn cancel_death(Query(query): Query<DeathQuery>) -> Result<Json<DeathResult>, StatusCode> {
    send_command(&query.death, "cancel_death");
    Ok(Json(DeathResult { message: "death cancel ok".to_string(), payload: query.death }))
}

fn send_command(target: &str, command: &'static str) {
    for host in ROOMS.read().values() {
        if target == "all" || target == host.name {
            host.sender.unbounded_send(Request::Command(command, Box::new(()))).ok();
        }
    }
}
