// ============================================================
// debugger
// ------------------------------------------------------------
//! Offer a debugger handler to print what message is sent.
// ============================================================

use crate::ygopro::message;
use crate::srvpru::processor::Handler;

pub fn register_handlers() {
    Handler::new(0, "ctos_debugger", |_| true, |context| Box::pin(async move {
        let room = context.get_room();
        let text = if let Some(room) = room {
            format!("[{:}]", room.lock().server_addr.unwrap())
        } 
        else { "[ - ]".to_string() };
        debug!("CTOS Message   [{:}] -> {:} {:?}", context.addr, text, context.message_type.as_ref().unwrap());
        Ok(false)
    })).register();

    Handler::new(0, "stoc_debugger", |_| true, |context| Box::pin(async move {
        let room = context.get_room();
        let text = if let Some(room) = room {
            format!("[{:}]", room.lock().server_addr.unwrap())
        } 
        else { "[ - ]".to_string() };
        debug!("STOC Message   [{:}] <- {:} {:?}", context.addr, text, context.message_type.as_ref().unwrap());
        Ok(false)
    })).register();

    Handler::new(0, "internal_debugger", |_| true, |context| Box::pin(async move {
        debug!("SRVPRU Message [{:}] -- {:?}", context.addr, context.message_type.as_ref().unwrap());
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