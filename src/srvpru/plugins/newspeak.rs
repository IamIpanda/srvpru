// ============================================================
// newspeak
// ------------------------------------------------------------
//! Stop user sending sensetive words in chat and room name.
// ============================================================

use std::net::IpAddr;
use std::sync::Arc;
use parking_lot::Mutex;

use crate::srvpru::ProcessorError;
use crate::ygopro::Colors;
use crate::ygopro::message::Struct;
use crate::ygopro::message::Direction;
use crate::ygopro::message::ctos::Chat;
use crate::ygopro::message::ctos::JoinGame;
use crate::ygopro::message::stoc::HsPlayerEnter;

use crate::srvpru::Context;
use crate::srvpru::Handler;
use crate::srvpru::Room;
use crate::srvpru::Player;
use crate::srvpru::generate_chat;
use crate::ygopro::message::ctos::PlayerInfo;
use crate::ygopro::message::stoc::ErrorMessage;
use crate::ygopro::message::string::cast_to_fix_length_array;

set_configuration! {
    bad_words: Vec<Vec<String>>,
    behavior: Vec<BadwordBehavior>,
    #[serde(default)]
    report_to_big_brother: String,
    #[serde(default)]
    big_brother_access_key: String
}

/// Decide what to do on bad words.
#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
pub enum BadwordBehavior {
    /// Block that message, and send a warning.
    Block,
    /// Replace sensitive words with *, and send a warning.
    Replace,
    /// Block that message, but sender will see it as normal.
    Silent,
    /// Do nothing.
    None
}

pub fn init() -> anyhow::Result<()>  {
    load_configuration()?;
    register_handlers();
    Ok(())
}

fn register_handlers() {
    Handler::before_message::<Chat, _>(101, "newspeak_chat", |context, request| Box::pin(async move {
        let configuration = get_configuration();
        let mut message = context.get_string(&request.msg, "message")?.clone();
        let level = judge_bad_word_level(&mut message);
        if level >= 0 {
            let player = context.get_player();
            let room = context.get_room();
            if let (Some(player), Some(room)) = (player, room) {
                let message = message.clone();
                tokio::spawn(async move { 
                    if let Err(e) = report_to_big_brother(room, player, level as u8, message).await {
                        warn!("Report to big brother failed: {:}", e);
                    }
                });
            }
            else {
                warn!("Can't determine room or player for badword. Report to big brother failed.");
            }
        }
        if level >= 0 {
            match configuration.behavior[level as usize] {
                BadwordBehavior::Block => {
                    send_warning_mesage(context, level).await.ok();
                    return Ok(true) 
                },
                BadwordBehavior::Replace => {        
                    let response = Chat { msg: crate::ygopro::message::string::cast_to_c_array(&message) };
                    context.response = Some(Box::new(response) as Box<dyn Struct>);
                    send_warning_mesage(context, level).await.ok();
                },
                BadwordBehavior::Silent => {
                    let player = context.get_player().ok_or(anyhow!("Cannot find current player"))?;
                    let mut _player = player.lock();
                    if let Some(socket) = _player.client_stream_writer.as_mut() {
                        let pos: u8 = context.get_position().into();
                        crate::srvpru::send(socket, &crate::ygopro::message::stoc::Chat { name: pos as u16, msg: request.msg.clone() }).await.ok();
                    }
                    return Ok(true);
                },
                BadwordBehavior::None => {},
            }
        }
        Ok(false)
    })).register();

    Handler::before_message::<PlayerInfo, _>(1, "newspeak_player_name", |context, request| Box::pin(async move {
        let name = context.get_string(&request.name, "name")?;
        let level = judge_bad_word_level(name);
        let configuration = get_configuration();
        if level >= 0 {
            match configuration.behavior[level as usize] {
                BadwordBehavior::Block => {
                    context.send(&struct_sequence![
                        generate_chat(&format!("{{bad_name_level{:}}}", level), Colors::Red, context.get_region()),
                        ErrorMessage{ msg: crate::ygopro::ErrorMessage::Joinerror, align: [0; 3], code: 2 }
                    ]).await.ok();
                    Err(ProcessorError::Abort)?;
                },
                BadwordBehavior::Replace => context.response = Some(Box::new(PlayerInfo { name: cast_to_fix_length_array(name) })),
                BadwordBehavior::Silent => *name = "******".to_string(), // Block on STOC
                BadwordBehavior::None => {}, 
            }
        }
        Ok(false)
    })).register();

    Handler::before_message::<JoinGame, _>(5, "newspeak_roomname", |context, request| Box::pin(async move {
        let name = context.get_string(&request.pass, "pass")?;
        let level = judge_bad_word_level(name);
        let configuration = get_configuration();
        if level > 0 {
            match configuration.behavior[level as usize] {
                BadwordBehavior::Block => {
                    context.send(&struct_sequence![
                        generate_chat(&format!("{{bad_roomname_level{:}}}", level), Colors::Red, context.get_region()),
                        ErrorMessage{ msg: crate::ygopro::ErrorMessage::Joinerror, align: [0; 3], code: 2 }
                    ]).await.ok();
                    Err(ProcessorError::Abort)?;
                },
                BadwordBehavior::Replace => context.response = Some(Box::new(JoinGame { 
                    version: request.version, 
                    align: 0, 
                    gameid: request.gameid,
                    pass: cast_to_fix_length_array(name)
                })),
                BadwordBehavior::Silent => *name = "illegal_room_name_".to_string() + &chrono::offset::Local::now().timestamp_millis().to_string(),
                BadwordBehavior::None => {},
            }
        }
        Ok(false)
    })).register();

    Handler::before_message(1, "newspeak_stoc_player_name", |context, request: &HsPlayerEnter| Box::pin(async move {
        let name = context.get_string(&request.name, "name")?;
        let level = judge_bad_word_level(name);
        let configuration = get_configuration();
        if level >= 0 {
            match configuration.behavior[level as usize] {
                BadwordBehavior::Block => {/* should be blocked by ctos */} 
                BadwordBehavior::Replace => context.response = Some(Box::new(HsPlayerEnter { name: cast_to_fix_length_array(name), pos: request.pos })),
                BadwordBehavior::Silent => {
                    *name = "******".to_string();
                    if request.pos != context.get_position() {
                        context.response = Some(Box::new(HsPlayerEnter { name: cast_to_fix_length_array("******"), pos: request.pos }))
                    }
                },
                BadwordBehavior::None => {},
            }
        }
        Ok(false)
    })).register();

    Handler::register_handlers("newspeak", Direction::CTOS, vec!["newspeak_chat", "newspeak_player_name", "newspeak_roomname"]);
    Handler::register_handlers("newspeak", Direction::STOC, vec!["newspeak_stoc_player_name"]);
}

fn judge_bad_word_level(target: &mut String) -> i8 {
    let configuration = get_configuration();
    let mut current_level = -1;
    for (level, bad_word_group) in configuration.bad_words.iter().enumerate().rev() {
        for bad_word in bad_word_group {
            if target.contains(bad_word) {
                if configuration.behavior[level] == BadwordBehavior::Block { return level as i8; }
                else if configuration.behavior[level] == BadwordBehavior::Replace {
                    *target = target.replace(bad_word, "**");
                }
                current_level = level as i8; 
            }
        }
    }
    current_level
}

async fn send_warning_mesage<'a>(context: &Context<'a>, level: i8) -> anyhow::Result<()> {
    let player = context.get_player().ok_or(anyhow!("Can't find current player"))?;
    let mut _player = player.lock();
    _player.send_to_client(&generate_chat(&format!("{{chat_warn_level{}}}", level), Colors::Red, context.get_region())).await?;
    Ok(())
}

struct BadWordReport {
    access_key: String,
    room_name: String,
    sender: String,
    ip: IpAddr,
    level: u8,
    content: String,
    _match: String
}

impl BadWordReport {
    fn to_form(self) -> Vec<(&'static str, String)> {
        vec![
            ("accesskey", self.access_key),
            ("roomname", self.room_name),
            ("sender", self.sender),
            ("ip", self.ip.to_string()),
            ("level", self.level.to_string()),
            ("content", self.content),
            ("match", self._match)
        ]
    }
}

async fn report_to_big_brother(room: Arc<Mutex<Room>>, player: Arc<Mutex<Player>>, level: u8, content: String) -> anyhow::Result<()> {
    let configuration = get_configuration();
    if configuration.report_to_big_brother == "" { return Ok(()); }
    let room_name = room.lock().name.clone();
    let sender = player.lock().name.clone();
    let ip = player.lock().client_addr.ip();
    let report = BadWordReport {
        access_key: configuration.big_brother_access_key.clone(),
        room_name,
        sender,
        ip,
        level,
        content,
        _match: "".to_string()
    };
    let client = reqwest::Client::new();
    client.post(&configuration.report_to_big_brother).form(&report.to_form()).send().await?;
    Ok(())
}