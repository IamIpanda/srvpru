//! Send the arena report form at the end of a duel, mirroring the original
//! srvpro's arena_mode post_score form.

use std::net::Ipv4Addr;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::DateTime;
use chrono::Local;
use slab::Slab;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use ygopro::plugin::replay::Format;
use ygopro::plugin::replay::ReplayWithName;
use ygopro_data::constants::Netplayer;
use ygopro_data::data::Deck;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::Configuration;
use ygopro_handler::All;

use crate::message as srvpro;
use crate::plugin::register_dependencies;
use crate::plugin::base::first_attack::FirstAttack;
use crate::plugin::base::ip::get_ip;
use crate::plugin::base::score::Score;
use crate::plugin::virtual_password::OriginName;
use crate::room::*;
use crate::mode::*;

pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::name::NAME,
    crate::plugin::base::ip::NAME,
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
    #[config(default = "ygopro::plugin::replay::Format::YgoproForge")]
    pub format: Format,
}

#[derive(Attachment)]
pub struct StartTime {
    pub start: DateTime<Local>,
}

pub struct Report {
    pub name: String,
    pub players: Slab<ReportPlayer>,
    pub replays: Vec<ReplayWithName>,
    pub start_time: DateTime<Local>,
    pub end_time: DateTime<Local>,
}

pub struct ReportPlayer {
    pub pos: Netplayer,
    pub name: String,
    pub origin_name: String,
    pub score: usize,
    pub decks: Vec<Deck>,
    pub ip: Ipv4Addr,
    pub first: Vec<bool>,
    pub wins: Vec<bool>,
}

impl Report {
    pub fn player_at_pos(&self, pos: Netplayer) -> Option<&ReportPlayer> {
        self.players.iter().find(|(_index, player)| player.pos == pos).map(|(_index, player)| player)
    }
}

#[handler(All)]
#[register_to(MODES as ModeHandler)]
fn on_configure(config: &mut RoomConfiguration) {
    let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return };
    config.ygopru_configuration.enable_plugin_with_inited::<ygopro::plugin::replay::Configuration>(ygopro::plugin::replay::NAME, 
        |replay| replay.into_mut().format = configuration.format);
}

#[handler(stoc::DuelStart)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_start(start_time: &mut StartTime) {
    start_time.start = Local::now();
}

#[after(srvpro::CreateProvider)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
async fn on_create_provider(room: &mut Room, states: &mut Anymap) {
    let Some(Provider::Embedded(provider)) = room.provider.as_ref() else { return };
    let (sender, receiver) = oneshot::channel();
    provider.ctos_sender.send(ygopro::duel::Request::Query(Box::new(move |_duel, duel_states| {
        sender.send(duel_states.get::<ygopro::plugin::replay::Attachment>().map(|attachment| attachment.subscribe())).ok();
    }))).ok();
    let Ok(Some(receiver)) = receiver.await else { return };
    states.insert(receiver);
}

#[handler(srvpro::Terminate)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_terminate(room: &mut Room, states: &mut Anymap, score: &mut Score, first_attack: &mut FirstAttack, start_time: &mut StartTime, tag: TagFlag) {
    let replays = states.remove::<broadcast::Receiver<ReplayWithName>>().map(|mut receiver| {
        let mut collected = Vec::new();
        while let Ok(replay) = receiver.try_recv() { collected.push(replay); }
        collected
    }).unwrap_or_default();
    let mut report = build_report(room, score, first_attack, start_time, tag);
    report.replays = replays;
    states.insert(report);
}

#[after(srvpro::Terminate)]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_terminate_post(states: &mut Anymap, configuration: Configuration, tag: TagFlag) {
    let Some(report) = states.get::<Report>() else { return };
    let player_a = Netplayer::Player(0);
    let player_b = Netplayer::Player(if tag.0 { 2 } else { 1 });
    let (Some(player_a), Some(player_b)) = (report.player_at_pos(player_a), report.player_at_pos(player_b)) else { return };
    post_form(&configuration, build_form(player_a, player_b, report, &configuration));
}

pub fn build_report(room: &Room, score: &Score, first_attack: &FirstAttack, start_time: &StartTime, tag: TagFlag) -> Report {
    let winners = score.winners.iter().map(|winner| winner.map(|winner| team_of(tag.0, winner))).collect::<Vec<Option<u8>>>();
    let mut players = Slab::new();
    for (_index, player) in room.players.iter() {
        let Some(netplayer) = player.states.get::<Netplayer>().copied() else { continue };
        let team = team_of(tag.0, netplayer);
        players.insert(ReportPlayer {
            pos: netplayer,
            name: player.states.get::<String>().cloned().unwrap_or_default(),
            origin_name: player.states.get::<OriginName>().map(|origin_name| origin_name.0.clone()).unwrap_or_default(),
            score: winners.iter().filter(|winner| **winner == Some(team)).count(),
            decks: player.states.get::<Vec<Deck>>().cloned().unwrap_or_default(),
            ip: get_ip(player),
            first: first_attack.first_attack.iter().map(|attacker| team_of(tag.0, *attacker) == team).collect(),
            wins: winners.iter().map(|winner| *winner == Some(team)).collect(),
        });
    }
    Report {
        players,
        replays: Vec::new(),
        start_time: start_time.start,
        end_time: Local::now(),
        name: room.name.clone(),
    }
}

pub fn post_form(configuration: &Configuration, form: Vec<(&'static str, String)>) {
    if configuration.url.is_empty() { return }
    let url = configuration.url.clone();
    tokio::task::spawn_blocking(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).build().into();
        let body = serde_urlencoded::to_string(&form).unwrap_or_default();
        match agent.post(&url).content_type("application/x-www-form-urlencoded").send(body) {
            Ok(response) => log::info!("arena report posted: {}", response.status()),
            Err(error) => log::warn!("arena report post failed: {}", error),
        }
    });
}
fn team_of(tag: bool, pos: Netplayer) -> u8 {
    let index = u8::from(pos);
    if tag { index / 2 } else { index }
}

pub fn build_form(player_a: &ReportPlayer, player_b: &ReportPlayer, report: &Report, configuration: &Configuration) -> Vec<(&'static str, String)> {
    let first: Vec<String> = player_a.first.iter().zip(&player_b.first).map(|(first_a, first_b)| {
        if *first_a { player_a.origin_name.clone() } else if *first_b { player_b.origin_name.clone() } else { String::new() }
    }).collect();
    let wins: Vec<String> = player_a.wins.iter().zip(&player_b.wins).map(|(win_a, win_b)| {
        if *win_a { player_a.origin_name.clone() } else if *win_b { player_b.origin_name.clone() } else { String::new() }
    }).collect();
    let replays: Vec<String> = report.replays.iter().map(|replay| STANDARD.encode(&replay.data)).collect();
    vec![
        ("accesskey", configuration.access_key.clone()),
        ("usernameA", player_a.origin_name.to_owned()),
        ("usernameB", player_b.origin_name.to_owned()),
        ("userscoreA", player_a.score.to_string()),
        ("userscoreB", player_b.score.to_string()),
        ("userdeckA", deck_text(player_a)),
        ("userdeckB", deck_text(player_b)),
        ("userdeckAHistory", deck_history(player_a)),
        ("userdeckBHistory", deck_history(player_b)),
        ("first", serde_json::to_string(&first).unwrap_or_default()),
        ("wins", serde_json::to_string(&wins).unwrap_or_default()),
        ("replays", serde_json::to_string(&replays).unwrap_or_default()),
        ("start", report.start_time.to_rfc3339()),
        ("end", report.end_time.to_rfc3339()),
        ("arena", configuration.arena.clone()),
        ("nonce", rand::random::<f64>().to_string()),
    ]
}

fn deck_text(player: &ReportPlayer) -> String {
    player.decks.first().map(Deck::to_string).unwrap_or_default()
}

fn deck_history(player: &ReportPlayer) -> String {
    serde_json::to_string(&player.decks.iter().map(Deck::to_string).collect::<Vec<String>>()).unwrap_or_default()
}

