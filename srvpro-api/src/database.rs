//! The sqlite connection shared by the http modules reading and writing the srvpro database.

use std::sync::LazyLock;

use parking_lot::Mutex;
use rusqlite::Connection;

pub static CONNECTION: LazyLock<Mutex<Option<Connection>>> = LazyLock::new(|| Mutex::new(None));
