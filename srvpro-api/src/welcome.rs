//! Read and change the welcome message shown to every joining player.

use std::fs;

use axum::Json;
use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
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

register_router!(WELCOME_ROUTER, build_welcome_router);

const PERMISSION: &str = "change_settings";
const FILE_NAME: &str = "welcome";
const FIELD: &str = "message";
const KEY: &str = "welcome_message";

#[derive(Deserialize)]
struct WelcomeRequest {
    welcome: String,
}

#[derive(Serialize)]
struct WelcomeResult {
    message: String,
    payload: String,
}

fn build_welcome_router() -> Router {
    auth::required(PERMISSION, Router::new().route("/api/welcome", get(get_welcome).post(change_welcome)))
}

async fn get_welcome() -> Json<WelcomeResult> {
    let welcome = config_manager::load().get(KEY).unwrap_or_default().to_string();
    Json(WelcomeResult { message: "get ok".to_string(), payload: welcome })
}

async fn change_welcome(Json(request): Json<WelcomeRequest>) -> Result<Json<WelcomeResult>, StatusCode> {
    if let Err(error) = store(&request.welcome) {
        log::error!("cannot write {FILE_NAME}.yaml: {error}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    config_manager::update(vec![(KEY.to_string(), request.welcome.clone())]);
    configuration::rebuild_and_notify(&[srvpro::plugin::welcome::NAME]).await;
    Ok(Json(WelcomeResult { message: "welcome ok".to_string(), payload: request.welcome }))
}

fn store(value: &str) -> std::io::Result<()> {
    let path = config::directory().join(format!("{FILE_NAME}.yaml"));
    let mut mapping = match fs::read_to_string(&path).ok().and_then(|content| serde_yaml::from_str::<serde_yaml::Value>(&content).ok()) {
        Some(serde_yaml::Value::Mapping(mapping)) => mapping,
        _ => serde_yaml::Mapping::new(),
    };
    mapping.insert(serde_yaml::Value::String(FIELD.to_string()), serde_yaml::Value::String(value.to_string()));
    let content = serde_yaml::to_string(&serde_yaml::Value::Mapping(mapping)).map_err(std::io::Error::other)?;
    fs::write(&path, content)
}
