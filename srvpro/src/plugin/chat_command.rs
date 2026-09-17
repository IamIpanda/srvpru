use linkme::distributed_slice;
use ygopro_data::constants::Color;
use ygopro_data::message::ctos;
use ygopro_handler::StopFlag;

use crate::room::CTOS_HANDLERS;
use crate::room::ClientToServerHandler;
use crate::room::Room;

pub type ChatCommand = fn(&mut Room, usize, &str);

#[derive(Clone, Copy)]
pub struct ChatCommandHandler {
    pub module_name: &'static str,
    pub handler: ChatCommand,
}

impl ChatCommandHandler {
    pub fn new(_priority: u8, _name: &'static str, module_name: &'static str, handler: ChatCommand) -> Self {
        Self { module_name, handler }
    }
}

#[distributed_slice]
pub static CHAT_COMMANDS: [fn() -> (&'static str, ChatCommandHandler)];

fn command_handler(room: &Room, command_name: &str) -> Option<ChatCommandHandler> {
    CHAT_COMMANDS.iter().map(|build| build())
        .find(|(name, handler)| *name == command_name && room.configuration.enable_plugins.contains(handler.module_name))
        .map(|(_, handler)| handler)
}

#[before(ctos::Chat, module = "srvpro::core")]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn before_chat(room: &mut Room, index: usize, chat: &ctos::Chat, stop: &mut StopFlag) -> &'static str {
    if !chat.msg.starts_with("/") { return "continue" }
    let (command_name, args) = match chat.msg.split_once(' ') {
        Some((name, rest)) => (name, rest.trim()),
        None => (chat.msg.as_ref(), ""),
    };
    match command_handler(room, command_name.strip_prefix('/').unwrap_or(command_name)) {
        Some(handler) => {
            stop.0 = true;
            (handler.handler)(room, index, args);
            "cancel"
        },
        None => "continue",
    }
}

#[command]
#[register_to(CHAT_COMMANDS as ChatCommandHandler with &'static str)]
fn help(room: &mut Room, index: usize, _args: &str) {
    if let Some(player) = room.players.get(index) {
        player.send_message("Available commands", Color::Blue);
        let names: Vec<&'static str> = CHAT_COMMANDS.iter().map(|build| build())
            .filter(|(_, handler)| room.configuration.enable_plugins.contains(handler.module_name))
            .map(|(name, _)| name)
            .collect();
        for command in names {
            player.send_message(&format!("/{}", command), Color::Darkgray);    
        }   
    }
}

#[command]
#[register_to(CHAT_COMMANDS as ChatCommandHandler with &'static str)]
fn roomname(room: &mut Room, index: usize, _args: &str) {
    room.players[index].send_message(&format!("Room name: {}", room.name), Color::Babyblue);
}
