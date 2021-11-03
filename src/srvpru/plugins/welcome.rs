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

set_configuration! {
    welcome_message: String
}

pub fn init() -> Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}

pub fn register_handlers() {
    let configuration = get_configuration();
    let welcome_handler = Handler::follow_message::<JoinGame, _>(4, "welcome",  move |context, _| Box::pin(async move {
        context.send(&generate_chat(&configuration.welcome_message, Colors::Babyblue, context.get_region())).await?;
        Ok(false)
    }));
    
    Handler::register_handler("welcome", welcome_handler);
}