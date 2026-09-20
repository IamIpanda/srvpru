//! Account authentication backed by a SQLite account database, by password or by signed token.

use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::metadata;
use std::sync::LazyLock;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use axum::Json;
use axum::Router;
use axum::extract::Request;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::middleware;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::post;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use linkme::distributed_slice;
use parking_lot::RwLock;
use ring::hmac;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Result;
use serde::Deserialize;
use serde::Serialize;
use ygopro_derive::Configuration;

use srvpro::configuration;

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

crate::register_router!(AUTH_ROUTER, build_auth_router);

#[derive(Clone, Configuration)]
#[config(sync, register_to = "srvpro::plugin::CONFIGURATIONS")]
pub struct Configuration {
    #[config(default = "180")]
    pub token_ttl_days: u64,
}

const ALGORITHM: &str = "HS256";
const SECRET_NAME: &str = "jwt";
const DEFAULT_TOKEN_TTL_DAYS: u64 = 180;

/// The caller a request was authenticated as; carried in request extensions by the middleware.
#[derive(Clone, Debug)]
pub struct Identity {
    pub username: String,
}

#[derive(Clone, Debug)]
struct User {
    password: String,
    permissions: HashSet<String>,
}

#[derive(Default)]
struct Snapshot {
    database: String,
    modified: Option<SystemTime>,
    secret: Vec<u8>,
    users: HashMap<String, User>,
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
}

#[derive(Deserialize)]
struct Credentials {
    #[serde(default)]
    username: String,
    #[serde(default)]
    pass: String,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
    expires_in: u64,
}

static SNAPSHOT: LazyLock<RwLock<Snapshot>> = LazyLock::new(|| RwLock::new(Snapshot::default()));

pub fn required(permission: &'static str, router: Router) -> Router {
    router.layer(middleware::from_fn_with_state(permission, require_permission))
}

pub fn optional(permission: &'static str, router: Router) -> Router {
    router.layer(middleware::from_fn_with_state(permission, allow_permission))
}

async fn require_permission(State(permission): State<&'static str>, mut request: Request, next: Next) -> Response {
    let Some(identity) = identify(&request, permission) else { return StatusCode::FORBIDDEN.into_response() };
    request.extensions_mut().insert(identity);
    next.run(request).await
}

async fn allow_permission(State(permission): State<&'static str>, mut request: Request, next: Next) -> Response {
    if let Some(identity) = identify(&request, permission) { request.extensions_mut().insert(identity); }
    next.run(request).await
}

fn build_auth_router() -> Router {
    Router::new().route("/api/login", post(login))
}

async fn login(Json(request): Json<LoginRequest>) -> Result<Json<LoginResponse>, StatusCode> {
    let mut snapshot = SNAPSHOT.write();
    if !refresh(&mut snapshot) { return Err(StatusCode::FORBIDDEN) }
    if !password_matches(&snapshot, &request.username, &request.password) { return Err(StatusCode::FORBIDDEN) }
    let Some(token) = issue_token(&snapshot, &request.username) else { return Err(StatusCode::FORBIDDEN) };
    Ok(Json(LoginResponse { token, expires_in: token_ttl().as_secs() }))
}

fn identify(request: &Request, permission: &str) -> Option<Identity> {
    let mut snapshot = SNAPSHOT.write();
    if !refresh(&mut snapshot) { return None }
    let (username, credential) = credentials(request)?;
    let username = if is_token(&credential) {
        verify_token(&snapshot, &credential)?
    } else {
        if !password_matches(&snapshot, &username, &credential) { return None }
        username
    };
    let user = snapshot.users.get(&username)?;
    user.permissions.contains(permission).then_some(Identity { username })
}

fn credentials(request: &Request) -> Option<(String, String)> {
    if let Some(token) = bearer(request.headers()) { return Some((String::new(), token.to_string())) }
    let credentials: Credentials = serde_urlencoded::from_str(request.uri().query()?).ok()?;
    (!credentials.pass.is_empty()).then_some((credentials.username, credentials.pass))
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    let (scheme, token) = headers.get(AUTHORIZATION)?.to_str().ok()?.split_once(' ')?;
    scheme.eq_ignore_ascii_case("bearer").then(|| token.trim())
}

fn is_token(credential: &str) -> bool {
    credential.split('.').count() == 3
}

fn password_matches(snapshot: &Snapshot, username: &str, password: &str) -> bool {
    snapshot.users.get(username).is_some_and(|user| user.password == password)
}

fn verify_token(snapshot: &Snapshot, token: &str) -> Option<String> {
    if snapshot.secret.is_empty() { return None }
    let mut parts = token.split('.');
    let (header, claims, signature) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() { return None }
    let key = hmac::Key::new(hmac::HMAC_SHA256, &snapshot.secret);
    let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
    hmac::verify(&key, format!("{header}.{claims}").as_bytes(), &signature).ok()?;
    let header: serde_json::Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(header).ok()?).ok()?;
    if header.get("alg").and_then(|algorithm| algorithm.as_str()) != Some(ALGORITHM) { return None }
    let claims: Claims = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims).ok()?).ok()?;
    (claims.exp > now()).then_some(claims.sub)
}

fn issue_token(snapshot: &Snapshot, username: &str) -> Option<String> {
    if snapshot.secret.is_empty() { return None }
    let header = encode(&serde_json::json!({ "alg": ALGORITHM, "typ": "JWT" }))?;
    let claims = encode(&serde_json::json!({ "sub": username, "exp": now() + token_ttl().as_secs() }))?;
    let signing_input = format!("{header}.{claims}");
    let key = hmac::Key::new(hmac::HMAC_SHA256, &snapshot.secret);
    let signature = URL_SAFE_NO_PAD.encode(hmac::sign(&key, signing_input.as_bytes()).as_ref());
    Some(format!("{signing_input}.{signature}"))
}

fn encode(value: &serde_json::Value) -> Option<String> {
    serde_json::to_vec(value).ok().map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs()).unwrap_or_default()
}

fn token_ttl() -> Duration {
    let days = configuration::get_configuration::<Configuration>().map(|configuration| configuration.token_ttl_days).unwrap_or(DEFAULT_TOKEN_TTL_DAYS);
    Duration::from_secs(days * 24 * 60 * 60)
}

fn refresh(snapshot: &mut Snapshot) -> bool {
    let Some(configuration) = configuration::get_configuration::<crate::Configuration>() else { return false };
    let database = configuration.database;
    let Ok(modified) = metadata(&database).and_then(|metadata| metadata.modified()) else { return false };
    if snapshot.database == database && snapshot.modified == Some(modified) { return true }
    match read(&database) {
        Ok((users, secret)) => { *snapshot = Snapshot { database, modified: Some(modified), secret, users }; true },
        Err(error) => { log::warn!("cannot read auth database {database}: {error}"); false },
    }
}

fn read(database: &str) -> Result<(HashMap<String, User>, Vec<u8>)> {
    let connection = Connection::open(database)?;
    let secret = connection.query_row("SELECT value FROM secrets WHERE name = ?1", [SECRET_NAME], |row| row.get::<_, String>(0)).optional()?.unwrap_or_default().into_bytes();
    let mut statement = connection.prepare("SELECT username, password, permissions FROM users")?;
    let mut users = HashMap::new();
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let permissions = row.get::<_, String>(2)?.split(',').map(|permission| permission.trim().to_owned()).collect();
        users.insert(row.get(0)?, User { password: row.get(1)?, permissions });
    }
    Ok((users, secret))
}
