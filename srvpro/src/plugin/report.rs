//! Send the arena report form at the end of a duel, mirroring the original
//! srvpro's arena_mode post_score form.

use std::io::Cursor;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use binrw::BinWrite;
use chrono::Local;
use ygopro_data::constants::Netplayer;
use ygopro_data::data::Deck;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::Configuration;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::message as srvpro;
use crate::plugin::base::first_attack::FirstAttack;
use crate::plugin::base::replays::Replays;
use crate::plugin::base::score::Score;
use crate::plugin::register_dependencies;
use crate::room::Room;
use crate::room::SRVPRO_HANDLERS;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;
use crate::room::SrvproMessageHandler;

pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::name::NAME,
    crate::plugin::base::position::NAME,
    crate::plugin::base::deck::NAME,
    crate::plugin::base::score::NAME,
    crate::plugin::base::first_attack::NAME,
    crate::plugin::base::replays::NAME
);

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    pub url: String,
    pub access_key: String,
    pub arena: String,
}

#[derive(Attachment)]
pub struct StartTime {
    pub start: String,
}

#[handler(stoc::DuelStart)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_start(start_time: &mut StartTime) {
    start_time.start = now_string();
}

#[handler(srvpro::Terminate)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_terminate(room: &mut Room, score: &mut Score, first_attack: &mut FirstAttack, replays: &mut Replays, start_time: &mut StartTime, configuration: Configuration) {
    if configuration.url.is_empty() {
        log::warn!("report url is not configured, skip posting arena form.");
        return;
    }
    let form = build_form(room, score, first_attack, replays, start_time, &configuration);
    let url = configuration.url;
    // Detached: the room event loop must never stall on a remote http endpoint.
    tokio::task::spawn_blocking(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).build().into();
        match agent.post(&url).send_form(form) {
            Ok(response) => log::info!("arena report posted: {}", response.status()),
            Err(error) => log::warn!("arena report post failed: {}", error),
        }
    });
}

fn build_form(room: &Room, score: &Score, first_attack: &FirstAttack, replays: &Replays, start_time: &StartTime, configuration: &Configuration) -> Vec<(String, String)> {
    let player1 = room.players.iter().find(|(_index, player)| player.states.get::<Netplayer>() == Some(&Netplayer::Player(0)));
    let player2 = room.players.iter().find(|(_index, player)| player.states.get::<Netplayer>() == Some(&Netplayer::Player(1)));
    let Some((_, player1)) = player1 else { return vec![] };
    let Some((_, player2)) = player2 else { return vec![] };
    let player1_name = player1.states.get::<String>().cloned().unwrap_or_default();
    let player2_name = player2.states.get::<String>().cloned().unwrap_or_default();
    let first: Vec<String> = first_attack.first_attack.iter().filter_map(|netplayer| match netplayer {
        Netplayer::Player(0) => Some(player1_name.clone()),
        Netplayer::Player(1) => Some(player2_name.clone()),
        _ => None,
    }).collect();
    let wins: Vec<String> = score.winners.iter().filter_map(|winner| match winner {
        Some(Netplayer::Player(0)) => Some(player1_name.clone()),
        Some(Netplayer::Player(1)) => Some(player2_name.clone()),
        _ => None,
    }).collect();
    let replays: Vec<String> = replays.replays.iter().map(replay_base64).collect();
    let end = now_string();

    let mut form: Vec<(String, String)> = Vec::new();
    form.push(("accesskey".to_string(), configuration.access_key.clone()));
    form.push(("usernameA".to_string(), player1.states.get::<String>().cloned().unwrap_or_default()));
    form.push(("usernameB".to_string(), player2.states.get::<String>().cloned().unwrap_or_default()));
    form.push(("userscoreA".to_string(), score.winners.iter().filter(|winner| **winner == Some(Netplayer::Player(0))).count().to_string()));
    form.push(("userscoreB".to_string(), score.winners.iter().filter(|winner| **winner == Some(Netplayer::Player(1))).count().to_string()));
    form.push(("userdeckA".to_string(), player1.states.get::<Vec<Deck>>().and_then(|decks| decks.first()).map(Deck::to_string).unwrap_or_default()));
    form.push(("userdeckB".to_string(), player2.states.get::<Vec<Deck>>().and_then(|decks| decks.first()).map(Deck::to_string).unwrap_or_default()));
    form.push(("userdeckAHistory".to_string(), serde_json::to_string(&player1.states.get::<Vec<Deck>>().map(|decks| decks.iter().map(Deck::to_string).collect::<Vec<String>>()).unwrap_or_default()).unwrap_or_default()));
    form.push(("userdeckBHistory".to_string(), serde_json::to_string(&player2.states.get::<Vec<Deck>>().map(|decks| decks.iter().map(Deck::to_string).collect::<Vec<String>>()).unwrap_or_default()).unwrap_or_default()));
    form.push(("first".to_string(), serde_json::to_string(&first).unwrap_or_default()));
    form.push(("wins".to_string(), serde_json::to_string(&wins).unwrap_or_default()));
    form.push(("replays".to_string(), serde_json::to_string(&replays).unwrap_or_default()));
    form.push(("start".to_string(), start_time.start.clone()));
    form.push(("end".to_string(), end));
    form.push(("arena".to_string(), configuration.arena.clone()));
    form.push(("nonce".to_string(), rand::random::<f64>().to_string()));
    form
}

fn replay_base64(replay: &stoc::Replay) -> String {
    let mut bytes = Vec::new();
    replay.replay.write_le(&mut Cursor::new(&mut bytes)).ok();
    STANDARD.encode(&bytes)
}

fn now_string() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

