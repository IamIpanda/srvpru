#![allow(dead_code)]
// ============================================================
// result_report
// ------------------------------------------------------------
//! After a match finish, send a report to target endpoint.
//! 
//! Dependecy:
//! - position_recorder
// ============================================================

use std::vec;
use num_enum::IntoPrimitive;

use crate::srvpru::Handler;
use crate::srvpru::structs::PlayerDestroy;
use crate::srvpru::structs::RoomDestroy;
use crate::ygopro::message::Direction;
use crate::ygopro::message::GMWin;
use crate::ygopro::constants::Netplayer;
use crate::ygopro::Deck;

#[derive(Copy, Clone, Eq, PartialEq, Debug, IntoPrimitive)]
#[repr(i8)]
pub enum MatchScore {
    NotStarted = -5,
    Dropped = -9,
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3
}

impl MatchScore {
    fn step(&mut self) {
        *self = match self {
            MatchScore::Zero => MatchScore::One,
            MatchScore::One => MatchScore::Two,
            MatchScore::Two => MatchScore::Three,
            _ => *self
        }
    }
}

impl std::default::Default for MatchScore {
    fn default() -> Self {
        return MatchScore::NotStarted;
    }
}

#[derive(Default, Debug)]
pub struct PlayerMatchResult {
    name: String,
    score: MatchScore,
    deck: String
}

set_configuration! {
    report_endpoint: String,
    access_key: String,
    arena: String
}

room_attach! {
    player_a_result: PlayerMatchResult,
    player_b_result: PlayerMatchResult,
    start_time: i64
}

struct MatchResultReport {
    access_key: String,
    username_a: String,
    username_b: String,
    user_score_a: MatchScore,
    user_score_b: MatchScore,
    //user_deck_a: Deck,
    //user_deck_b: Deck,
    first: Vec<String>,
    replays: Vec<String>,
    start: i64,
    end: i64,
    arena: String
}

impl MatchResultReport {
    pub fn to_form(self) -> Vec<(&'static str, String)> {
        vec![
            ("accessKey", self.access_key),
            ("usernameA", self.username_a),
            ("usernameB", self.username_b),
            ("user_scoreA", (self.user_score_a as i8).to_string()),
            ("user_scoreB", (self.user_score_b as i8).to_string()),
            ("first", serde_json::to_string(&self.first).unwrap()),
            ("replays", "".to_string()),
            ("start", self.start.to_string()),
            ("end", self.end.to_string()),
            ("arena", self.arena)
        ]
    }
}

pub fn register_handlers() {
    Handler::follow_message::<RoomDestroy, _>(100, "match_result_sender", |context, _| Box::pin(async move {
        let configuration = get_configuration();
        let attachment = get_room_attachment_sure(context);
        let report = MatchResultReport {
            access_key: configuration.access_key.clone(),
            username_a: attachment.player_a_result.name.clone(),
            username_b: attachment.player_b_result.name.clone(),
            user_score_a: attachment.player_a_result.score,
            user_score_b: attachment.player_b_result.score,
            first: vec![],
            replays: vec![], 
            start: attachment.start_time,
            end: chrono::offset::Local::now().timestamp(),
            arena: configuration.access_key.clone(),
        };
        let client = reqwest::Client::new();
        client.post(&configuration.report_endpoint).form(&report.to_form()).send().await?;
        Ok(false)
    })).register();

    Handler::follow_message::<PlayerDestroy, _>(100, "match_result_player_drop_listener", |context, _| Box::pin(async move {
        let position = context.get_position().ok_or(anyhow!("Cannot determine use position."))?;
        let attachment = get_room_attachment(context);
        if attachment.is_none() { return Ok(false); }
        let mut attachment = attachment.unwrap();
        match position {
            Netplayer::Player1 => attachment.player_a_result.score = MatchScore::Dropped,
            Netplayer::Player2 => attachment.player_b_result.score = MatchScore::Dropped,
            _ => {}
        }
        Ok(false)
    })).register();

    Handler::follow_message::<GMWin, _>(100, "match_result_countor", |context, request| Box::pin(async move {
        let mut attachment = get_room_attachment_sure(context);
        match request.winner {
            Netplayer::Player1 => attachment.player_a_result.score.step(),
            Netplayer::Player2 => attachment.player_b_result.score.step(),
            _ => {}
        };
        Ok(false)
    })).register();

    Handler::register_handlers("result_report", Direction::SRVPRU, vec!["match_result_sender", "match_result_player_drop_listener"]);
    Handler::register_handlers("result_report", Direction::STOC, vec!["match_result_countor"])

}


pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}