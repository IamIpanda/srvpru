// ============================================================
// telescreen
// ------------------------------------------------------------
//! Insert a telescreen user in any duel,
//! it will record all the message received.  
//! Offer a half-way observer for outside. 
//! 
//! Dependency :
//! - [version_checker](super::version_checker)
//! - [stage_recorder](super::recorder::stage_recorder)
// ============================================================

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::io::AsyncReadExt;
use parking_lot::Mutex;

use crate::srvpru::CommonError;
use crate::srvpru::HandlerCondition;
use crate::srvpru::HandlerOccasion;
use crate::srvpru::Player;
use crate::srvpru::ProcessorError;
use crate::srvpru::generate_chat;
use crate::srvpru::generate_raw_chat;
use crate::srvpru::processor::Handler;
use crate::srvpru::plugins::recorder::stage_recorder;
use crate::srvpru::plugins::version_checker;
use crate::srvpru::PlayerPrecursor;

use crate::ygopro::Colors;
use crate::ygopro::Netplayer;
use crate::ygopro::message::ctos;
use crate::ygopro::message::stoc;
use crate::ygopro::message::srvpru;
use crate::ygopro::message::Direction;
use crate::ygopro::message::MessageType;
use crate::ygopro::message::string::cast_to_fix_length_array;
use crate::ygopro::message::generate::wrap_mapped_struct;

room_attach! {
    pointer: Arc<Mutex<Telescreen>>,
    intercept_next: bool
}

#[derive(Default, Debug)]
pub struct Telescreen {
    listener: Option<JoinHandle<()>>,
    buffer: Vec<Vec<u8>>,
    watchers: Vec<Player>,
}

depend_on! {
    "version_checker",
    "stage_recorder"
}

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    register_dependency()?;
    Ok(())
}

const TELESCREEN_NAME: &str = "The telescreen";

fn register_handlers() {
    Handler::follow_message::<srvpru::RoomCreated, _>(250, "telescreen_injector", |context, _| Box::pin(async move {
        let mut attachment = get_room_attachment_sure(context)?;
        let room = context.get_room().ok_or(CommonError::RoomNotExist)?;
        let addr = room.lock().server_addr.clone().ok_or(anyhow!("Server haven't spawn."))?;
        
        if room.lock().players.len() == 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
        if room.lock().players.len() == 0 {
            return Err(anyhow!("Player 1 don't join game on time."));
        }
        register_telescreen(addr, attachment.pointer.clone()).await?;
        attachment.intercept_next = true;
        Ok(false)
    })).register();

    Handler::before_message::<ctos::PlayerInfo, _>(9, "telescreen_blocker", |context, message| Box::pin(async move {
        let name = context.get_string(&(message.name), "name")?;
        if name == TELESCREEN_NAME {
            context.send(&generate_chat("{bad_user_name}", Colors::Red, context.get_region())).await.ok();
            Err(ProcessorError::Abort)?;
        }
        Ok(false)
    })).register();
    
    Handler::before_message::<ctos::JoinGame, _>(8, "telescreen_watcher", |context, message| Box::pin(async move {
        let duel_stage = context.get_duel_stage_in_join_game(message);
        if duel_stage <= stage_recorder::DuelStage::Begin { return Ok(false) };

        let attachment = get_attachment_by_name(context, &message).ok_or(anyhow!("Cannot find telescreen attachement."))?;
        let mut stream = context.socket.take().ok_or(anyhow!("The socket already taken."))?;
        let mut telescreen = (&attachment.pointer).lock();
        for data in telescreen.buffer.iter() {
            stream.write_all(&data).await?;
        };

        let room = context.get_room_in_join_game(message).ok_or(CommonError::RoomNotExist)?.clone();
        let (mut player, _) = PlayerPrecursor::upgrade(context.addr.clone(), room.clone()).ok_or(anyhow!("Failed to upgrade player cursor"))?;
        player.client_stream_writer = Some(stream);
        telescreen.watchers.push(player);
        // watcher won't be put in PLAYERS; so any message won't be truly sent to server.
        // But add clients to room query so that it can be correctly lead to fowllowing interceptors.
        let mut rooms_by_client_addr = crate::srvpru::room::ROOMS_BY_CLIENT_ADDR.write();
        rooms_by_client_addr.insert(context.addr.clone(), room.clone());

        context.block_message()
    })).register();

    Handler::before_message::<stoc::HsPlayerEnter, _>(1, "telescreen_hider", |context, message| Box::pin(async move {
        if context.get_string(&message.name, "name")? == TELESCREEN_NAME {
            return context.block_message();
        }
        Ok(false)
    })).register();

    Handler::before_message::<stoc::HsPlayerChange, _>(1, "telescreen_hider2", |context, message| Box::pin(async move {
        if message.status == crate::ygopro::PlayerChange::Observe(Netplayer::Player2) { 
            let mut attachment = get_room_attachment_sure(context)?;
            if attachment.intercept_next {
                attachment.intercept_next = false;
                return context.block_message(); 
            }
        }
        Ok(false)
    })).register();

    Handler::before_message::<stoc::HsWatchChange, _>(1, "telescreen_hider3", |context, message| Box::pin(async move {
        if message.match_count == 0 { return Ok(false); } // Telescrenn actual don't light up.
        message.match_count -= 1;
        context.reserialize = true;
        Ok(false)
    })).register();

    // Intercept all message except chat from 
    Handler::new(1, "telescreen_message_interceptor", HandlerOccasion::Before, HandlerCondition::Dynamic(Box::new(|context| context.message_type != Some(MessageType::CTOS(ctos::MessageType::Chat)))), |context| Box::pin(async move {
        if let Some(telescreen) = get_room_attachment(context) {
            let _telescreen = telescreen.pointer.lock();
            if _telescreen.watchers.iter().any(|watcher| watcher.client_addr == context.addr) {
                warn!("Big brother try to do something other than chat.");
                Err(anyhow!("Big brother try to do something other than chat."))?
            }
        }
        Ok(false)
    })).register();

    Handler::before_message::<ctos::Chat, _>(255, "telescreen_loudspeaker", |context, message| Box::pin(async move {
        let telescreen = get_room_attachment_sure(context)?;
        let mut _telescreen = telescreen.pointer.lock();
        if _telescreen.watchers.iter().any(|watcher| watcher.client_addr == context.addr) {
            let message = context.get_string(&message.msg, "msg")?;
            let chat = generate_raw_chat(&message, Colors::Observer);
            context.send_to_room(&chat).await.ok(); // Send to player itself will fail, as context.socket always None.
            for player in _telescreen.watchers.iter_mut() {
                player.send_to_client(&chat).await?;
            }
            context.block_message()
        }
        else { Ok(false) }
    })).register();

    srvpru_handler!(srvpru::RoomDestroy, |_, message| {
        let attachment = drop_room_attachment(message).ok_or(anyhow!("Attachment already taken."))?;
        let mut _attachment = attachment.pointer.lock();
        if let Some(handle) = _attachment.listener.as_mut() {
            handle.abort();
        };
    }).register_as("telescreen_room_attachment_dropper");

    Handler::register_handlers("telescreen", Direction::CTOS, vec!["telescreen_watcher", "telescreen_blocker", "telescreen_message_interceptor", "telescreen_loudspeaker"]);
    Handler::register_handlers("telescreen", Direction::STOC, vec!["telescreen_hider", "telescreen_hider2", "telescreen_hider3"]);
    Handler::register_handlers("telescreen", Direction::SRVPRU, vec!["telescreen_injector", "telescreen_room_attachment_dropper"]);
}

async fn register_telescreen(addr: SocketAddr, telescreen: Arc<Mutex<Telescreen>>) -> anyhow::Result<()> {
    let stream = TcpStream::connect(addr).await?;
    let (mut reader, mut writer) = stream.into_split();
    let version = version_checker::get_configuration().version;
    writer.write_all(&wrap_mapped_struct(&struct_sequence! {
        ctos::PlayerInfo { name: cast_to_fix_length_array(TELESCREEN_NAME) },
        ctos::JoinGame { 
            version,
            align: 0,
            gameid: 0,
            pass: cast_to_fix_length_array(TELESCREEN_NAME)
        },
        ctos::HsToOBServer {}
    })).await?;
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