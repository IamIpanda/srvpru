#![allow(dead_code)]
use std::sync::Arc;

use parking_lot::Mutex;
use srvpru::PlayerDestroy;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use anyhow::Result;

use crate::ygopro::Colors;
use crate::ygopro::message::Direction;
use crate::ygopro::message::Struct;
use crate::ygopro::message::ctos;
use crate::ygopro::message::stoc;
use crate::ygopro::message::srvpru;
use crate::ygopro::message::MappedStruct;
use crate::ygopro::message::MessageType;
use crate::ygopro::message::generate::wrap_data;

use crate::srvpru::i18n;
use crate::srvpru::Room;
use crate::srvpru::Player;
use crate::srvpru::Context;
use crate::srvpru::ProcessorError;


#[derive(Error, Debug)]
pub enum CommonError {
    #[error("Socket already taken")]
    SocketTaken,
    #[error("Try to get a player not exist")]
    PlayerNotExist,
    #[error("Try to get a room not exist")]
    RoomNotExist,
    #[error("Try to normalize a illegal string")]
    IllegalString,
    #[error("Try to get a parameter in wrong type")]
    IllegalType
}

/// Send a struct to target socket.
pub async fn send<T: Struct + MappedStruct + serde::Serialize>(socket: &mut OwnedWriteHalf, obj: &T) -> Result<()> {
    send_raw_data(socket, T::message(), &(bincode::serialize(&obj)?)).await
}

/// Send data to target socket, adding a length header and type header.
pub async fn send_raw_data(socket: &mut OwnedWriteHalf, message_type: MessageType, data: &[u8]) -> Result<()> {
    socket.write_all(&wrap_data(message_type, data)).await?;
    Ok(())
}

/// Generate a [stoc::Chat], with a "\[Server\]: " prefix, and translate all placeholders.
pub fn generate_chat(message: &str, color: Colors, region: &str) -> stoc::Chat {
    generate_raw_chat(&("[Server]: ".to_string() + &i18n::render(message, region)), color)
}

/// Generate a [stoc::Chat], without any process.
pub fn generate_raw_chat(message: &str, color: Colors) -> stoc::Chat {
    stoc::Chat { name: color as u16, msg: crate::ygopro::message::string::cast_to_c_array(message) }
}

impl<'a> Context<'a> {
    /// Set `block_message` to true, and stop process.
    pub fn block_message(&mut self) -> Result<bool> {
        self.block_message = true;
        Ok(true)
    }

    /// Send a message to client, and then refuse client join game. \
    /// **MUST** call before room created.
    pub async fn refuse_join_game(&mut self, message: Option<&str>) -> Result<bool> {
        let refuse_message = crate::ygopro::message::stoc::ErrorMessage{ msg: crate::ygopro::ErrorMessage::Joinerror, align: [0; 3], code: 2 };
        if let Some(message) = message {
            self.send(&struct_sequence! [
                generate_chat(message, Colors::Red, self.get_region()),
                refuse_message
            ]).await.ok();
        }
        else {
            self.send(&refuse_message).await.ok();
        }
        Err(ProcessorError::Abort.into())
    }

    /// Cast `message` to target type.
    pub fn cast_message_to_type<T: Struct>(&self) -> Option<&T> {
        let _box = self.message.as_ref()?;
        _box.downcast_ref()
    }

    /// Cast `message` to target type.
    pub fn cast_message_to_mut_type<T: Struct>(&mut self) -> Option<&mut T> {
        let _box = self.message.as_mut()?;
        _box.downcast_mut()
    }

    /// Send a struct via `socket`.
    pub async fn send(&mut self, obj: &(impl Struct + MappedStruct + serde::Serialize)) -> Result<()> {
        let socket = self.socket.as_mut().ok_or(anyhow!(CommonError::SocketTaken))?;
        send(socket, obj).await
    }

    /// Send a struct to who send this message.
    pub async fn send_back(&mut self, obj: &(impl Struct + MappedStruct + serde::Serialize)) -> Result<()> {
        let player = self.get_player().ok_or(CommonError::PlayerNotExist)?.clone();
        let mut _player = player.lock();
        let socket_wrapper = match self.direction {
            Direction::CTOS => _player.client_stream_writer.as_mut(),
            Direction::STOC => _player.server_stream_writer.as_mut(),
            Direction::SRVPRU => { return Err(anyhow!("Wrong direction.")); }
        };
        let socket = socket_wrapper.ok_or(anyhow!("Socket already taken."))?;
        send(socket, obj).await
    }

    /// Send a struct to all memeber of this room.
    pub async fn send_to_room(&mut self, obj: &(impl Struct + MappedStruct + serde::Serialize)) -> Result<()> {
        let room = self.get_room().ok_or(CommonError::RoomNotExist)?.clone();
        let mut _room = room.lock();
        _room.send(obj).await;
        if self.direction == Direction::STOC { // STOC, a player will have socket already taken.
            if let Some(socket) = self.socket.as_mut() {
                send(socket, obj).await?;
            }
        }
        Ok(())
    }

    /// Get region of Player who send or will receive this message.
    pub fn get_region(&self) -> &'static str {
        self.get_player().map(|player| player.lock().region).unwrap_or("zh-cn")
    }

    /// Get [Room] of [Player] who send or will receive this message.
    pub fn get_room(&self) -> Option<&Arc<Mutex<Room>>> {
        self.room.get_or_try_init(move || {
            match self.direction {
                Direction::SRVPRU if self.message_type == Some(MessageType::SRVPRU(srvpru::MessageType::RoomCreated))
                                  || self.message_type == Some(MessageType::SRVPRU(srvpru::MessageType::RoomDestroy)) => {
                    Room::get_room_by_server_addr(self.addr).ok_or(CommonError::RoomNotExist)
                },
                _ => Room::get_room_by_client_addr(self.addr).ok_or(CommonError::RoomNotExist),
            }
        }).ok()
    }

    /// Get [Room] of [Player] who send this message, when processing a [ctos::JoinGame].
    pub fn get_room_in_join_game(&mut self, request: &ctos::JoinGame) -> Option<Arc<Mutex<Room>>> {
        if let Some(room) = self.room.get() { return Some(room.clone()); }
        let room = Room::get_room(self.get_string(&request.pass, "pass").ok()?)?;
        Some(self.room.get_or_init(move || room).clone())
    }

    /// Get [Player] who send or will receive this message.
    pub fn get_player(&self) -> Option<&Arc<Mutex<Player>>> {
        self.player.get_or_try_init(move || {
            if self.message_type == Some(MessageType::SRVPRU(srvpru::MessageType::PlayerDestroy)) {
                Ok(self.cast_message_to_type::<PlayerDestroy>().ok_or(CommonError::IllegalType)?.player.clone())
            }
            else { 
                Player::get_player(self.addr).ok_or(CommonError::PlayerNotExist) 
            }
        }).ok()
    }
 
    pub fn set_parameter<T: std::any::Any + Send + Sync>(&mut self, name: &'static str, parameter: T) {
        self.parameters.insert(name, Box::new(parameter) as Box<dyn std::any::Any + Send + Sync>);
    }

    pub fn get_parameter<T: std::any::Any + Send + Sync>(&mut self, name: &'static str) -> Option<&mut T> {
        self.parameters.get_mut(name).map(|parameter| parameter.downcast_mut::<T>()).flatten()
    }

    pub fn get_or_insert_parameter<T: std::any::Any + Send + Sync, F: FnOnce() -> T >(&mut self, name: &'static str, generator: F) -> &mut T {
        self.parameters.entry(name).or_insert_with(move || { Box::new(generator()) as Box<dyn std::any::Any + Send + Sync> }).downcast_mut::<T>().unwrap()
    }

    pub fn get_string(&mut self, str: &[u16], name: &'static str) -> core::result::Result<&mut String, CommonError> {
        Ok(self.parameters.entry(name)
            .or_insert( Box::new(crate::ygopro::message::string::cast_to_string(&str).ok_or(CommonError::IllegalString)?) as Box<dyn std::any::Any + Send + Sync>)
            .downcast_mut().ok_or(CommonError::IllegalType)?)
    }
}

impl Player {
    /// Send a message to client.
    pub async fn send_to_client(&mut self, obj: &(impl Struct + MappedStruct + serde::Serialize)) -> Result<()> {
        let socket = self.client_stream_writer.as_mut().ok_or(anyhow!("Socket already taken."))?;
        send(socket, obj).await
    }

    /// Send a message to ygopro server.
    pub async fn send_to_server(&mut self, obj: &(impl Struct + MappedStruct + serde::Serialize)) -> Result<()> {
        let socket = self.server_stream_writer.as_mut().ok_or(anyhow!("Socket already taken."))?;
        send(socket, obj).await
    }

}

impl Room {
    /// Send a message to each [Player] of current room. \
    /// Will ignore any error or socket taken.
    pub async fn send(&self, obj: &(impl Struct + MappedStruct + serde::Serialize)) {
        for player in self.players.iter() {
            let mut player = player.lock();
            // Sender itself find stream already taken. Don't matter.
            if let Some(socket) = player.client_stream_writer.as_mut() {
                send(socket, obj).await.ok(); 
            }
        }
    }

    /// Send a chat to each [Player] of current room. \
    /// Will transform all blocks to fit that Player's region.
    pub async fn send_chat(&self, template: &str, color: Colors) {
        for player in self.players.iter() {
            let mut player = player.lock();
            let region = player.region;
            if let Some(socket) = player.client_stream_writer.as_mut() {
                send(socket, &generate_chat(template, color, region)).await.ok();
            }
        }
    }
}

/// Return `Ok(false)` if parameter is `None`.
#[macro_export]
macro_rules! unwrap_or_return {
    ( $e:expr ) => {
        match $e {
            Some(x) => x,
            None => return Ok(false),
        }
    }
}
