// ============================================================
// telescreen
// ------------------------------------------------------------
//! Insert a telescreen user in any duel,
//! it will record all the message received.  
//! Offer a half-way observer for outside. 
//! 
//! Must enable:
//! - version_checker
//! - stage_recorder
// ============================================================

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::io::AsyncReadExt;
use parking_lot::Mutex;

use crate::srvpru::player;
use crate::srvpru::Player;
use crate::srvpru::structs;
use crate::srvpru::processor::Handler;
use crate::srvpru::plugins::stage_recorder;
use crate::srvpru::plugins::version_checker;

use crate::ygopro::constants::Colors;
use crate::ygopro::message::CTOSChat;
use crate::ygopro::message::CTOSJoinGame;
use crate::ygopro::message::CTOSMessageType;
use crate::ygopro::message::Direction;
use crate::ygopro::message::MessageType;
use crate::ygopro::message::cast_to_array;
use crate::ygopro::message::wrap_struct;
use crate::ygopro::message::wrap_mapped_struct;
use crate::ygopro::message;

#[derive(Default, Debug)]
pub struct Telescreen {
    listener: Option<JoinHandle<()>>,
    buffer: Vec<Vec<u8>>,
    watchers: Vec<Player>,
}

room_attach! {
    pointer: Arc<Mutex<Telescreen>>
}

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())    
}

fn register_handlers() {
    srvpru_handler!(structs::RoomCreated, get_room_attachment_sure, |context, _| {
        let room = context.get_room().ok_or(anyhow!("Cannot get room"))?;
        let addr = room.lock().server_addr.clone().ok_or(anyhow!("Server haven't spawn."))?;
        // Room create and player 1 join room are concurrent.
        // So it may happen: Telescreen join the game faster than player1, and become the host.
        if room.lock().players.len() == 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
            if room.lock().players.len() == 0 {
                return Err(anyhow!("Player 1 don't join game on time."));
            }
        }
        register_telescreen(addr, attachment.pointer.clone()).await?;
    }).register_as("telescreen_injector");

    Handler::follow_message::<CTOSJoinGame, _>(8, "telescreen_watcher", |context, request| Box::pin(async move {
        let stage_recorder = stage_recorder::get_attachment_by_name(request);
        if stage_recorder.is_none() { return Ok(false) }
        let stage_recorder = stage_recorder.unwrap();
        if stage_recorder.duel_stage <= stage_recorder::DuelStage::Begin { return Ok(false) };

        let attachment = get_attachment_by_name(&request).unwrap();
        let mut stream = context.socket.take().ok_or(anyhow!("The socket already taken."))?;
        let mut telescreen = (&attachment.pointer).lock();
        for data in telescreen.buffer.iter() {
            stream.write_all(&data).await?;
        };

        let room = crate::srvpru::room::Room::find_room_by_name(&message::cast_to_string(&request.pass).unwrap()).ok_or(anyhow!("Cannot find the room."))?;
        let (mut player, _) = player::upgrade_player_precursor(context.addr.clone(), room.clone()).ok_or(anyhow!("Failed to upgrade player cursor"))?;
        player.client_stream_writer = Some(stream);
        telescreen.watchers.push(player);
        // watcher won't be put in PLAYERS; so any message won't be truly sent to server.
        // But add clients to room query so that it can be correctly lead to fowllowing interceptors.
        let mut rooms_by_client_addr = crate::srvpru::room::ROOMS_BY_CLIENT_ADDR.write();
        rooms_by_client_addr.insert(context.addr.clone(), room);
        Ok(true)
    })).register();

    // Intercept all message except chat from 
    Handler::new(1, "telescreen_message_interceptor", |context| context.message_type != Some(MessageType::CTOS(CTOSMessageType::Chat)), |context| Box::pin(async move {
        let telescreen = get_room_attachment(context);
        if telescreen.is_none() { return Ok(false); }
        let telescreen = telescreen.unwrap();
        let _telescreen = telescreen.pointer.lock();
        if _telescreen.watchers.iter().any(|watcher| watcher.client_addr == *context.addr) {
            warn!("Big brother try to do something other than chat.");
            Err(anyhow!("Big brother try to do something other than chat."))
        }
        else { Ok(false) }
    })).register();

    Handler::follow_message::<CTOSChat, _>(255, "telescreen_loudspeaker", |context, request| Box::pin(async move {
        let telescreen = get_room_attachment_sure(context);
        let mut _telescreen = telescreen.pointer.lock();
        if _telescreen.watchers.iter().any(|watcher| watcher.client_addr == *context.addr) {
            let message = message::cast_to_string(&request.msg).ok_or(anyhow!("Cannot cast sent message"))?;
            info!("{:?}", context.socket);
            context.send_raw_chat_to_room(&message, Colors::Observer).await.ok(); // Send to player itself will fail, as context.socket always None.
            for player in _telescreen.watchers.iter_mut() {
                player.send_chat_raw(&message, Colors::Observer).await?;
            }
            Ok(true)
        }
        else { Ok(false) }
    })).register();

    srvpru_handler!(crate::srvpru::structs::RoomDestroy, |_, request| {
        let attachment = drop_room_attachment(request).ok_or(anyhow!("Attachment already taken."))?;
        let mut _attachment = attachment.pointer.lock();
        if let Some(handle) = _attachment.listener.as_mut() {
            handle.abort();
        };
    }).register_as("telescreen_room_attachment_dropper");

    Handler::register_handlers("telescreen", Direction::CTOS, vec!("telescreen_watcher", "telescreen_message_interceptor", "telescreen_loudspeaker"));
    Handler::register_handlers("telescreen", Direction::SRVPRU, vec!("telescreen_injector", "telescreen_room_attachment_dropper"));
}

async fn register_telescreen(addr: SocketAddr, telescreen: Arc<Mutex<Telescreen>>) -> anyhow::Result<()> {
    let stream = TcpStream::connect(addr).await?;
    let (mut reader, mut writer) = stream.into_split();
    let version = version_checker::get_configuration().version;
    writer.write_all(&wrap_mapped_struct(&message::CTOSPlayerInfo { name:  cast_to_array("The telescreen") })).await?;
    writer.write_all(&wrap_mapped_struct(&message::CTOSJoinGame { 
        version,
        align: 0,
        gameid: 0,
        pass: cast_to_array("The telescreen")
    })).await?;
    writer.write_all(&wrap_struct(&MessageType::CTOS(CTOSMessageType::HsToOBServer), &message::Empty {} )).await?; 
    writer.forget();
    let telescreen_for_listener = telescreen.clone();
    let mut _telescreen = telescreen.lock();
    _telescreen.listener = Some(tokio::spawn(async move {
        let mut buf = [0; 10240];
        loop {
            let n = match reader.read(&mut buf).await {
                Ok(n) if n == 0 => break,
                Ok(n) => n,
                Err(e) => {
                    error!("Error on big brother listening on {:}: {}", &addr, e);
                    break
                }
            };
            let slice = &buf[0..n];
            let mut telescreen = telescreen_for_listener.lock();
            telescreen.buffer.push(slice.to_vec());
            for player in telescreen.watchers.iter_mut() {
                if let Some(stream) = player.client_stream_writer.as_mut() {
                    stream.write_all(slice).await.ok(); // We don't care watcher success or not. If it fail he can rejoin.
                }
            }
        }
        info!("Telescreen on {:} is shutdown.", addr);
    }));
    Ok(())
}