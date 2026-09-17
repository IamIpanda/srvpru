//! Bootstrap of the account database: file, schema and the first account.

use std::fs::create_dir_all;
use std::path::Path;

use rand::Rng;
use rand::distributions::Alphanumeric;
use rusqlite::Connection;
use ygopro_derive::Configuration;

use srvpro::configuration;

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync, register_to = "srvpro::plugin::CONFIGURATIONS")]
pub struct Configuration {
    #[config(default = "\"admin\".to_string()")]
    pub username: String,
    #[config(default = "\"admin\".to_string()")]
    pub password: String,
}

const SCHEMA: &str = include_str!("init.sql");
const PERMISSIONS: &str = r#"{"stop":true,"shout":true,"get_rooms":true}"#;
const SECRET_NAME: &str = "jwt";
const SECRET_LENGTH: usize = 64;

#[before(srvpro::message::Init)]
#[register_to(srvpro::SRVPRO_GLOBAL_HANDLERS as srvpro::GlobalHandler)]
fn on_init() {
    let configuration = configuration::get();
    let Some(database) = configuration.configurations.get::<crate::Configuration>().map(|configuration| configuration.database.clone()) else { return };
    let Some(initial) = configuration.configurations.get::<Configuration>().cloned() else { return };
    if database.is_empty() { return }
    if let Err(error) = bootstrap(&database, &initial) { log::error!("cannot bootstrap account database {database}: {error}") }
}

fn bootstrap(database: &str, initial: &Configuration) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(directory) = Path::new(database).parent() && !directory.as_os_str().is_empty() { create_dir_all(directory)? }
    let connection = Connection::open(database)?;
    connection.execute_batch(SCHEMA)?;
    create_secret(&connection, database)?;
    let accounts: i64 = connection.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    if accounts > 0 { return Ok(()) }
    connection.execute("INSERT INTO users (username, password, permissions) VALUES (?1, ?2, ?3)", rusqlite::params![initial.username, initial.password, PERMISSIONS])?;
    log::warn!("created the initial account {} in {database}, change its password before exposing the API", initial.username);
    Ok(())
}

fn create_secret(connection: &Connection, database: &str) -> rusqlite::Result<()> {
    let secret: String = rand::thread_rng().sample_iter(&Alphanumeric).take(SECRET_LENGTH).map(char::from).collect();
    let created = connection.execute("INSERT OR IGNORE INTO secrets (name, value) VALUES (?1, ?2)", rusqlite::params![SECRET_NAME, secret])?;
    if created > 0 { log::warn!("created the {SECRET_NAME} signing secret in {database}") }
    Ok(())
}
