// ============================================================
// newspeak
// ------------------------------------------------------------
//! Stop user sending sensetive words in chat and room name.
// ============================================================

use crate::ygopro::message::CTOSChat;
use crate::ygopro::message::STOCChat;
use crate::ygopro::constants::Colors;
use crate::ygopro::message::Direction;

use crate::srvpru::Context;
use crate::srvpru::Handler;
use crate::ygopro::message::Struct;

set_configuration! {
    bad_words: Vec<Vec<String>>,
    behavior: Vec<BadwordBehavior>,
    #[serde(default)]
    report_to_big_brother: String
}

#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
pub enum BadwordBehavior {
    /// Block that message, and send a warning.
    Block,
    /// Replace sensitive words with *, and send a warning.
    Replace,
    /// Block that message, but sender will see it as normal.
    Silent
}

pub fn init() -> anyhow::Result<()>  {
    load_configuration()?;
    register_handlers();
    Ok(())
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
    let socket = _player.client_stream_writer.as_mut().ok_or(anyhow!("Player socket already taken."))?;
    crate::srvpru::send_chat(socket, &format!("{{chat_warn_level{}}}", level), "zh-cn", Colors::Red).await?;
    Ok(())
}

fn register_handlers() {
    Handler::follow_message::<CTOSChat, _>(101, "newspeak_chat", |context, request| Box::pin(async move {
        let configuration = get_configuration();
        let mut message = context.get_string(&request.msg, "message")?.clone();
        let level = judge_bad_word_level(&mut message);
        if level >= 0 {
            match configuration.behavior[level as usize] {
                BadwordBehavior::Block => {
                    send_warning_mesage(context, level).await.ok();
                    return Ok(true) 
                },
                BadwordBehavior::Replace => {        
                    let response = CTOSChat { msg: crate::ygopro::message::cast_to_c_array(&message) };
                    context.response = Some(Box::new(response) as Box<dyn Struct>);
                    send_warning_mesage(context, level).await.ok();
                },
                BadwordBehavior::Silent => {
                    let player = context.get_player().ok_or(anyhow!("Cannot find current player"))?;
                    let mut _player = player.lock();
                    if let Some(socket) = _player.client_stream_writer.as_mut() {
                        let pos: u8 = context.get_position().map(|position| position.into()).unwrap_or(7);
                        crate::srvpru::send(socket, &STOCChat { name: pos as u16, msg: request.msg.clone() }).await.ok();
                    }
                    return Ok(true);
                },
            }
            if configuration.report_to_big_brother.len() > 0 {
                let _message = message.clone();
                tokio::spawn(async move{
                    // send request here
                });
            }
        }
        Ok(false)
    })).register();

    Handler::register_handlers("newspeak", Direction::CTOS, vec!["newspeak_chat"]);
}