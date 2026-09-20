//! Read and change the words filtered from room and player names.

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

register_router!(BAD_WORDS_ROUTER, build_bad_words_router);

const PERMISSION: &str = "change_settings";
const FILE_NAME: &str = "bad_words";
const FIELD: &str = "groups";
const KEY: &str = "bad_words_groups";

#[derive(Deserialize)]
struct BadWordsRequest {
    groups: serde_json::Value,
}

#[derive(Serialize)]
struct BadWordsResult {
    message: String,
}

fn build_bad_words_router() -> Router {
    auth::required(PERMISSION, Router::new().route("/api/bad_words", post(change_bad_words)))
}

async fn change_bad_words(Json(request): Json<BadWordsRequest>) -> Result<Json<BadWordsResult>, StatusCode> {
    if let Err(error) = store(&request.groups) {
        log::error!("cannot write {FILE_NAME}.yaml: {error}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    config_manager::update(vec![(KEY.to_string(), request.groups.to_string())]);
    configuration::rebuild_and_notify(&[srvpro::plugin::bad_words::NAME]).await;
    Ok(Json(BadWordsResult { message: "bad words ok".to_string() }))
}

fn store(groups: &serde_json::Value) -> std::io::Result<()> {
    let path = config::directory().join(format!("{FILE_NAME}.yaml"));
    let mut mapping = match fs::read_to_string(&path).ok().and_then(|content| serde_yaml::from_str::<serde_yaml::Value>(&content).ok()) {
        Some(serde_yaml::Value::Mapping(mapping)) => mapping,
        _ => serde_yaml::Mapping::new(),
    };
    mapping.insert(serde_yaml::Value::String(FIELD.to_string()), serde_yaml::to_value(groups).map_err(std::io::Error::other)?);
    let content = serde_yaml::to_string(&serde_yaml::Value::Mapping(mapping)).map_err(std::io::Error::other)?;
    fs::write(&path, content)
}
