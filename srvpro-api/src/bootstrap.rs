use std::fs;
use std::path::Path;

use rand::Rng;
use rand::distributions::Alphanumeric;
use rusqlite::Connection;
use ygopro_derive::Configuration;

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
const PERMISSIONS: &str = "stop,shout,get_rooms,change_settings,manage_users,start_death";
const SECRET_NAME: &str = "jwt";
const SECRET_LENGTH: usize = 64;
const PASSWORD_LENGTH: usize = 64;

#[handler(srvpro::message::Init, priority = 15)]
#[register_to(srvpro::SRVPRO_GLOBAL_HANDLERS as srvpro::GlobalHandler)]
fn on_init() -> &'static str {
    let mut configurations = anymap3::Map::<dyn anymap3::CloneAny + Send + Sync>::new();
    crate::Configuration::default_configuration(&mut configurations).ok();
    Configuration::default_configuration(&mut configurations).ok();
    let Some(database) = configurations.remove::<crate::Configuration>().map(|configuration| configuration.database) else { return "terminate" };
    let Some(initial) = configurations.remove::<Configuration>() else { return "terminate" };
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
