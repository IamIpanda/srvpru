// ============================================================
// dialogue
// ------------------------------------------------------------
//! Show dialogue when specific monster summon.
// ============================================================

use std::collections::HashMap;

use rand::prelude::SliceRandom;

use crate::srvpru::Context;
use crate::srvpru::generate_raw_chat;
use crate::ygopro::Colors;
use crate::ygopro::message::Direction;
use crate::ygopro::message::gm;
use crate::srvpru::Handler;

set_configuration! {
    dialogues: HashMap<u32, Vec<String>>
}

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}

fn register_handlers() {
    Handler::follow_message::<gm::Summoning, _>(100, "dialogue_normal_summon", |context, message| Box::pin(async move {
        if exist_dialogue(message.card) {
            send_dialogue(context, message.card).await.ok(); // We don't care if dialogue send success
        }
        Ok(false)
    })).register();

    Handler::follow_message::<gm::Spsummoning, _>(100, "dialogue_special_summon", |context, message| Box::pin(async move {
        if exist_dialogue(message.card) {
            send_dialogue(context, message.card).await.ok();
        }
        Ok(false)
    })).register();

    Handler::register_handlers("dialogue", Direction::STOC, vec!["dialogue_normal_summon", "dialogue_special_summon"]);
}

fn exist_dialogue(card: u32) -> bool {
    get_configuration().dialogues.contains_key(&card)
}

async fn send_dialogue<'a>(context: &mut Context<'a>, card: u32) -> anyhow::Result<()> {
    let configuration = get_configuration();
    let dialogues = configuration.dialogues.get(&card).ok_or(anyhow!("This card is with no dialogue"))?;
    let dialogue = dialogues.choose(&mut rand::thread_rng()).ok_or(anyhow!("Cannot select dialogue"))?;
    for line in dialogue.split("\n") {
        // Use raw chat to escape transforming patterns, as patterns are expensive.
        context.send_to_room(&generate_raw_chat(&("[Server]: ".to_string() + line), Colors::Pink)).await?;
    }
    Ok(())
}