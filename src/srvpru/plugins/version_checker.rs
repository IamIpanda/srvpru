// ============================================================
// version_checker
// ------------------------------------------------------------
//! Stop any ygopro client with wrong version join the game.
// ============================================================

use crate::ygopro::Colors;
use crate::ygopro::message::Direction;
use crate::ygopro::message::ctos::JoinGame;

use crate::srvpru::{ProcessorError, generate_chat};
use crate::srvpru::processor::Handler;

set_configuration! {
    version: u16
}

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}

pub fn register_handlers() {
    let configuration = get_configuration();

    Handler::follow_message::<JoinGame, _>(2, "version_checker",  move |context, request| Box::pin(async move {
        if request.version < configuration.version {
            context.send(&generate_chat("{outdated_client}", Colors::Red, context.get_region())).await?;
            Err(ProcessorError::Abort)?;
        }
        Ok(false)
    })).register();

    Handler::register_handlers("version_checker", Direction::CTOS, vec!("version_checker"));
}
