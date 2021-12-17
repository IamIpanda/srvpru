// ============================================================
// debugger
// ------------------------------------------------------------
//! Offer a debugger handler to print what message is sent.
// ============================================================

use crate::ygopro::message;
use crate::ygopro::message::srvpru;
use crate::srvpru::Handler;
use crate::srvpru::Context;
use crate::srvpru::HandlerOccasion;
use crate::srvpru::HandlerCondition;

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}

pub fn register_handlers() {
    Handler::new(0, "ctos_debugger", HandlerOccasion::Before, HandlerCondition::Always, |context| Box::pin(async move {
        debug!("CTOS  Message {:} -> {:} {:}", player_name(context), room_name(context), message_type_name(context.message_type));
        Ok(false)
    })).register();

    Handler::new(0, "stoc_debugger", HandlerOccasion::Before, HandlerCondition::Always, |context| Box::pin(async move {
        debug!("STOC  Message {:} <- {:} {:}", player_name(context), room_name(context), message_type_name(context.message_type));
        Ok(false)
    })).register();

    Handler::new(0, "internal_debugger", HandlerOccasion::Before, HandlerCondition::Always, |context| Box::pin(async move {
        debug!("SRVPR Message [{:}] -- {}", context.addr, message_type_name(context.message_type));
        if context.message_type == Some(message::MessageType::SRVPRU(srvpru::MessageType::CTOSProcessError)) {
            debug!("CTOS ERROR - {:?}", context.cast_message_to_type::<srvpru::CTOSProcessError>().unwrap().error);
        }
        if context.message_type == Some(message::MessageType::SRVPRU(srvpru::MessageType::STOCProcessError)) {
            debug!("STOC ERROR - {:?}", context.cast_message_to_type::<srvpru::STOCProcessError>().unwrap().error);
        }
        if context.message_type == Some(message::MessageType::SRVPRU(srvpru::MessageType::SRVPRUProcessError)) {
            debug!("INTERNAL ERROR - {:?}", context.cast_message_to_type::<srvpru::SRVPRUProcessError>().unwrap().error);
        }
        Ok(false)
    })).register();

    Handler::register_handlers("debugger", message::Direction::CTOS, vec!("ctos_debugger"));
    Handler::register_handlers("debugger", message::Direction::STOC, vec!("stoc_debugger"));
    Handler::register_handlers("debugger", message::Direction::SRVPRU, vec!("internal_debugger"));
}

fn message_type_name(message_type: Option<crate::ygopro::message::MessageType>) -> String {
    match message_type {
        None => "[unknown]".to_string(),
        Some(message_type) => format!("{:}", message_type)
    }
}

fn room_name<'a>(context: &Context<'a>) -> String {
    let room = match context.get_room() {
        Some(room) => room,
        None => return String::new()
    };
    let _room = room.lock();
    let address = match _room.server_addr.as_ref() {
        Some(address) => format!("[{:}]", address),
        None => "[ - ]".to_string()
    };
    format!("{} {}", address, _room.name)
}

fn player_name<'a>(context: &Context<'a>) -> String {
    let player = match context.get_player() {
        Some(player) => player,
        None => return String::new()
    };
    let _player = player.lock();
    let address = _player.client_addr;
    format!("{} [{}]", _player.name, address)
}