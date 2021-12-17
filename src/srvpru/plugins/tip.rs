// ============================================================
// tips
// ------------------------------------------------------------
//! Send noisy tips.
// ============================================================

use rand::prelude::SliceRandom;
use tokio::task::JoinHandle;

use crate::srvpru::Handler;
use crate::srvpru::CommonError;
use crate::ygopro::Colors;
use crate::srvpru::generate_chat;
use crate::srvpru::message::RoomCreated;
use crate::srvpru::message::RoomDestroy;
use crate::srvpru::plugins::base::chat_command;
use crate::ygopro::message::stoc::DuelStart;


set_configuration! {
    #[serde(default)]
    tips: Vec<String>,
    #[serde(default = "default_interval_when_prepare")]
    interval_when_prepare: u64,
    #[serde(default = "default_interval_when_in_game")]
    interval_when_in_game: u64
}

fn default_interval_when_prepare() -> u64 { 100000 }
fn default_interval_when_in_game() -> u64 { 200000 }

room_attach! {
    tip_sender: Option<JoinHandle<()>>
}

depend_on! {
    "chat_command"
}

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    register_dependency()?;
    Ok(())
}

fn register_handlers() {
    chat_command::before_message( "tip", |context, _| Box::pin(async move {
        let tip = select_a_tip();
        context.send_back(&generate_chat(tip, Colors::Lightblue, context.get_region())).await.ok();
    })).register_for_plugin("tip");

    Handler::follow_message::<RoomCreated, _>(100, "tip_interval_runner_1", |context, message| Box::pin(async move {
        let mut attachment = get_room_attachment_sure(context)?;
        let room = message.room.clone();
        let mut interval =  tokio::time::interval(tokio::time::Duration::from_millis(get_configuration().interval_when_prepare));
        attachment.tip_sender = Some(tokio::spawn(async move {
            loop {
                interval.tick().await;
                room.lock().send_chat(select_a_tip(), Colors::Blue).await;
            }
        }));
        Ok(false)
    })).register_for_plugin("tip");

    Handler::follow_message::<DuelStart, _>(100, "tip_interval_runner_2", |context, _| Box::pin(async move {
        let mut attachment = get_room_attachment_sure(context)?;
        let room = context.get_room().ok_or(CommonError::RoomNotExist)?.clone();
        let mut interval =  tokio::time::interval(tokio::time::Duration::from_millis(get_configuration().interval_when_in_game));
        if let Some(thread)  = attachment.tip_sender.as_mut() { thread.abort(); } 
        attachment.tip_sender = Some(tokio::spawn(async move {
            loop {
                interval.tick().await;
                room.lock().send_chat(select_a_tip(), Colors::Blue).await;
            }
        })); 
        Ok(false)
    })).register_for_plugin("tip");

    Handler::before_message::<RoomDestroy, _>(100, "tip_dropper", |_, message| Box::pin(async move {
        if let Some(room_attachment) = drop_room_attachment(message) {
            if let Some(handler) = room_attachment.tip_sender {
                handler.abort();
            }
        }
        Ok(false)
    })).register_for_plugin("tip");
}

fn select_a_tip() -> &'static str {
    get_configuration().tips.choose(&mut rand::thread_rng()).map(|string| string.as_str()).unwrap_or("N/A")
}