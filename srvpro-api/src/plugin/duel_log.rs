//! Duel log: persist the report of every room and serve the tournament board.

// srvpro's duel-log endpoints, all of them GET:
//   /api/duellog           duel_log / duel_log
//   /api/archive.zip       download_replay / download_replay_archive
//   /api/clearlog          clear_duel_log / clear_duel_log
//   /api/replay/<filename> download_replay / download_replay

use axum::Json;
use axum::Router;
use axum::extract::Path;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::types::Value;
use serde::Deserialize;
use serde::Serialize;
use ygopro_derive::Configuration;

use srvpro::configuration;
use srvpro::plugin::report::Report;
use srvpro::room::Anymap;
use srvpro::room::SRVPRO_HANDLERS as ROOM_HANDLERS;
use srvpro::room::SrvproMessageHandler;

use crate::auth;
use crate::database::CONNECTION;
use crate::register_router;

/// Name for activitating this module in the plugin system.
pub static NAME: &'static str = module_path!();

const PERMISSION: &str = "duel_log";
const DEFAULT_LIMIT: i64 = 50;
const LIST_LIMIT: i64 = 10;
const BASE64_PREFIX: &str = "base64://";
const FILE_PREFIX: &str = "file://";

register_router!(DUEL_LOG_ROUTER, build_duel_log_router);

#[derive(Clone, Configuration)]
#[config(sync, register_to = "srvpro::plugin::CONFIGURATIONS")]
pub struct Configuration {
    /// Store replays inline as base64 instead of referencing their file path.
    pub always_save_replay_to_database: bool,
}

fn build_duel_log_router() -> Router {
    auth::required(PERMISSION, Router::new()
        .route("/api/duellog", get(list))
        .route("/api/duellog/list/{player}", get(list_player_replays))
        .route("/api/replay/{id}", get(get_replay)))
}

#[derive(Serialize)]
pub struct DuelLog {
    id: String,
    name: String,
    duel_count: i64,
    players: Vec<String>,
    start_time: String,
    end_time: String,
    team_a_score: i64,
    team_b_score: i64,
    replays: Vec<String>,
}

#[after(srvpro::message::Terminate)]
#[register_to(ROOM_HANDLERS as SrvproMessageHandler)]
fn on_terminate(states: &mut Anymap) {
    let Some(report) = states.get::<Report>() else { return };
    let Some(configuration) = configuration::get_configuration::<Configuration>() else { return };
    let guard = CONNECTION.lock();
    let Some(connection) = guard.as_ref() else { return };
    if let Err(error) = write(connection, report, &configuration) { log::warn!("cannot write duel log: {error}") }
}

fn write(connection: &Connection, report: &Report, configuration: &Configuration) -> rusqlite::Result<()> {
    let Some(id) = report.replays.first().map(|replay| replay.id.as_str()) else {
        log::warn!("report has no replay, skip duel log.");
        return Ok(());
    };
    connection.execute(
        "INSERT INTO duel_log (id, name, duel_count, players, start_time, end_time, team_a_score, team_b_score, replays) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![id, report.name, duel_count(report), players(report), report.start_time.to_rfc3339(), report.end_time.to_rfc3339(), team_score(report, 0), team_score(report, opposing_team(report)), encode_replays(report, configuration)],
    )?;
    Ok(())
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ListQuery {
    callback: String,
    #[serde(alias = "roomname")]
    name: String,
    #[serde(alias = "duelcount")]
    duel_count: String,
    #[serde(alias = "playername")]
    player_name: String,
    score: String,
    limit: String,
    offset: String,
}

async fn list(Query(query): Query<ListQuery>) -> Response {
    let guard = CONNECTION.lock();
    let Some(connection) = guard.as_ref() else { return StatusCode::INTERNAL_SERVER_ERROR.into_response() };
    let logs = match read(connection, &query) {
        Ok(logs) => logs,
        Err(error) => {
            log::warn!("cannot read duel logs: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let classic_logs = logs.iter().map(to_classic).collect::<Vec<SrvproDuelLogResponse>>();
    let body = serde_json::to_string_pretty(&classic_logs).unwrap_or_else(|_| "[]".to_owned());
    if query.callback.is_empty() {
        ([(CONTENT_TYPE, "application/json")], body).into_response()
    } else {
        ([(CONTENT_TYPE, "text/javascript")], format!("{}( {} );", query.callback, body)).into_response()
    }
}

async fn list_player_replays(Path(player): Path<String>) -> Response {
    let guard = CONNECTION.lock();
    let Some(connection) = guard.as_ref() else { return StatusCode::INTERNAL_SERVER_ERROR.into_response() };
    match search_by_player_id(connection, &player) {
        Ok(ids) => Json(ids).into_response(),
        Err(error) => {
            log::warn!("cannot read duel logs: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn get_replay(Path(id): Path<String>) -> Response {
    let guard: parking_lot::lock_api::MutexGuard<'_, parking_lot::RawMutex, Option<Connection>> = CONNECTION.lock();
    let Some(connection) = guard.as_ref() else { return StatusCode::INTERNAL_SERVER_ERROR.into_response() };
    match search_replays(connection, &id) {
        Some(entries) => {
            let mut body = Vec::new();
            for entry in &entries {
                match resolve_replay(entry.as_str()).await {
                    Some(bytes) => body.extend(bytes),
                    None => log::warn!("cannot resolve replay entry {entry}"),
                }
            }
            ([(CONTENT_TYPE, "application/octet-stream")], body).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn search_by_player_id(connection: &Connection, player: &str) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection.prepare("SELECT id FROM duel_log WHERE instr(',' || players || ',', ',' || ?1 || '=') > 0 ORDER BY rowid DESC LIMIT ?2")?;
    let mut rows = statement.query(rusqlite::params![player, LIST_LIMIT])?;
    let mut ids = Vec::new();
    while let Some(row) = rows.next()? {
        ids.push(row.get(0)?);
    }
    Ok(ids)
}

fn search_replays(connection: &Connection, id: &str) -> Option<Vec<String>> {
    let replays: String = connection.query_row("SELECT replays FROM duel_log WHERE id = ?1", rusqlite::params![id], |row| row.get(0)).optional().ok()??;
    Some(replays.split(',').filter(|entry| !entry.is_empty()).map(str::to_owned).collect())
}

async fn resolve_replay(entry: &str) -> Option<Vec<u8>> {
    if let Some(encoded) = entry.strip_prefix(BASE64_PREFIX) { return STANDARD.decode(encoded).ok() }
    if let Some(path) = entry.strip_prefix(FILE_PREFIX) {
        let path = path.to_owned();
        return tokio::task::spawn_blocking(move || std::fs::read(path).ok()).await.ok().flatten()
    }
    None
}

fn duel_count(report: &Report) -> i64 {
    report.players.iter().next().map(|(_index, player)| player.wins.len()).unwrap_or_default() as i64
}

fn players(report: &Report) -> String {
    let mut players = report.players.iter().map(|(_index, player)| (u8::from(player.pos), format!("{}={}", player.origin_name, player.ip))).collect::<Vec<(u8, String)>>();
    players.sort();
    players.into_iter().map(|(_pos, player)| player).collect::<Vec<String>>().join(",")
}

fn team_score(report: &Report, pos: u8) -> i64 {
    report.players.iter().find(|(_index, player)| u8::from(player.pos) == pos).map(|(_index, player)| player.score).unwrap_or_default() as i64
}

fn opposing_team(report: &Report) -> u8 {
    report.players.iter().map(|(_index, player)| u8::from(player.pos)).find(|index| *index >= 2).unwrap_or(1)
}

fn encode_replays(report: &Report, configuration: &Configuration) -> String {
    report.replays.iter().map(|replay| {
        if configuration.always_save_replay_to_database {
            format!("{BASE64_PREFIX}{}", STANDARD.encode(&replay.data))
        } else {
            replay.path.as_ref().map(|path| format!("{FILE_PREFIX}{}", path.display())).unwrap_or_else(|| format!("{BASE64_PREFIX}{}", STANDARD.encode(&replay.data)))
        }
    }).collect::<Vec<String>>().join(",")
}

fn read(connection: &Connection, query: &ListQuery) -> rusqlite::Result<Vec<DuelLog>> {
    let mut sql = String::from("SELECT id, name, duel_count, players, start_time, end_time, team_a_score, team_b_score, replays FROM duel_log WHERE 1");
    let mut parameters: Vec<Value> = Vec::new();
    let name = query.name.trim();
    if !name.is_empty() {
        sql.push_str(" AND name = ?");
        parameters.push(name.to_owned().into());
    }
    if let Ok(duel_count) = query.duel_count.trim().parse::<i64>() {
        sql.push_str(" AND duel_count = ?");
        parameters.push(duel_count.into());
    }
    let player_name = query.player_name.trim();
    if !player_name.is_empty() {
        sql.push_str(" AND ',' || players || ',' LIKE ?");
        parameters.push(format!("%,{player_name}=%").into());
    }
    if let Ok(score) = query.score.trim().parse::<i64>() {
        sql.push_str(" AND (team_a_score = ? OR team_b_score = ?)");
        parameters.push(score.into());
        parameters.push(score.into());
    }
    let limit = query.limit.trim().parse::<i64>().ok().unwrap_or(DEFAULT_LIMIT);
    let offset = query.offset.trim().parse::<i64>().ok().unwrap_or(0);
    sql.push_str(" ORDER BY rowid DESC LIMIT ? OFFSET ?");
    parameters.push(limit.into());
    parameters.push(offset.into());
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query(rusqlite::params_from_iter(parameters))?;
    let mut logs = Vec::new();
    while let Some(row) = rows.next()? {
        let players: String = row.get(3)?;
        let replays: String = row.get(8)?;
        logs.push(DuelLog {
            id: row.get(0)?,
            name: row.get(1)?,
            duel_count: row.get(2)?,
            players: players.split(',').filter(|player| !player.is_empty()).map(str::to_owned).collect(),
            start_time: row.get(4)?,
            end_time: row.get(5)?,
            team_a_score: row.get(6)?,
            team_b_score: row.get(7)?,
            replays: replays.split(',').filter(|replay| !replay.is_empty()).map(str::to_owned).collect(),
        });
    }
    Ok(logs)
}


#[derive(Serialize)]
pub struct SrvproDuelLogResponse {
    id: String,
    room_id: Option<i64>,
    time: String,
    name: String,
    cloud_replay_id: Option<String>,
    replay_file_name: String,
    room_mode: Option<i64>,
    duel_count: i64,
    players: Vec<SrvproDuelLogPlayerResponse>,
}

#[derive(Serialize)]
pub struct SrvproDuelLogPlayerResponse {
    real_name: String,
    ip: String,
    score: i64,
    // Dropped from the original srvpro response, and the design behind them is nonsense: none of
    // these fields carry information the stored replay does not already have, and every value the
    // original filled in was a mid-duel snapshot that means nothing once the duel is over.
    // is_first: bool,
    // lp: i64,
    // card_count: i64,
    // start_deck_buffer: String,
    // current_deck_buffer: String,
    // winner: bool,
}

pub fn to_classic(log: &DuelLog) -> SrvproDuelLogResponse {
    let tag = log.players.len() > 2;
    let start = chrono::DateTime::parse_from_rfc3339(&log.start_time).map(|time| time.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_else(|_| log.start_time.clone());
    let end = chrono::DateTime::parse_from_rfc3339(&log.end_time).map(|time| time.format("%H:%M:%S").to_string()).unwrap_or_else(|_| log.end_time.clone());
    SrvproDuelLogResponse {
        id: log.id.clone(),
        room_id: None,
        time: format!("{start} - {end}"),
        name: log.name.clone(),
        cloud_replay_id: None,
        replay_file_name: log.replays.iter().find_map(|replay| replay.strip_prefix(FILE_PREFIX)).and_then(|path| std::path::Path::new(path).file_name()).and_then(|name| name.to_str()).map(str::to_owned).unwrap_or_default(),
        room_mode: None,
        duel_count: log.duel_count,
        players: log.players.iter().enumerate().map(|(index, player)| {
            let (real_name, ip) = player.split_once('=').map(|(name, ip)| (name.to_owned(), ip.to_owned())).unwrap_or_else(|| (player.clone(), String::new()));
            let team = if tag { index / 2 } else { index };
            SrvproDuelLogPlayerResponse { real_name, ip, score: if team == 0 { log.team_a_score } else { log.team_b_score } }
        }).collect(),
    }
}
