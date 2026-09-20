use std::fs;
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
    pub password: String,
}

const SCHEMA: &str = include_str!("init.sql");
const PERMISSIONS: &str = "stop,shout,get_rooms,change_settings,manage_users";
const SECRET_NAME: &str = "jwt";
const SECRET_LENGTH: usize = 64;
const PASSWORD_LENGTH: usize = 64;

#[handler(srvpro::message::Init, priority = 20)]
#[register_to(srvpro::SRVPRO_GLOBAL_HANDLERS as srvpro::GlobalHandler)]
fn on_init() -> &'static str {
    let database = configuration::get_configuration::<crate::Configuration>().expect("srvpro-api configuration is not registered").database;
    let initial = configuration::get_configuration::<Configuration>().expect("account bootstrap configuration is not registered");
    if database.is_empty() {
        log::error!("srvpro database is not configured");
        return "terminate";
    }
    if let Err(error) = bootstrap(&database, &initial) {
        log::error!("cannot bootstrap account database {database}: {error}");
        return "terminate";
    }
    "continue"
}

fn bootstrap(database: &str, initial: &Configuration) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(directory) = Path::new(database).parent() && !directory.as_os_str().is_empty() { fs::create_dir_all(directory)? }
    let connection = Connection::open(database)?;
    connection.execute_batch(SCHEMA)?;
    create_secret(&connection, database)?;
    let accounts: i64 = connection.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    if accounts > 0 { return Ok(()) }
    let password = if initial.password.is_empty() {
        rand::thread_rng().sample_iter(&Alphanumeric).take(PASSWORD_LENGTH).map(char::from).collect()
    } else {
        initial.password.clone()
    };
    connection.execute("INSERT INTO users (username, password, permissions) VALUES (?1, ?2, ?3)", rusqlite::params![initial.username, password, PERMISSIONS])?;
    if initial.password.is_empty() {
        log::warn!("no initial password is configured, created the account {} in {database} with the generated password {password}", initial.username);
    } else {
        log::warn!("created the initial account {} with given password in {database}", initial.username);
    }
    Ok(())
}

fn create_secret(connection: &Connection, database: &str) -> rusqlite::Result<()> {
    let secret = rand::thread_rng().sample_iter(&Alphanumeric).take(SECRET_LENGTH).map(char::from).collect::<String>();
    let created = connection.execute("INSERT OR IGNORE INTO secrets (name, value) VALUES (?1, ?2)", rusqlite::params![SECRET_NAME, secret])?;
    if created > 0 { log::warn!("created the {SECRET_NAME} signing secret in {database}") }
    Ok(())
}
