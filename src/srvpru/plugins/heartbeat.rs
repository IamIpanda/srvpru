// ============================================================
// Heartbeat
// ------------------------------------------------------------
//! If a client don't send heartbeat at time, expel it.
//! 
//! ygopro heartbeat is by following rules: \
//! For each time sending a [`TimeLimit`], client should send a 
//! [`TimeConfirm`] back in [`diastole_time`].
//! This plugin send a [`TimeLimit`] with [`beat_per_minute`].
//! 
//! Dependency:
//! - [position_recorder](super::recorder::position_recorder)
// ============================================================

use tokio::time::Duration;
use tokio::task::JoinHandle;

use crate::ygopro::Netplayer; 
use crate::ygopro::Location;
use crate::ygopro::message::gm;
use crate::ygopro::message::ctos::TimeConfirm;
use crate::ygopro::message::stoc::TimeLimit;
use crate::ygopro::message::stoc::DuelStart;
use crate::ygopro::message::MessageType;
use crate::srvpru::Handler;
use crate::srvpru::CommonError;

set_configuration! {
    #[serde(default = "default_beat_per_minute")]
    beat_per_minute: u64,
    #[serde(default = "default_disatole_time")]
    diastole_time: u64
}

fn default_beat_per_minute() -> u64 { 3 } // 20 seconds for one beat
fn default_disatole_time() -> u64 { 10 } // response in 10 seconds

player_attach! {
    death_clock: Option<JoinHandle<()>>,
    exempt: bool
}

room_attach! {
    heartbeat_watcher: Option<JoinHandle<()>>
}

depend_on! {
    "position_recorder"
}

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    register_exempts();
    register_dependency()?;
    Ok(())
}

fn register_handlers() {
    Handler::follow_message::<DuelStart, _>(100, "heartbeat_register", |context, _| Box::pin(async move {
        let mut attachment = get_room_attachment_sure(context)?;
        if attachment.heartbeat_watcher.is_some() { return Ok(false) }
        let players = context.get_room().ok_or(CommonError::RoomNotExist)?.lock().get_players_in_hashmap();
        attachment.heartbeat_watcher = Some(tokio::spawn(async move {
            let configuration = get_configuration();
            let mut interval = tokio::time::interval(Duration::from_secs(60 / configuration.beat_per_minute));
            interval.tick().await;
            loop {
                interval.tick().await;
                let mut player_attachments = PLAYER_ATTACHMENTS.write();
                for (position, player) in players.iter() {
                    if *position == Netplayer::Observer { continue; }
                    if let Some(player_attachment) = player_attachments.get_mut(&player.lock().client_addr) {
                        if player_attachment.exempt { return; }
                    }
                    if player.lock().send_to_client(&struct_sequence![
                        TimeLimit { player: Netplayer::Player1, left_time: 0 },
                        TimeLimit { player: Netplayer::Player2, left_time: 0 }
                    ]).await.is_ok() {
                        set_death_clock(player.clone());
                    }
                }
            };
        }));
        Ok(false)
    })).register_for_plugin("heartbeat");

    Handler::follow_message::<TimeConfirm, _>(100, "heartbeat", |context, _| Box::pin(async move {
        trace!("Get heartbeat from player {:}", context.get_player().ok_or(CommonError::PlayerNotExist)?.lock().name);
        if let Some(death_clock) = get_player_attachment_sure(context).death_clock.take() {
            death_clock.abort();
        }
        Ok(false)
    })).register_for_plugin("heartbeat");

    Handler::follow_message::<TimeLimit, _>(100, "heartbeat_diastole", |context, _| Box::pin(async move {
        set_death_clock(context.get_player().ok_or(CommonError::PlayerNotExist)?.clone());
        Ok(false)
    })).register_for_plugin("heartbeat");
}

fn register_exempts() {
    Handler::before_message::<gm::ConfirmCards, _>(100, "heartbeat_exempt_confirm", |context, message| Box::pin(async move {
        if message.cards.iter().any(|card| matches!(card.location, Location::Limbo | Location::Deck | Location::Extra)) {
            get_player_attachment_sure(context).exempt = true;
        }
        Ok(false)
    })).register_for_plugin("heartbeat");


    // ----------------------------------------------------------------------------------------------------
    // Attention
    // ----------------------------------------------------------------------------------------------------
    // Implement is different here.
    //
    // Srvpro logic: 
    // Find a long resolve card in gm::Chaining -> mark long_resolve_cards
    // Find a long_resolve_cards mark in gm::Chained -> mark long_resolve_chain
    // Find a long_resolve_chain mark in gm::ChainSolving -> mark exempt for each player in room.
    // Find a gm::ChainNegated or gm::ChainDisabled -> remove long_resolve_chain flag.
    // gm::ChainSolved -> remove long_resolve_cards, long_resolve_chain flag.
    // 
    // Srvpru logic:
    // gm::ChainSolving -> mark exempt for current player
    // gm::ChainSolved, gm::ChainNegated, gm:: ChainDisabled -> remove exempt
    // ----------------------------------------------------------------------------------------------------
    Handler::before_message::<gm::ChainSolving, _>(100, "heartbeat_exempt_chain", |context, _| Box::pin(async move {
        get_player_attachment_sure(context).exempt = true;
        Ok(false)
    })).register_for_plugin("heartbeat");

    Handler::new(101, "heartbeat_exempt_resolved", crate::srvpru::HandlerOccasion::Before, crate::srvpru::HandlerCondition::Dynamic(Box::new(|context| {
        matches!(context.message_type, Some(MessageType::GM(gm::MessageType::ChainEnd)) 
                                     | Some(MessageType::GM(gm::MessageType::ChainNegated)) 
                                     | Some(MessageType::GM(gm::MessageType::ChainDisabled)))
    })), |context| Box::pin(async move {
        get_player_attachment_sure(context).exempt = false;
        Ok(false)
    })).register();

    Handler::register_handlers("heartbeat", crate::ygopro::message::Direction::STOC, vec!("heartbeat_exempt_resolved"))
}

fn set_death_clock(player: std::sync::Arc<parking_lot::Mutex<crate::srvpru::Player>>) {
    trace!("Set heartbeat clock on {:}", player.lock().name); 
    let mut player_attachments = PLAYER_ATTACHMENTS.write();
    let mut attachment = player_attachments.entry(player.lock().client_addr).or_default();
    { attachment.death_clock.take().map(|handle| handle.abort()); }
    attachment.death_clock = Some(tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(get_configuration().diastole_time)).await;
        let mut _player = player.lock();
        if let Some(attachment) = PLAYER_ATTACHMENTS.read().get(&_player.client_addr) {
            if ! attachment.exempt {
                _player.expel();
            }
        }
    }));
}
