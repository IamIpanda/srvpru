use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use parking_lot::RwLock;

use crate::srvpru::Handler;
use crate::srvpru::Context;
use crate::ygopro::message::ctos;
use crate::ygopro::message::srvpru;

lazy_static! {
    static ref COMMANDS: RwLock<HashMap<&'static str, Box<dyn for <'a, 'b> Fn(&'b mut Context<'a>, &'b String) -> Pin<Box<dyn Future<Output = ()> + Send + 'b>> + Send + Sync>>> = RwLock::new(HashMap::new());
    static ref PLUGIN_COMMANDS: RwLock<HashMap<&'static str, Vec<&'static str>>> = RwLock::new(HashMap::new());
    static ref ENABLED_COMMANDS: RwLock<Vec<&'static str>> = RwLock::new(Vec::new());
}

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}

fn register_handlers() {
    Handler::before_message::<ctos::Chat, _>(100, "chat_command", |context, message| Box::pin(async move {
        let message = context.get_string(&message.msg, "message")?.clone();
        if message.starts_with("/") {
            for (name, execution) in COMMANDS.read().iter() {
                if message[1..].starts_with(name) {
                    execution(context, &message).await;
                    break;
                }
            }
            return context.block_message()
        }
        Ok(false)
    })).register_for_plugin("chat_command");
}

pub fn before_message<F>(name: &'static str, execution: F) -> Handler
    where F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b String) -> Pin<Box<dyn Future<Output = ()> + Send + 'b>> + Copy,
          F: Send + Sync + 'static {
    Handler::before_message::<srvpru::ServerStart, _>(100, &format!("chat_command_{:}", name), move |_, _| Box::pin(async move {
        COMMANDS.write().insert(name, Box::new(execution));
        Ok(false)
    }))
}