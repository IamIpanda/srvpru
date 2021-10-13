// ============================================================
// reconnect
// ------------------------------------------------------------
//! Allow dropped user reconnect to game.
//! 
//! Need enable:
//! - stage_recorder
// ============================================================

use std::sync::Arc;

use tokio::task::JoinHandle;
use parking_lot::Mutex;
use anyhow::Result;

use crate::ygopro::message;
use crate::ygopro::message::Direction;
use crate::ygopro::message::MessageType;
use crate::ygopro::message::CTOSMessageType;
use crate::ygopro::message::STOCMessageType;
use crate::ygopro::message::CTOSJoinGame;
use crate::ygopro::constants::Colors;

use crate::srvpru::utils;
use crate::srvpru::structs;
use crate::srvpru::Context;
use crate::srvpru::Handler;
use crate::srvpru::Room;
use crate::srvpru::Player;
use crate::srvpru::plugins::stage_recorder;
use crate::srvpru::plugins::stage_recorder::DuelStage;
use crate::srvpru::server;

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}

set_configuration! {
    timeout: u64,
    #[serde(default)]
    can_reconnect_by_kick: bool
}

room_attach! {
    dropped_player: Option<Arc<Mutex<Player>>>
}

player_attach! {
    used_deck: Vec<u8>,
    countdown: Option<JoinHandle<()>>,
    reconnecting: ReconnectStatus
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
    srvpru_handler!(0, structs::PlayerDestroy, get_player_attachment_sure, |context, request| {
        // This user is already in a countdown (means timeout)
        if attachment.countdown.is_some() { return Ok(false); }
        // Duel is in prepare or already finished
        let duel_stage = &stage_recorder::get_room_attachment_sure(context).duel_stage;
        if duel_stage == &DuelStage::Begin || duel_stage == &DuelStage::End { return Ok(false); }
        // Already have a user drop. Drop that player as normal. 
        // (And will cause game end => room drop => player arc release)
        let mut room_attach = get_room_attachment_sure(context);
        if room_attach.dropped_player.is_some() { return Ok(false); }
        // Move player to waiting status
        room_attach.dropped_player = Some(request.player.clone()); 
        attachment.reconnecting = ReconnectStatus::Dropped; 
        // Count down on timeout
        let _request = (*request).clone();
        let addr = context.addr.clone();
        attachment.countdown = Some(tokio::spawn(async move {
            let configuration = get_configuration();
            tokio::time::sleep(tokio::time::Duration::from_millis(configuration.timeout)).await;
            // real drop
            crate::srvpru::server::trigger_internal(addr, _request);
        }));
        // Send hint
        let name = request.player.lock().name.clone();
        context.send_chat_to_room(&format!("{} {{disconnect_from_game}}", name), Colors::Babyblue).await?;
        return context.block_message();
    }).register_as("reconnect_player_destroy_interceptor");

    srvpru_handler!(7, CTOSJoinGame, |context, request| {
        // When dropper rejoin the game, there will be two players:
        // The former player contains in room.dropped_player (a)
        // The new player join with a new addr, which is still a player precursor (b)
        // We temp upgrade the (b) to player, and after success, move (b) to (a) place.
        // and trigger a move event to make (a) move to (b).
        // player (b) will not own any attachment on reconenct plugin.
        let configuration = get_configuration();
        // Search room by name, as room is not built now.
        let room_name = context.get_string(&request.pass, "pass")?;
        let room = Room::find_room_by_name(&room_name);
        if room.is_none() { return Ok(false); }
        // get attachment
        let mut room_attachments = ROOM_ATTACHMENTS.write();
        let room_attachment = room_attachments.get_mut(room_name);
        if room_attachment.is_none() { return Ok(false); }
        let room_attachment = room_attachment.unwrap();
        if room_attachment.dropped_player.is_none() {
            if configuration.can_reconnect_by_kick { return Ok(false); }
            else { return Ok(false); }
        }
        // Lock actual room and player to send resposne
        let room_mutex = room.unwrap();
        let player = room_attachment.dropped_player.as_mut(); 
        let player_mutex = player.unwrap();
        {
            let mut _room = room_mutex.lock();
            let _player = player_mutex.lock();
            // Don't need to actually start a server as there already is one.
            // Just send a message to make user 'see' a room.
            context.send(&message::STOCJoinGame { info: _room.host_info.clone() }).await?;
            context.send(&message::STOCTypeChange { _type: 16 }).await?;
            context.send(&message::STOCHsPlayerEnter { name: message::cast_to_array(&_player.name), pos: 0 }).await?;
            context.send_chat("{pre_reconnecting_to_room}", Colors::Babyblue).await?;
            // Change stage. Mark it on player (a) attachment.
            let mut player_attachements = PLAYER_ATTACHMENTS.write();
            let mut attachment = player_attachements.get_mut(&_player.client_addr).unwrap();
            attachment.reconnecting = ReconnectStatus::Prepare;
        }
        // Temp add player (b) to room, then update deck can find it.
        // Now room contains player(a) and player(b) at the same time.
        // Don't send precursor loaded messages, so discard it here.
        {
            let mut player_precursors = crate::srvpru::player::PLAYER_PRECURSORS.write();
            let mut players = crate::srvpru::player::PLAYERS.write();
            let mut rooms_by_client_addr = crate::srvpru::room::ROOMS_BY_CLIENT_ADDR.write();
            let mut room = room_mutex.lock();
            let player_precursor = player_precursors.remove(&context.addr).unwrap();
            let player = crate::srvpru::Player {
                room: room_mutex.clone(),
                name: player_precursor.name,
                client_addr: context.addr.clone(),
                client_stream_writer: context.socket.take(),
                server_stream_writer: None,
                reader_handler: tokio::spawn(async {}),
            };
            let player = Arc::new(Mutex::new(player));
            players.insert(context.addr.clone(), player.clone());
            rooms_by_client_addr.insert(context.addr.clone(), room_mutex.clone());
            room.players.push(player);
        }
        return context.block_message();
    }).register_as("reconnect_joingame_interceptor");

    srvpru_handler!(CTOSMessageType::UpdateDeck, get_room_attachment_sure, |context| {
        if let Some(player) = attachment.dropped_player.as_mut() {
            let addr = player.lock().client_addr.clone();
            let mut player_attachments = PLAYER_ATTACHMENTS.write();
            let mut attachment = player_attachments.remove(&addr).unwrap();
            let new_player = context.get_player().unwrap();
            match attachment.reconnecting {
                ReconnectStatus::Prepare => {
                    if attachment.used_deck == context.request_buffer {
                        // Pair success. start reconnect.
                        context.send_chat_to_room("{reconnecting_to_room}", Colors::Babyblue).await?;
                        // Stop count down.
                        if let Some(handle) = &attachment.countdown { handle.abort(); }
                        // Move player.
                        server::SOCKET_SERVER.get().unwrap().trigger_internal(&context.addr, structs::PlayerMove { post_player: player.clone(), new_player }).await?;
                        // Feed data.
                        reconnect(context, &mut attachment).await?;
                        // Here cannot release player attachements locker, so have to move the attachment iself
                        player_attachments.insert(context.addr.clone(), attachment);
                        return context.block_message();
                    } else {
                        // Above is Player (a).
                        // Acutal back client is in player (b).
                        let mut player = new_player.lock();
                        if let Some(socket) = player.client_stream_writer.as_mut() {
                            utils::send_chat(socket, "{reconnect_failed}", "zh-cn", Colors::Babyblue).await?;
                        }
                        return context.block_message();
                    }
                }
                _ => {
                }
            }
        }
        else {
            let mut player_attachement = get_player_attachment_sure(context);
            player_attachement.used_deck = context.request_buffer.to_vec();
        };
    }).register_as("reconnect_deck_recorder");

    srvpru_handler!(STOCMessageType::FieldFinish, get_player_attachment_sure, |context| {
        let mut room_attachment = get_room_attachment_sure(context);
        room_attachment.dropped_player = None;
        attachment.reconnecting = ReconnectStatus::Normal;
    }).register_as("reconnect_cleanup");

    srvpru_handler!(crate::srvpru::structs::PlayerDestroy, |_, request| {
        if let Some(_player_attachment) = drop_player_attachment(request) {
            if let Some(handle) = _player_attachment.countdown {
                handle.abort();
            }
        };
    }).register_as("reconnect_player_attachment_dropper");


    srvpru_handler!(CTOSMessageType::HsReady, get_player_attachment_sure, |context| {
        if attachment.reconnecting == ReconnectStatus::Recovering {
            context.response = None;
            return Ok(true);
        };
    }).register_as("reconnect_ready_stopper");

    register_room_attachement_dropper();

    Handler::register_handlers("reconnect", Direction::SRVPRU, vec!("reconnect_player_destroy_interceptor", "reconnect_room_attachment_dropper", "reconnect_player_attachment_dropper"));
    Handler::register_handlers("reconnect", Direction::CTOS, vec!("reconnect_deck_recorder", "reconnect_joingame_interceptor", "reconnect_ready_stopper"));
    Handler::register_handlers("reconnect", Direction::STOC, vec!("reconnect_cleanup"));
}

async fn reconnect<'a, 'b>(context: &'b mut Context<'a>, mut attachment: &mut PlayerAttachment) -> Result<()> {
    attachment.reconnecting = ReconnectStatus::Recovering;
    let duel_stage = &stage_recorder::get_room_attachment_sure(context).duel_stage;
    let player = context.get_player().ok_or(anyhow!("Cannot get player"))?;
    let mut player = player.lock();
    let message = match duel_stage {
        DuelStage::Dueling => {
            let socket = player.server_stream_writer.as_mut().ok_or(anyhow!("Server stream already taken."))?;
            utils::send_empty(socket, &MessageType::CTOS(CTOSMessageType::RequestField)).await?;
            return Ok(());
        },
        DuelStage::Finger => STOCMessageType::SelectHand,
        DuelStage::Firstgo => STOCMessageType::SelectTp,
        DuelStage::Siding => STOCMessageType::ChangeSide,
        _ => { warn!("try to reconnect in wrong status."); return Ok(()); }
    };
    let socket = player.client_stream_writer.as_mut().ok_or(anyhow!("Client stream already taken."))?;
    utils::send_empty(socket, &MessageType::STOC(STOCMessageType::DuelStart)).await?;
    utils::send_empty(socket, &MessageType::STOC(message)).await?;
    Ok(())
}

fn reconnect_by_kick() {
    
}

fn ip_equal(addr1: SocketAddr, addr2: SocketAddr) -> bool {
    match addr1 {
        SocketAddr::V4(_addr1) => match addr2 { SocketAddr::V4(_addr2) => _addr1.ip() == _addr2.ip(), _ => false },
        SocketAddr::V6(_addr1) => match addr2 { SocketAddr::V6(_addr2) => _addr1.ip() == _addr2.ip(), _ => false },
    }
}