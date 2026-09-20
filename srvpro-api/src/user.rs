//! Accounts of the srvpro database: list, create or update, and delete them.

use axum::Json;
use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use linkme::distributed_slice;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;

use crate::auth;
use crate::database;
use crate::register_router;

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_router!(USER_ROUTER, build_user_router);

const PERMISSION: &str = "manage_users";

#[derive(Serialize)]
struct User {
    username: String,
    permissions: Vec<String>,
}

#[derive(Serialize)]
struct UsersResult {
    users: Vec<User>,
}

#[derive(Deserialize)]
struct SaveUserRequest {
    username: String,
    #[serde(default)]
    password: String,
    permissions: Vec<String>,
}

#[derive(Deserialize)]
struct DeleteUserRequest {
    username: String,
}

#[derive(Serialize)]
struct UserResult {
    message: String,
    username: String,
}

fn build_user_router() -> Router {
    auth::required(PERMISSION, Router::new().route("/api/users", get(get_users).post(save_user).delete(delete_user)))
}

async fn get_users() -> Result<Json<UsersResult>, StatusCode> {
    let guard = database::CONNECTION.lock();
    let Some(connection) = guard.as_ref() else { return Err(StatusCode::INTERNAL_SERVER_ERROR) };
    let users = read(connection).map_err(|error| {
        log::warn!("cannot read users: {error}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(UsersResult { users }))
}

async fn save_user(Json(request): Json<SaveUserRequest>) -> Result<Json<UserResult>, StatusCode> {
    if request.username.is_empty() { return Err(StatusCode::BAD_REQUEST) }
    let guard = database::CONNECTION.lock();
    let Some(connection) = guard.as_ref() else { return Err(StatusCode::INTERNAL_SERVER_ERROR) };
    let permissions = request.permissions.join(",");
    let saved = if request.password.is_empty() {
        connection.execute("UPDATE users SET permissions = ?2 WHERE username = ?1", rusqlite::params![request.username, permissions])
    } else {
        connection.execute("INSERT OR REPLACE INTO users (username, password, permissions) VALUES (?1, ?2, ?3)", rusqlite::params![request.username, request.password, permissions])
    };
    match saved {
        Ok(0) => Err(StatusCode::NOT_FOUND),
        Ok(_) => Ok(Json(UserResult { message: "save ok".to_string(), username: request.username })),
        Err(error) => {
            log::warn!("cannot save user {}: {error}", request.username);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn delete_user(Json(request): Json<DeleteUserRequest>) -> Result<Json<UserResult>, StatusCode> {
    let guard = database::CONNECTION.lock();
    let Some(connection) = guard.as_ref() else { return Err(StatusCode::INTERNAL_SERVER_ERROR) };
    match connection.execute("DELETE FROM users WHERE username = ?1", rusqlite::params![request.username]) {
        Ok(0) => Err(StatusCode::NOT_FOUND),
        Ok(_) => Ok(Json(UserResult { message: "delete ok".to_string(), username: request.username })),
        Err(error) => {
            log::warn!("cannot delete user {}: {error}", request.username);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn read(connection: &Connection) -> rusqlite::Result<Vec<User>> {
    let mut statement = connection.prepare("SELECT username, permissions FROM users ORDER BY username")?;
    let mut rows = statement.query([])?;
    let mut users = Vec::new();
    while let Some(row) = rows.next()? {
        let permissions: String = row.get(1)?;
        users.push(User { username: row.get(0)?, permissions: split(&permissions) });
    }
    Ok(users)
}

fn split(permissions: &str) -> Vec<String> {
    permissions.split(',').map(|permission| permission.trim().to_owned()).filter(|permission| !permission.is_empty()).collect()
}
