//! Push a chat line to every established room.

use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use linkme::distributed_slice;
use serde::Deserialize;
use serde::Serialize;
use ygopro_data::constants::Color;
use ygopro_data::message::stoc;

use crate::auth;
use crate::register_router;
use srvpro::message as srvpro_message;
use srvpro::room::Request;
use srvpro::room::ROOMS;

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_router!(SHOUT_ROUTER, build_shout_router);

const PERMISSION: &str = "shout";

#[derive(Deserialize)]
struct ShoutQuery {
    shout: String,
}

#[derive(Serialize)]
struct ShoutResult {
    message: String,
    payload: String,
}

fn build_shout_router() -> Router {
    auth::required(PERMISSION, Router::new().route("/api/shout", get(shout)))
}

async fn shout(Query(query): Query<ShoutQuery>) -> Result<Json<ShoutResult>, StatusCode> {
    let chat = stoc::Chat { player: Color::Yellow.into(), msg: query.shout.as_str().into() };
    for host in ROOMS.read().values() {
        host.sender.unbounded_send(Request::Ex(srvpro_message::DirectSTOC { message: chat.clone().into(), target: None }.into(), 0)).ok();
    }
    Ok(Json(ShoutResult { message: "shout ok".to_string(), payload: query.shout }))
}
