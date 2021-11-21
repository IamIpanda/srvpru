// ============================================================
// welcome
// ------------------------------------------------------------
//! Send welcome message when player get in.
// ============================================================

use anyhow::Result;

use crate::ygopro::Colors;
use crate::ygopro::message::ctos::JoinGame;

use crate::srvpru::processor::Handler;
use crate::srvpru::generate_chat;

fn default_welcome_message() -> String { "Srvpru Server".to_string() }

set_configuration! {
    #[serde(default = "default_welcome_message")]
    welcome_message: String
}

pub fn init() -> Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}

pub fn register_handlers() {
    let configuration = get_configuration();
    Handler::follow_message::<JoinGame, _>(4, "welcome",  move |context, _| Box::pin(async move {
        context.send_back(&generate_chat(&configuration.welcome_message, Colors::Green, context.get_region())).await?;
        Ok(false)
    })).register();
    
    Handler::register_handlers("welcome", crate::ygopro::message::Direction::CTOS, vec!["welcome"]);
}