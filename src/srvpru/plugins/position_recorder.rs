// ============================================================
// position_recorder
// ------------------------------------------------------------
//! Record which position is player in.
// ============================================================

use crate::ygopro::constants::Netplayer;
use crate::ygopro::message::STOCTypeChange;
use crate::srvpru::Context;
use crate::srvpru::Handler;

player_attach! {
    position: Netplayer
}

impl std::default::Default for Netplayer {
    fn default() -> Self {
        return Netplayer::Observer;
    }
}

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}

pub fn register_handlers() {
    Handler::follow_message::<STOCTypeChange, _>(100, "position_recorder", |context, request| Box::pin(async move {
        let mut attachment = get_player_attachment_sure(context);
        let position = Netplayer::try_from(request._type & 0xf);
        if let Ok(position) = position {
            attachment.position = position;
        }
        Ok(false)
    })).register();
    Handler::register_handlers("position_recorder", crate::ygopro::message::Direction::STOC, vec!["position_recorder"]);
}

impl<'a> Context<'a> {
    pub fn get_position(&self) -> Option<Netplayer> {
        let attachment = get_player_attachment(self)?;
        return Some(attachment.position);
    }
}
