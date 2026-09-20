//! Configuration entries, overridden by the `config` table of the account database and served over HTTP.

use std::collections::BTreeMap;

use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use linkme::distributed_slice;
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
    if let Some(connection) = database::CONNECTION.lock().as_mut() {
        if let Err(error) = write(connection, &request.key, &request.value) {
            log::error!("cannot write config table: {error}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    config_manager::update(vec![(request.key.clone(), request.value.clone())]);
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

#[handler(srvpro::message::Init, priority = 11)]
#[register_to(srvpro::SRVPRO_GLOBAL_HANDLERS as srvpro::GlobalHandler)]
fn on_init() {
    let Some(database) = configuration::get_configuration::<crate::Configuration>().map(|configuration| configuration.database) else { return };
    if database.is_empty() { return }
    let connection = match Connection::open(&database) {
        Ok(connection) => connection,
        Err(error) => {
            log::warn!("cannot open {database}: {error}");
            return;
        }
    };
    match read(&connection) {
        Ok(entries) => config_manager::update(entries),
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
