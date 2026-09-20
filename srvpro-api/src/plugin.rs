//! Plugin enablement and configuration refresh over HTTP.

use axum::Json;
use axum::Router;
use axum::routing::get;
use axum::routing::post;
use linkme::distributed_slice;
use serde::Deserialize;
use serde::Serialize;

use crate::auth;
use crate::register_router;
use srvpro::configuration;

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_router!(PLUGIN_ROUTER, build_plugin_router);

const PERMISSION: &str = "change_settings";

#[derive(Deserialize)]
struct PluginsRequest {
    plugins: Vec<String>,
}

#[derive(Serialize)]
struct PluginsResult {
    plugins: Vec<String>,
}

#[derive(Serialize)]
struct PluginResult {
    message: String,
    plugins: Vec<String>,
}

fn build_plugin_router() -> Router {
    auth::required(PERMISSION, Router::new()
        .route("/api/plugins", get(get_plugins))
        .route("/api/plugins/enable", post(enable_plugin))
        .route("/api/plugins/disable", post(disable_plugin))
        .route("/api/plugins/refresh", post(refresh_plugin_config)))
}

async fn get_plugins() -> Json<PluginsResult> {
    Json(PluginsResult { plugins: configuration::get().enable_plugins.iter().cloned().collect() })
}

async fn enable_plugin(Json(request): Json<PluginsRequest>) -> Json<PluginResult> {
    for plugin in &request.plugins {
        configuration::enable(plugin).await;
    }
    Json(PluginResult { message: "enable ok".to_string(), plugins: request.plugins })
}

async fn disable_plugin(Json(request): Json<PluginsRequest>) -> Json<PluginResult> {
    for plugin in &request.plugins {
        configuration::disable(plugin).await;
    }
    Json(PluginResult { message: "disable ok".to_string(), plugins: request.plugins })
}

async fn refresh_plugin_config(Json(request): Json<PluginsRequest>) -> Json<PluginResult> {
    configuration::rebuild_and_notify(&request.plugins).await;
    Json(PluginResult { message: "refresh ok".to_string(), plugins: request.plugins })
}
