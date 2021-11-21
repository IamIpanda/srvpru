// ============================================================
// reconnect
// ------------------------------------------------------------
//! Allow dropped user reconnect to game.
//! 
//! Need enable:
//! - [stage_recorder](super::stage_recorder)
//! 
//! **ATTENTION**  
//! Reconnect plugin must record the last stoc game message 
//! inner attachments, that will severely loss the performance.
// ============================================================

use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio::io::AsyncWriteExt;
use parking_lot::Mutex;
use anyhow::Result;

use crate::ygopro::message;
use crate::ygopro::message::Direction;
use crate::ygopro::message::MessageType;
use crate::ygopro::message::ctos;
use crate::ygopro::message::stoc;
use crate::ygopro::message::gm;
use crate::ygopro::message::srvpru;
use crate::ygopro::Colors;
use crate::ygopro::Netplayer;

use crate::srvpru::Context;
use crate::srvpru::Handler;
use crate::srvpru::Room;
use crate::srvpru::Player;
use crate::srvpru::plugins::stage_recorder;
use crate::srvpru::plugins::stage_recorder::DuelStage;
use crate::srvpru::generate_chat;
use crate::ygopro::message::stoc::FieldFinish;

fn default_timeout() -> u64 { 90 }

set_configuration! {
    #[serde(default = "default_timeout")]
    timeout: u64,
    #[serde(default)]
    can_reconnect_by_kick: bool
}

depend_on! {
    "stage_recorder"
}

room_attach! {
    dropped_player: Option<Arc<Mutex<Player>>>
}

player_attach! {
    used_deck: Vec<u8>,
    countdown: Option<JoinHandle<()>>,
    reconnecting: ReconnectStatus,
    last_game_message: Option<Vec<u8>>,
    last_hint_message: Option<gm::Hint>
}

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_dependency()?;
    register_handlers();
    Ok(())
}

#[derive(PartialEq, Eq, PartialOrd, Ord ,Debug)]
pub enum ReconnectStatus {
    Normal,
    Dropped,
    Prepare,
    Recovering,
}

impl std::default::Default for ReconnectStatus {
    fn default() -> Self {
        return ReconnectStatus::Normal;
    }
}

fn register_handlers() {
    // Stop player termination.
    Handler::before_message::<srvpru::PlayerDestroy, _>(0, "reconnect_player_destroy_interceptor", |context, request| Box::pin(async move {
        let player = request.player.clone();
        let mut player_attachment = get_player_attachment_sure(context);
        // This user is already in a countdown (means timeout)
        if player_attachment.countdown.is_some() { return Ok(false); }
        // Duel is in prepare or already finished
        let duel_stage = context.get_duel_stage();
        if duel_stage == DuelStage::Begin || duel_stage == DuelStage::End { return Ok(false); }
        // Already have a user drop. Drop that player as normal. 
        // (And will cause game end => room drop => player arc release)
        let mut room_attachment = unwrap_or_return!(get_room_attachment(context));
        if let Some(dropped_player) = room_attachment.dropped_player.take() {
            // The second user drops.
            {
                // Make now dropping player drop, that player fail the game.
                request.player.lock().server_stream_writer.take();
            }
            let addr = dropped_player.lock().client_addr;
            crate::srvpru::trigger_internal_async(addr, srvpru::PlayerDestroy { player: dropped_player });
            return Ok(false); 
        }
        // Move player to waiting status
        room_attachment.dropped_player = Some(request.player.clone()); 
        player_attachment.reconnecting = ReconnectStatus::Dropped; 
        // Count down on timeout
        let countdown_player = player.clone();
        let addr = *context.addr;
        player_attachment.countdown = Some(tokio::spawn(async move {
            let configuration = get_configuration();
            tokio::time::sleep(tokio::time::Duration::from_millis(configuration.timeout)).await;
            crate::srvpru::trigger_internal_async(addr, srvpru::PlayerDestroy { player: countdown_player });
        }));
        // Send hint
        let (name, region) = {
            let _player = player.lock();
            (_player.name.clone(), _player.region)
        };
        context.send_to_room(&generate_chat(&format!("{} {{disconnect_from_game}}", name), Colors::Babyblue, region)).await?;
        return context.block_message();
    })).register();

    Handler::before_message::<ctos::JoinGame, _>(7, "reconnect_joingame_interceptor", |context, request| Box::pin(async move {
        // When dropper rejoin the game, there will be two players:
        // The former player contains in room.dropped_player (a)
        // The new player join with a new addr, which is still a player precursor (b)
        // We temp upgrade the (b) to player, and after success, move (b) to (a) place.
        // and trigger a move event to make (a) move to (b).
        // player (b) will not own any attachment on reconenct plugin.
        let configuration = get_configuration();
        // Search room by name, as room is not built now.
        let room_name = context.get_string(&request.pass, "pass")?;
        let room = crate::unwrap_or_return!(Room::get_room(&room_name));
        // check if can reconnect
        let mut room_attachment = crate::unwrap_or_return!(get_attachment_by_name(context, request));
        let dropped_player = crate::unwrap_or_return!(room_attachment.dropped_player.as_mut());
        if ! ip_equal(dropped_player.lock().client_addr, *context.addr) {
            if configuration.can_reconnect_by_kick { return Ok(false) }
            else { return Ok(false) }
        }
        // Lock actual room and player to send resposne
        {
            let mut _room = room.lock();
            let _dropped_player = dropped_player.lock();
            // Don't need to actually start a server as there already is one.
            // Just send a message to make user 'see' a room.
            context.send(&struct_sequence![
                message::stoc::JoinGame { info: _room.host_info.clone() },
                message::stoc::TypeChange { _type: 1 },
                message::stoc::HsPlayerEnter { name: message::string::cast_to_fix_length_array("********"), pos: Netplayer::Player1 }, 
                message::stoc::HsPlayerEnter { name: message::string::cast_to_fix_length_array(&_dropped_player.name), pos: Netplayer::Player2 },
                generate_chat("{pre_reconnecting_to_room}", Colors::Babyblue, context.get_region())
            ]).await?;
            // Change stage. Mark it on player (a) attachment.
            PLAYER_ATTACHMENTS.write().get_mut(&_dropped_player.client_addr).map(|attachment| attachment.reconnecting = ReconnectStatus::Prepare);
        }
        // Temp add player (b) to room, then update deck can find it.
        // Now room contains player(a) and player(b) at the same time.
        // Don't need to send precursor loaded messages, so discard it here.
        {
            let (mut player, _) = crate::srvpru::PlayerPrecursor::upgrade(*context.addr, room.clone()).ok_or(anyhow!("Cannot find precursor"))?;
            player.client_stream_writer = context.socket.take();
            let player = Arc::new(Mutex::new(player));
            crate::srvpru::player::PLAYERS.write().insert(*context.addr, player.clone());
            crate::srvpru::room::ROOMS_BY_CLIENT_ADDR.write().insert(*context.addr, room.clone());
            room.lock().players.push(player);
        }
        return context.block_message();
    })).register();

    srvpru_handler!(ctos::MessageType::UpdateDeck, get_room_attachment_sure, |context| {
        if let Some(player) = attachment.dropped_player.as_mut() {
            let addr = player.lock().client_addr;
            let mut attachment = PLAYER_ATTACHMENTS.write().remove(&addr).ok_or(anyhow!("Can't get room attachment."))?;
            let new_player = context.get_player().ok_or(anyhow!("Can't get player"))?;
            if attachment.reconnecting == ReconnectStatus::Prepare {
                if attachment.used_deck == context.request_buffer {
                    // Pair success. start reconnect.
                    context.send_back(&generate_chat("{reconnecting_to_room}", Colors::Babyblue, context.get_region())).await?;
                    // Stop count down.
                    if let Some(handle) = &attachment.countdown { handle.abort(); attachment.countdown = None; }
                    // Move player.
                    crate::srvpru::server::trigger_internal(*context.addr, srvpru::PlayerMove { post_player: player.clone(), new_player }).await;
                    // Here cannot release player attachements locker, so have to move the attachment iself                
                    attachment.reconnecting = ReconnectStatus::Recovering;
                    PLAYER_ATTACHMENTS.write().insert(*context.addr, attachment);
                    // Feed data.
                    reconnect(context).await?;
                    return context.block_message();
                } else {
                    // Above is Player (a).
                    // Acutal back client is in player (b).
                    new_player.lock().send_to_client(&generate_chat("{reconnect_failed}", Colors::Babyblue, context.get_region())).await?;
                    return context.block_message();
                }
            }
        }
        else {
            let mut player_attachement = get_player_attachment_sure(context);
            player_attachement.used_deck = context.request_buffer.to_vec();
        };
    }).register_as("reconnect_deck_recorder");

    Handler::follow_message(100, "reconnect_cleanup", |context, _: &FieldFinish| Box::pin(async move {
        let mut attachment = get_player_attachment_sure(context);
        let mut room_attachment = get_room_attachment_sure(context);
        room_attachment.dropped_player = None;
        attachment.reconnecting = ReconnectStatus::Normal;
        if let Some(last_hint) = attachment.last_hint_message.as_ref() {
            context.send(last_hint).await.ok();
        }
        if let Some(last_gm) = attachment.last_game_message.as_ref() {
            if let Some(socket) = context.socket {
                socket.write_all(last_gm).await.ok();
            }
        };
        Ok(false)
    })).register();

    srvpru_handler!(srvpru::PlayerDestroy, |_, request| {
        if let Some(_player_attachment) = drop_player_attachment(request) {
            if let Some(handle) = _player_attachment.countdown {
                handle.abort();
            }
        };
    }).register_as("reconnect_player_attachment_dropper");


    srvpru_handler!(ctos::MessageType::HsReady, get_player_attachment_sure, |context| {
        if attachment.reconnecting == ReconnectStatus::Recovering {
            return context.block_message();
        };
    }).register_as("reconnect_ready_stopper");

    srvpru_handler!(stoc::GameMessage, get_player_attachment, |context, request| {
        if let Some(mut attachment) = attachment {
            if attachment.reconnecting == ReconnectStatus::Normal && request.kind != gm::MessageType::Retry {
                attachment.last_game_message = Some(context.request_buffer.to_vec());
            }
        };
    }).register_as("reconnect_gm_recorder");

    srvpru_handler!(gm::Hint, get_player_attachment_sure, |context, request| {
        if request._type == crate::ygopro::Hint::SelectMessage {
            attachment.last_hint_message = Some(request.clone());
        };
    }).register_as("reconnect_hint_recorder");

    register_room_attachement_dropper();
    Handler::register_handlers("reconnect", Direction::SRVPRU, vec!("reconnect_player_destroy_interceptor", "reconnect_player_attachment_dropper"));
    Handler::register_handlers("reconnect", Direction::CTOS, vec!("reconnect_deck_recorder", "reconnect_joingame_interceptor", "reconnect_ready_stopper", "reconnect_hint_recorder"));
    Handler::register_handlers("reconnect", Direction::STOC, vec!("reconnect_cleanup", "reconnect_gm_recorder"));
}

async fn reconnect<'a, 'b>(context: &'b mut Context<'a>) -> Result<()> {
    let duel_stage = &stage_recorder::get_room_attachment_sure(context).duel_stage;
    let player = context.get_player().ok_or(anyhow!("Cannot get player"))?;
    let mut _player = player.lock();
    let message = match duel_stage {
        DuelStage::Dueling => {
            _player.send_to_server(&ctos::RequestField{}).await?;
            return Ok(());
        },
        DuelStage::Finger  => stoc::MessageType::SelectHand,
        DuelStage::Firstgo => stoc::MessageType::SelectTp,
        DuelStage::Siding  => stoc::MessageType::ChangeSide,
        _ => { warn!("try to reconnect in wrong status."); return Ok(()); }
    };
    _player.send_to_client(&stoc::DuelStart{}).await?;
    let socket = _player.client_stream_writer.as_mut().ok_or(anyhow!("Socket already taken."))?;
    crate::srvpru::send_raw_data(socket, MessageType::STOC(message), &[]).await?;
    Ok(())
}

fn ip_equal(addr1: SocketAddr, addr2: SocketAddr) -> bool {
    match addr1 {
        SocketAddr::V4(_addr1) => match addr2 { SocketAddr::V4(_addr2) => _addr1.ip() == _addr2.ip(), _ => false },
        SocketAddr::V6(_addr1) => match addr2 { SocketAddr::V6(_addr2) => _addr1.ip() == _addr2.ip(), _ => false },
    }
}