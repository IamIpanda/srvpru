use std::fs;

use axum::Json;
use axum::Router;
use axum::http::StatusCode;
use axum::routing::post;
use serde::Deserialize;
use serde::Serialize;

use crate::auth;
use crate::config;
use crate::portal::ConfigTab;
use crate::portal::ConfigTabContent;
use srvpro::configuration;
use srvpro::managers::config_manager;

const PERMISSION: &str = "change_settings";
const FILE_NAME: &str = "tip";
const FIELD: &str = "tips";
const KEY: &str = "tip_tips";

#[distributed_slice(crate::portal::PORTAL_CONFIG_TABS)]
pub static CONFIG: ConfigTab = ConfigTab {
    plugin: srvpro::plugin::tip::NAME,
    title: "小贴士",
    order: 21,
    toggle: true,
    content: ConfigTabContent::Component { export: "Tip" },
};

#[derive(Deserialize)]
struct TipRequest {
    tips: Vec<String>,
}

#[derive(Serialize)]
struct TipResult {
    message: String,
}

#[api(srvpro::plugin::tip::NAME)]
fn build_tip_router() -> Router {
    auth::required(PERMISSION, Router::new().route("/api/tip", post(change_tips)))
}

async fn change_tips(Json(request): Json<TipRequest>) -> Result<Json<TipResult>, StatusCode> {
    if let Err(error) = store(&request.tips) {
        log::error!("cannot write {FILE_NAME}.yaml: {error}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    config_manager::update(vec![(KEY.to_string(), serde_json::to_string(&request.tips).unwrap_or_else(|_| "[]".to_string()))]);
    configuration::rebuild_and_notify(&[srvpro::plugin::tip::NAME]).await;
    Ok(Json(TipResult { message: "tip ok".to_string() }))
}

fn store(tips: &[String]) -> std::io::Result<()> {
    let path = config::directory().join(format!("{FILE_NAME}.yaml"));
    let mut mapping = match fs::read_to_string(&path).ok().and_then(|content| serde_yaml::from_str::<serde_yaml::Value>(&content).ok()) {
        Some(serde_yaml::Value::Mapping(mapping)) => mapping,
        _ => serde_yaml::Mapping::new(),
    };
    mapping.insert(serde_yaml::Value::String(FIELD.to_string()), serde_yaml::to_value(tips).map_err(std::io::Error::other)?);
    let content = serde_yaml::to_string(&serde_yaml::Value::Mapping(mapping)).map_err(std::io::Error::other)?;
    fs::write(&path, content)
}
