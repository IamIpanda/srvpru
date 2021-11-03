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

use crate::srvpru::Room;
use crate::ygopro::Colors;
use crate::ygopro::Netplayer;
use crate::ygopro::message::ctos;
use crate::ygopro::message::gm;
use crate::ygopro::message::srvpru;
use crate::ygopro::message::Direction;

use crate::srvpru::Handler;
use crate::srvpru::generate_chat;

room_attach! {
    countdown: Option<JoinHandle<()>>,
    tournament_state: TournamentState
}

export_room_attach_as!(get_tournament);

pub fn init() -> anyhow::Result<()> {
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
    Handler::follow_message::<ctos::HsStart, _>(100, "tournament_duel_start", |context, _| Box::pin(async move {
        let mut attachment = get_room_attachment_sure(context);
        let room = context.get_room().ok_or(anyhow!("Can't find room."))?;
        attachment.countdown = Some(tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(2400)).await;
            let _room = room.lock();
            if let Some(attachment) = ROOM_ATTACHMENTS.write().get_mut(&_room.name) {
                attachment.countdown = None;
                attachment.tournament_state = TournamentState::Death(4);
                _room.send(&generate_chat("{death_start}", Colors::Red, "zh-cn")).await;
            }
        }));
        Ok(false)
    })).register();

    Handler::follow_message::<gm::NewTurn, _>(100, "tournament_death_move", |context, _| Box::pin(async move {
        let mut attachment = get_room_attachment_sure(context);
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
    })).register();

    Handler::follow_message::<gm::Lpupdate, _>(100, "tournament_sudden_death", |context, _| Box::pin(async move {
        let attachment = get_room_attachment_sure(context);
        if attachment.tournament_state == TournamentState::Sudden {
            let room = context.get_room().ok_or(anyhow!("Cannot get the room"))?;
            room.lock().decide_result_by_lp().await?; 
        }
        Ok(false)
    })).register();

    Handler::follow_message::<srvpru::RoomDestroy, _>(100, "tournament_room_attachment_dropper", |_, request| Box::pin(async move {
        let attachment = drop_room_attachment(request);
        if let Some(attachment) = attachment {
            if let Some(countdown) = attachment.countdown {
                countdown.abort();
            }
        }
        Ok(false)
    })).register();

    Handler::register_handlers("tournament", Direction::CTOS, vec!["tournament_duel_start"]);
    Handler::register_handlers("tournament", Direction::STOC, vec!["tournament_death_move", "tournament_sudden_death"]);
    Handler::register_handlers("tournament", Direction::SRVPRU, vec!["tournament_room_attachment_dropper"]);
}

impl Room {
    async fn decide_result_by_lp(&self) -> anyhow::Result<bool> {
        let players = self.get_players_in_hashmap();
        let mut player1 = players.get(&Netplayer::Player1).ok_or(anyhow!("Can't find Player1"))?.lock();
        let mut player2 = players.get(&Netplayer::Player2).ok_or(anyhow!("Can't find Player2"))?.lock();
        let player1_lp = player1.get_lp();
        let player2_lp = player2.get_lp();
        if player1_lp < player2_lp {
            player1.send_to_server(&ctos::Surrender {}).await?;
            Ok(true)
        }
        else if player1_lp > player2_lp {
            player2.send_to_server(&ctos::Surrender {}).await?;
            Ok(true)
        }
        else {
            return Ok(false)
        }
    }
}