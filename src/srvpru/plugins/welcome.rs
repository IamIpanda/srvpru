// ============================================================
// welcome
// ------------------------------------------------------------
//! Send welcome message when player get in.
// ============================================================

use anyhow::Result;

use crate::ygopro::message::*;
use crate::ygopro::constants::*;
use crate::srvpru::processor::Handler;


set_configuration! {
    welcome_message: String
}

pub fn register_handlers() {
    let configuration = get_configuration();
    let welcome_handler = Handler::follow_message::<CTOSJoinGame, _>(4, "welcome",  move |context, _| Box::pin(async move {
        context.send_chat(&configuration.welcome_message, Colors::Babyblue).await?;
        Ok(false)
    }));
    
    Handler::register_handler("welcome", welcome_handler);
}


pub fn init() -> Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}