// ============================================================
// debugger
// ------------------------------------------------------------
//! Offer a debugger handler to print what message is sent.
// ============================================================

use crate::ygopro::message;
use crate::ygopro::message::srvpru;
use crate::srvpru::Handler;
use crate::srvpru::HandlerOccasion;
use crate::srvpru::HandlerCondition;

pub fn register_handlers() {
    Handler::new(0, "ctos_debugger", HandlerOccasion::Before, HandlerCondition::Always, |context| Box::pin(async move {
        let room = context.get_room();
        let text = if let Some(room) = room {
            format!("[{:}]", room.lock().server_addr.unwrap())
        } 
        else { "[ - ]".to_string() };
        debug!("CTOS Message   [{:}] -> {:} {:?}", context.addr, text, context.message_type.as_ref().unwrap());
        Ok(false)
    })).register();

    Handler::new(0, "stoc_debugger", HandlerOccasion::Before, HandlerCondition::Always, |context| Box::pin(async move {
        let room = context.get_room();
        let text = if let Some(room) = room {
            format!("[{:}]", room.lock().server_addr.unwrap())
        } 
        else { "[ - ]".to_string() };
        debug!("STOC Message   [{:}] <- {:} {:?}", context.addr, text, context.message_type.as_ref().unwrap());
        trace!("{:?}", context.request);
        Ok(false)
    })).register();

    Handler::new(0, "internal_debugger", HandlerOccasion::Before, HandlerCondition::Always, |context| Box::pin(async move {
        debug!("SRVPRU Message [{:}] -- {:?}", context.addr, context.message_type.as_ref().unwrap());
        if context.message_type == Some(message::MessageType::SRVPRU(srvpru::MessageType::CtosProcessError)) {
            debug!("CTOS ERROR - {:?}", context.cast_request_to_type::<srvpru::CtosProcessError>().unwrap().error);
        }
        if context.message_type == Some(message::MessageType::SRVPRU(srvpru::MessageType::StocProcessError)) {
            debug!("STOC ERROR - {:?}", context.cast_request_to_type::<srvpru::StocProcessError>().unwrap().error);
        }
        if context.message_type == Some(message::MessageType::SRVPRU(srvpru::MessageType::InternalProcessError)) {
            debug!("INTERNAL ERROR - {:?}", context.cast_request_to_type::<srvpru::InternalProcessError>().unwrap().error);
        }
        Ok(false)
    })).register();

    Handler::register_handlers("debugger", message::Direction::CTOS, vec!("ctos_debugger"));
    Handler::register_handlers("debugger", message::Direction::STOC, vec!("stoc_debugger"));
    Handler::register_handlers("debugger", message::Direction::SRVPRU, vec!("internal_debugger"));
}

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}