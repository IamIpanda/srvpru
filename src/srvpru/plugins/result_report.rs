// ============================================================
// result_report
// ------------------------------------------------------------
//! After a match finish, send a report to target endpoint.
//! 
//! Dependency:
//! - [position_recorder](super::position_recorder)
//! - [deck_recorder](super::deck_recorder)
// ============================================================

use std::vec;
use num_enum::IntoPrimitive;

use crate::srvpru::Handler;
use crate::unwrap_or_return;
use crate::ygopro::data::Deck;
use crate::ygopro::message::srvpru::PlayerDestroy;
use crate::ygopro::message::srvpru::RoomDestroy;
use crate::ygopro::message::Direction;
use crate::ygopro::message::gm::Win;
use crate::ygopro::message::gm::Start;
use crate::ygopro::Netplayer;

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}

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

#[derive(Default, Debug)]
pub struct PlayerMatchResult {
    score: MatchScore,
}

set_configuration! {
    report_endpoint: String,
    access_key: String,
    arena: String
}

room_attach! {
    player_a_result: PlayerMatchResult,
    player_b_result: PlayerMatchResult,
    first: Vec<String>,
    start_time: i64
}

struct MatchResultReport {
    access_key: String,
    user_a_name: String,
    user_b_name: String,
    user_a_score: MatchScore,
    user_b_score: MatchScore,
    user_a_deck: Deck,
    user_b_deck: Deck,
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
            ("usernameA", self.user_a_name),
            ("usernameB", self.user_b_name),
            ("userscoreA", (self.user_a_score as i8).to_string()),
            ("userscoreB", (self.user_b_score as i8).to_string()),
            ("userdeckA", self.user_a_deck.to_string()),
            ("userdeckB", self.user_b_deck.to_string()),
            ("first", serde_json::to_string(&self.first).unwrap_or("[]".to_string())),
            ("replays", "".to_string()),
            ("start", self.start.to_string()),
            ("end", self.end.to_string()),
            ("arena", self.arena)
        ]
    }
}

static REQWEST_CLIENT: once_cell::sync::OnceCell<reqwest::Client> = once_cell::sync::OnceCell::new();
pub fn register_handlers() {
    Handler::before_message::<RoomDestroy, _>(95, "match_result_sender", |context, _| Box::pin(async move {
        let configuration = get_configuration();
        let attachment = get_room_attachment_sure(context);
        let players = context.get_room().ok_or(anyhow!("Cannot get room"))?.lock().get_players_in_hashmap();
        let player_a = unwrap_or_return!(players.get(&Netplayer::Player1));
        let player_b = unwrap_or_return!(players.get(&Netplayer::Player2));
        let _player_a = player_a.lock();
        let _player_b = player_b.lock();
        let report = MatchResultReport {
            access_key: configuration.access_key.clone(),
            user_a_name: _player_a.name.clone(),
            user_b_name: _player_b.name.clone(),
            user_a_score: attachment.player_a_result.score,
            user_b_score: attachment.player_b_result.score,
            user_a_deck: _player_a.get_deck().map(|attachment| attachment.start_deck.clone()).unwrap_or_default(),
            user_b_deck: _player_b.get_deck().map(|attachment| attachment.start_deck.clone()).unwrap_or_default(),
            first: Vec::new(),
            replays: vec![], 
            start: attachment.start_time,
            end: chrono::offset::Local::now().timestamp(),
            arena: configuration.access_key.clone(),
        };
        REQWEST_CLIENT.get_or_init(|| reqwest::Client::new()).post(&configuration.report_endpoint).form(&report.to_form()).send().await.ok();
        Ok(false)
    })).register();

    Handler::before_message::<PlayerDestroy, _>(100, "match_result_player_drop_listener", |context, _| Box::pin(async move {
        let position = context.get_position();
        if let Some(mut attachment) = get_room_attachment(context) {
            match position {
                Netplayer::Player1 => attachment.player_a_result.score = MatchScore::Dropped,
                Netplayer::Player2 => attachment.player_b_result.score = MatchScore::Dropped,
                _ => {}
            }
        }
        Ok(false)
    })).register();

    Handler::before_message::<Win, _>(100, "match_result_countor", |context, request| Box::pin(async move {
        let mut attachment = get_room_attachment_sure(context);
        match request.winner {
            Netplayer::Player1 => attachment.player_a_result.score.step(),
            Netplayer::Player2 => attachment.player_b_result.score.step(),
            _ => {}
        };
        Ok(false)
    })).register();

    Handler::before_message::<Start, _>(100, "match_result_first_recorder", |context, request| Box::pin(async move {
        if request._type & 0xf > 0 {
            let mut attachment = get_room_attachment_sure(context);
            attachment.first.push(context.get_player().ok_or(anyhow!("Cannot get player"))?.lock().name.clone());
        }
        Ok(false)
    })).register();
    
    register_room_attachement_dropper();
    Handler::register_handlers("result_report", Direction::SRVPRU, vec!["match_result_sender", "match_result_player_drop_listener"]);
    Handler::register_handlers("result_report", Direction::STOC, vec!["match_result_countor", "match_result_first_recorder"]);
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
