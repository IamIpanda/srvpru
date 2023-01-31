use std::fmt::format;
use std::net::SocketAddr;
use std::sync::Arc;

use parking_lot::Mutex;
use ygopro::message::MessageType;

use crate::srvpro::Player; 
use crate::srvpro::Room;
use crate::srvpro::message::AnyMessage;

#[before(AnyMessage, priority = 0)]
fn debug(message_type: MessageType, player: Arc<Mutex<Player>>, room: Arc<Mutex<Room>>, client_addr: SocketAddr) {
    let player_name = player.lock().name.clone();
    let room = room.lock();
    let server_str = match &room.server {
        Some(server) => format!("{:}", server),
        None => "Not spawned".to_string()
    };
    let message = match message_type {
        MessageType::STOC(message_type) => format!("STOC  Message [{:}]{:} <- [{:}]{:} {:}", player_name, client_addr, room.name, server_str, message_type),
        MessageType::CTOS(message_type) => format!("CTOS  Message [{:}]{:} -> [{:}]{:} {:}", player_name, client_addr, room.name, server_str, message_type),
        MessageType::GM(message_type)   => format!("STOC  Message [{:}]{:} <= [{:}]{:} {:}", player_name, client_addr, room.name, server_str, message_type),
        MessageType::Other(_, message_type)      => format!("SRVPR Message [{:}]{:} -> [{:}]{:} {:}", player_name, client_addr, room.name, server_str, message_type),
    };
    debug!("{}", message);
}

#[before(AnyMessage, priority = 0)]
fn debug2(message_type: MessageType, player: Arc<Mutex<Player>>, room: Arc<Mutex<Room>>, client_addr: SocketAddr) {
}
