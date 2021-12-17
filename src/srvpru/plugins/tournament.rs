// ============================================================
// tournament
// ------------------------------------------------------------
//! Limit a match to target time, and step in death 3 turn 
//! when timeout.
//! 
//! dependecy:
//! - [lp_recorder](super::lp_recorder)
// ============================================================

use tokio::task::JoinHandle;

use crate::ygopro::Colors;
use crate::ygopro::Netplayer;
use crate::ygopro::message::ctos;
use crate::ygopro::message::gm;
use crate::ygopro::message::srvpru;

use crate::srvpru::CommonError;
use crate::srvpru::Room;
use crate::srvpru::Handler;
use crate::srvpru::generate_chat;
use crate::srvpru::plugins::base::api::register_api;

set_configuration! {
    #[serde(default = "default_round_time")]
    round_time: u64
}

fn default_round_time() -> u64 { 40 }

room_attach! {
    countdown: Option<JoinHandle<()>>,
    tournament_state: TournamentState
}

export_room_attach_as!(get_tournament);

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum TournamentState {
    Duel,
    Death(u8),
    Sudden
}

impl std::default::Default for TournamentState {
    fn default() -> Self {
        return TournamentState::Duel;
    }
}

fn register_handlers() { 
    Handler::before_message::<ctos::HsStart, _>(100, "tournament_duel_start", |context, _| Box::pin(async move {
        let mut attachment = get_room_attachment_sure(context)?;
        let room = context.get_room().ok_or(CommonError::RoomNotExist)?.clone();
        let time = get_configuration().round_time * 60;
        if time <= 0 { return Ok(false) }
        attachment.countdown = Some(tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(time)).await;
            let _room = room.lock();
            if let Some(attachment) = ROOM_ATTACHMENTS.write().get_mut(&_room.name) {
                attachment.countdown = None;
                attachment.tournament_state = TournamentState::Death(4);
                _room.send(&generate_chat("{death_start}", Colors::Red, "zh-cn")).await;
            }
        }));
        Ok(false)
    })).register_for_plugin("tournament");

    Handler::before_message::<gm::NewTurn, _>(100, "tournament_death_move", |context, _| Box::pin(async move {
        if context.get_position() != Netplayer::Player1 { return Ok(false) }
        let mut attachment = get_room_attachment_sure(context)?;
        if let TournamentState::Death(remain_turn) = attachment.tournament_state {
            if remain_turn - 1 <= 0 {
                let room = context.get_room().ok_or(anyhow!("Cannot get the room"))?; 
                if room.lock().decide_result_by_lp().await? == false {
                    context.send_to_room(&generate_chat("{death_start_final}", Colors::Red, context.get_region())).await.ok();
                    attachment.tournament_state = TournamentState::Sudden;
                }
            }
            else {
                attachment.tournament_state = TournamentState::Death(remain_turn - 1);
                context.send_to_room(&generate_chat(&format!("{{death_remain_part1}} {} {{death_remain_part2}}", remain_turn - 1), Colors::Red, context.get_region())).await.ok();
            }
        }
        Ok(false)
    })).register_for_plugin("tournament");

    Handler::before_message::<srvpru::LpChange, _>(100, "tournament_sudden_death", |context, _| Box::pin(async move {
        let in_sudden_death = get_room_attachment_sure(context)?.tournament_state == TournamentState::Sudden;
        if in_sudden_death {
            let room = context.get_room().ok_or(anyhow!("Cannot get the room"))?;
            room.lock().decide_result_by_lp().await?; 
        }
        Ok(false)
    })).register_for_plugin("tournament");

    Handler::before_message::<srvpru::RoomDestroy, _>(100, "tournament_room_attachment_dropper", |_, message| Box::pin(async move {
        let attachment = drop_room_attachment(message);
        if let Some(attachment) = attachment {
            if let Some(countdown) = attachment.countdown {
                countdown.abort();
            }
        }
        Ok(false)
    })).register_for_plugin("tournament");

    register_api(|router| router.route("/tournament/death", axum::routing::post(get_into_death)));
}

async fn get_into_death() {
    for (_, room) in crate::srvpru::ROOMS.read().iter() {
        let _room = room.lock();
        match _room.get_tournament() {
            Some(attachment) => attachment,
            None => return,
        }.tournament_state = TournamentState::Death(4);
        _room.send(&generate_chat("{death_start}", Colors::Red, "zh-cn")).await;
    }
}

impl Room {
    async fn decide_result_by_lp(&self) -> anyhow::Result<bool> {
        let players = self.get_players_in_hashmap();
        let mut player1 = players.get(&Netplayer::Player1).ok_or(CommonError::PlayerNotExist)?.lock();
        let mut player2 = players.get(&Netplayer::Player2).ok_or(CommonError::PlayerNotExist)?.lock();
        let player1_lp = player1.get_lp();
        let player2_lp = player2.get_lp();
        if player1_lp < player2_lp {
            player1.send_to_server(&ctos::Surrender {}).await?;
            // TODO: Change to match kill here
            player1.expel();
            Ok(true)
        }
        else if player1_lp > player2_lp {
            player2.send_to_server(&ctos::Surrender {}).await?;
            player2.expel();
            Ok(true)
        }
        else {
            return Ok(false)
        }
    }
}