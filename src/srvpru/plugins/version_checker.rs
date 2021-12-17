// ============================================================
// version_checker
// ------------------------------------------------------------
//! Stop any ygopro client with wrong version join the game.
// ============================================================

use crate::ygopro::Colors;
use crate::ygopro::message::Direction;
use crate::ygopro::message::ctos::JoinGame;

use crate::srvpru::ProcessorError;
use crate::srvpru::generate_chat;
use crate::srvpru::processor::Handler;

set_reloadable_configuration! {
    version: u16
}

pub fn init() -> anyhow::Result<()> {
    init_configuration("version_checker")?;
    register_handlers();
    Ok(())
}

pub fn register_handlers() {
    Handler::before_message::<JoinGame, _>(2, "version_checker",  move |context, message| Box::pin(async move {
        let configuration = get_configuration();
        if message.version < configuration.version {
            context.send(&generate_chat("{outdated_client}", Colors::Red, context.get_region())).await?;
            Err(ProcessorError::Abort)?;
        }
        Ok(false)
    })).register();

    Handler::register_handlers("version_checker", Direction::CTOS, vec!("version_checker"));
}
