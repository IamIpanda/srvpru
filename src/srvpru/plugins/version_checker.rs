// ============================================================
// version_checker
// ------------------------------------------------------------
//! Stop any ygopro client with wrong version join the game.
// ============================================================

use crate::ygopro::message;
use crate::srvpru::processor::Handler;

set_configuration! {
    version: u16
}

pub fn register_handlers() {
    let configuration = CONFIGURATION.get().unwrap();

    Handler::follow_message::<message::CTOSJoinGame, _>(2, "version_checker",  move |context, request| Box::pin(async move {
        if request.version < configuration.version {
            context.send_chat("{outdated_client}", crate::ygopro::constants::Colors::Red).await?;
            return Ok(true)
        }
        Ok(false)
    })).register();

    Handler::register_handlers("version_checker", message::Direction::CTOS, vec!("version_checker"));
}


pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}