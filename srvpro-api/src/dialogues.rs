//! Set the dialogues shown when a card is summoned.

use std::fs;

use axum::Json;
use axum::Router;
use axum::http::StatusCode;
use axum::routing::post;
use linkme::distributed_slice;
use serde::Deserialize;
use serde::Serialize;

use crate::auth;
use crate::config;
use crate::register_router;
use srvpro::configuration;
use srvpro::managers::config_manager;

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_router!(DIALOGUES_ROUTER, build_dialogues_router);

const PERMISSION: &str = "change_settings";
const FILE_NAME: &str = "dialogues";
const FIELD: &str = "dialogues";
const KEY: &str = "dialogues_dialogues";

#[derive(Deserialize)]
struct DialoguesRequest {
    dialogues: serde_json::Value,
}

#[derive(Serialize)]
struct DialoguesResult {
    message: String,
    payload: serde_json::Value,
}

fn build_dialogues_router() -> Router {
    auth::required(PERMISSION, Router::new().route("/api/dialogues", post(change_dialogues)))
}

async fn change_dialogues(Json(request): Json<DialoguesRequest>) -> Result<Json<DialoguesResult>, StatusCode> {
    if let Err(error) = store(&request.dialogues) {
        log::error!("cannot write {FILE_NAME}.json: {error}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    config_manager::update(vec![(KEY.to_string(), request.dialogues.to_string())]);
    configuration::rebuild_and_notify(&[srvpro::plugin::dialogues::NAME]).await;
    Ok(Json(DialoguesResult { message: "dialogue ok".to_string(), payload: request.dialogues }))
}

fn store(dialogues: &serde_json::Value) -> std::io::Result<()> {
    let path = config::directory().join(format!("{FILE_NAME}.json"));
    let mut object = match fs::read_to_string(&path).ok().and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok()) {
        Some(serde_json::Value::Object(object)) => object,
        _ => serde_json::Map::new(),
    };
    object.insert(FIELD.to_string(), dialogues.clone());
    let content = serde_json::to_string_pretty(&serde_json::Value::Object(object)).map_err(std::io::Error::other)?;
    fs::write(&path, content)
}
