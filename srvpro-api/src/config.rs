//! Configuration entries, overridden by the `config` table of the account database and served over HTTP.

use std::collections::BTreeMap;

use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;

use crate::auth;
use crate::database;
use crate::register_router;
use srvpro::configuration;
use srvpro::managers::config_manager;

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_router!(CONFIG_ROUTER, build_config_router);

const PERMISSION: &str = "change_settings";
const API_ENABLED_PLUGINS_KEY: &str = "api_enabled_plugins";
const API_DISABLED_PLUGINS_KEY: &str = "api_disabled_plugins";

#[derive(Deserialize)]
struct ConfigsQuery {
    prefix: String,
}

#[derive(Deserialize)]
struct SetConfigRequest {
    key: String,
    value: String,
}

#[derive(Deserialize)]
struct DeleteConfigRequest {
    key: String,
}

#[derive(Serialize)]
struct SetConfigResult {
    message: String,
    key: String,
    value: String,
}

#[derive(Serialize)]
struct DeleteConfigResult {
    message: String,
    key: String,
}

fn build_config_router() -> Router {
    auth::required(PERMISSION, Router::new()
        .route("/api/config", get(get_configs).post(set_config).delete(delete_config)))
}

async fn get_configs(Query(query): Query<ConfigsQuery>) -> Json<BTreeMap<String, String>> {
    Json(config_manager::load().prefixed(&query.prefix))
}

async fn set_config(Json(request): Json<SetConfigRequest>) -> Result<Json<SetConfigResult>, StatusCode> {
    if let Err(error) = set(&request.key, &request.value) {
        log::error!("cannot write config table: {error}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok(Json(SetConfigResult { message: "set config ok".to_string(), key: request.key, value: request.value }))
}

async fn delete_config(Json(request): Json<DeleteConfigRequest>) -> Result<Json<DeleteConfigResult>, StatusCode> {
    if let Some(connection) = database::CONNECTION.lock().as_mut() {
        if let Err(error) = remove(connection, &request.key) {
            log::error!("cannot delete config table: {error}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    config_manager::remove(&[request.key.clone()]);
    Ok(Json(DeleteConfigResult { message: "delete config ok".to_string(), key: request.key }))
}

pub fn directory() -> std::path::PathBuf {
    std::env::var("SRVPRO_CONFIG_PATH").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../srvpro/config").to_string()).into()
}

#[handler(srvpro::message::Init, priority = 20)]
#[register_to(srvpro::SRVPRO_GLOBAL_HANDLERS as srvpro::GlobalHandler)]
fn on_init(init: &mut srvpro::message::Init) {
    let mut configurations = anymap3::Map::<dyn anymap3::CloneAny + Send + Sync>::new();
    crate::Configuration::default_configuration(&mut configurations).ok();
    let Some(database) = configurations.remove::<crate::Configuration>().map(|configuration| configuration.database) else { return };
    if database.is_empty() { return }
    let connection = match Connection::open(&database) {
        Ok(connection) => connection,
        Err(error) => {
            log::warn!("cannot open {database}: {error}");
            return;
        }
    };
    match read(&connection) {
        Ok(entries) => {
            let enabled = entries.iter().find(|(key, _)| key.as_str() == API_ENABLED_PLUGINS_KEY).map(|(_, value)| value.as_str());
            let disabled = entries.iter().find(|(key, _)| key.as_str() == API_DISABLED_PLUGINS_KEY).map(|(_, value)| value.as_str());
            configuration::apply_plugin_lists(init, enabled, disabled);
            config_manager::update(entries);
        }
        Err(error) => log::warn!("cannot read config table from {database}: {error}"),
    }
    *database::CONNECTION.lock() = Some(connection);
}

fn read(connection: &Connection) -> rusqlite::Result<Vec<(String, String)>> {
    let mut statement = connection.prepare("SELECT key, value FROM config")?;
    let mut rows = statement.query([])?;
    let mut entries = Vec::new();
    while let Some(row) = rows.next()? {
        entries.push((row.get(0)?, row.get(1)?));
    }
    Ok(entries)
}

fn write(connection: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    connection.execute("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)", rusqlite::params![key, value])?;
    Ok(())
}

fn remove(connection: &Connection, key: &str) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM config WHERE key = ?1", rusqlite::params![key])?;
    Ok(())
}

pub(crate) fn set(key: &str, value: &str) -> rusqlite::Result<()> {
    if let Some(connection) = database::CONNECTION.lock().as_mut() {
        write(connection, key, value)?;
    }
    config_manager::update(vec![(key.to_string(), value.to_string())]);
    Ok(())
}

pub(crate) fn set_plugin_enabled(plugin_name: &str, enabled: bool) {
    for (key, matched) in [(API_ENABLED_PLUGINS_KEY, enabled), (API_DISABLED_PLUGINS_KEY, !enabled)] {
        let value = {
            let current = config_manager::load();
            let mut list = current.get(key).unwrap_or_default().split(',').map(str::trim).filter(|name| !name.is_empty()).collect::<Vec<_>>();
            list.retain(|name| *name != plugin_name);
            if matched { list.push(plugin_name) }
            list.join(",")
        };
        if let Err(error) = set(key, &value) { log::error!("cannot write config table: {error}"); }
    }
}
