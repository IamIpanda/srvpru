// ============================================================
// newspeak
// ------------------------------------------------------------
//! Stop user sending sensetive words in chat and room name.
// ============================================================

use std::net::IpAddr;
use std::sync::Arc;
use parking_lot::Mutex;

use crate::ygopro::Colors;
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
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq)]
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
    Handler::before_message::<Chat, _>(101, "newspeak_chat", |context, message| Box::pin(async move {
        let configuration = get_configuration();
        let mut chat_message = context.get_string(&message.msg, "message")?.clone();
        let level = judge_bad_word_level(&mut chat_message);
        if level >= 0 {
            if let (Some(player), Some(room)) = (context.get_player(), context.get_room()) {
                let chat_message = chat_message.clone();
                let player = player.clone();
                let room = room.clone();
                tokio::spawn(async move { 
                    if let Err(e) = report_to_big_brother(room, player, level as u8, chat_message).await {
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
                    return context.block_message();
                },
                BadwordBehavior::Replace => {
                    message.msg = crate::ygopro::message::string::cast_to_c_array(&chat_message);
                    context.reserialize = true;
                    send_warning_mesage(context, level).await.ok();
                },
                BadwordBehavior::Silent => { 
                    {
                        let player = context.get_player().ok_or(anyhow!("Cannot find current player"))?;
                        let mut _player = player.lock();
                        if let Some(socket) = _player.client_stream_writer.as_mut() {
                            let pos: u8 = context.get_position().into();
                            crate::srvpru::send(socket, &crate::ygopro::message::stoc::Chat { name: pos as u16, msg: message.msg.clone() }).await.ok();
                        }
                    }
                    return context.block_message();
                },
                BadwordBehavior::None => {},
            }
        }
        Ok(false)
    })).register();

    Handler::before_message::<PlayerInfo, _>(1, "newspeak_player_name", |context, message| Box::pin(async move {
        let name = context.get_string(&message.name, "name")?;
        let level = judge_bad_word_level(name);
        let configuration = get_configuration();
        if level >= 0 {
            match configuration.behavior[level as usize] {
                BadwordBehavior::Block => 
                    return context.refuse_join_game(Some(&format!("{{bad_name_level{:}}}", level))).await,
                BadwordBehavior::Replace => {
                    message.name = cast_to_fix_length_array(name);
                    context.reserialize = true; 
                },
                BadwordBehavior::Silent => *name = "******".to_string(), // Block on STOC
                BadwordBehavior::None => {}, 
            }
        }
        Ok(false)
    })).register();

    Handler::before_message::<JoinGame, _>(5, "newspeak_roomname", |context, message| Box::pin(async move {
        let name = context.get_string(&message.pass, "pass")?;
        let level = judge_bad_word_level(name);
        let configuration = get_configuration();
        if level > 0 {
            match configuration.behavior[level as usize] {
                BadwordBehavior::Block => 
                    return context.refuse_join_game(Some(&format!("{{bad_room_level{:}}}", level))).await,
                BadwordBehavior::Replace => {
                    message.pass = cast_to_fix_length_array(name);
                    context.reserialize = true;
                }
                BadwordBehavior::Silent => *name = "illegal_room_name_".to_string() + &chrono::offset::Local::now().timestamp_millis().to_string(),
                BadwordBehavior::None => {},
            }
        }
        Ok(false)
    })).register();

    Handler::before_message::<HsPlayerEnter, _>(1, "newspeak_stoc_player_name", |context, message| Box::pin(async move {
        let name = context.get_string(&message.name, "name")?;
        let level = judge_bad_word_level(name);
        let configuration = get_configuration();
        if level >= 0 {
            match configuration.behavior[level as usize] {
                BadwordBehavior::Block => {/* should be blocked by ctos */} 
                BadwordBehavior::Replace => {
                    message.name = cast_to_fix_length_array(name);
                    context.reserialize = true;
                },
                BadwordBehavior::Silent => {
                    *name = "******".to_string();
                    if message.pos != context.get_position() {
                        message.name = cast_to_fix_length_array("******");
                        context.reserialize = true;
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
    let player = context.get_player().clone().ok_or(anyhow!("Can't find current player"))?;
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

static REQWEST_CLIENT: once_cell::sync::OnceCell<reqwest::Client> = once_cell::sync::OnceCell::new();
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
    REQWEST_CLIENT.get_or_init(|| reqwest::Client::new()).post(&configuration.report_to_big_brother).form(&report.to_form()).send().await?;
    Ok(())
}