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

/// Send a struct to target socket.
pub async fn send<T: Struct + MappedStruct + serde::Serialize>(socket: &mut OwnedWriteHalf, obj: &T) -> Result<()> {
    send_raw_data(socket, T::message(), &(bincode::serialize(&obj)?)).await
}

/// Send data to target socket, adding a length header and type header.
pub async fn send_raw_data(socket: &mut OwnedWriteHalf, message_type: MessageType, data: &[u8]) -> Result<()> {
    socket.write_all(&wrap_data(message_type, data)).await?;
    Ok(())
}

/// Generate a stoc::Chat, with a [server] prefix, and translate all placeholders.
pub fn generate_chat(message: &str, color: Colors, region: &str) -> stoc::Chat {
    generate_raw_chat(&("[Server]: ".to_string() + &i18n::render(message, region)), color)
}

/// Generate a stoc::Chat, without any process.
pub fn generate_raw_chat(message: &str, color: Colors) -> stoc::Chat {
    stoc::Chat { name: color as u16, msg: crate::ygopro::message::string::cast_to_c_array(message) }
}

impl<'a> Context<'a> {
    pub fn block_message(&mut self) -> Result<bool> {
        self.response = None;
        Ok(true)
    }

    pub fn cast_request_to_type<T: Struct>(&self) -> Option<&T> {
        let _box = self.request.as_ref()?;
        _box.downcast_ref()
    }

    pub fn cast_request_to_mut_type<T: Struct>(&mut self) -> Option<&mut T> {
        let _box = self.request.as_mut()?;
        _box.downcast_mut()
    }

    pub async fn send(&mut self, obj: &(impl Struct + MappedStruct + serde::Serialize)) -> Result<()> {
        let socket = self.socket.as_mut().ok_or(anyhow!("Socket already taken."))?;
        send(socket, obj).await
    }

    pub async fn send_back(&mut self, obj: &(impl Struct + MappedStruct + serde::Serialize)) -> Result<()> {
        let player = self.get_player().ok_or(anyhow!("Cannot get player"))?;
        let mut _player = player.lock();
        let socket_wrapper = match self.direction {
            Direction::CTOS => _player.client_stream_writer.as_mut(),
            Direction::STOC => _player.server_stream_writer.as_mut(),
            Direction::SRVPRU => { return Err(anyhow!("Wrong direction.")); }
        };
        let socket = socket_wrapper.ok_or(anyhow!("Socket already taken."))?;
        send(socket, obj).await
    }

    pub async fn send_to_room(&mut self, obj: &(impl Struct + MappedStruct + serde::Serialize)) -> Result<()> {
        let room = self.get_room().ok_or(anyhow!("Cannot find the room"))?;
        let mut _room = room.lock();
        _room.send(obj).await;
        if self.direction == Direction::STOC { // STOC, a player will have socket already taken.
            if let Some(socket) = self.socket.as_mut() {
                send(socket, obj).await?;
            }
        }
        Ok(())
    }

    pub fn get_region(&self) -> &'static str {
        self.get_player().map(|player| player.lock().region).unwrap_or("zh-cn")
    }

    pub fn get_room(&self) -> Option<Arc<Mutex<Room>>> {
        match self.direction {
            Direction::SRVPRU if self.message_type == Some(MessageType::SRVPRU(srvpru::MessageType::RoomCreated))
                              || self.message_type == Some(MessageType::SRVPRU(srvpru::MessageType::RoomDestroy)) => {
                Room::get_room_by_server_addr(*self.addr)
            },
            _ => Room::get_room_by_client_addr(*self.addr),
        }
    }

    pub fn get_room_in_join_game(&mut self, request: &ctos::JoinGame) -> Option<Arc<Mutex<Room>>> {
        Room::get_room(self.get_string(&request.pass, "pass").ok()?)
    }

    pub fn get_player(&self) -> Option<Arc<Mutex<Player>>> {
        if self.message_type == Some(MessageType::SRVPRU(srvpru::MessageType::PlayerDestroy)) {
            return self.cast_request_to_type::<PlayerDestroy>().map(|req| req.player.clone());
        }
        Player::get_player(*self.addr)
    }

    pub fn get_string(&mut self, str: &[u16], name: &'static str) -> Result<&mut String> {
        Ok(self.parameters.entry(name.to_string()).or_insert( crate::ygopro::message::string::cast_to_string(&str).ok_or(anyhow!("Failed to cast bytes to string."))?))
    }
}

impl Player {
    pub async fn send_to_client(&mut self, obj: &(impl Struct + MappedStruct + serde::Serialize)) -> Result<()> {
        let socket = self.client_stream_writer.as_mut().ok_or(anyhow!("Socket already taken."))?;
        send(socket, obj).await
    }

    pub async fn send_to_server(&mut self, obj: &(impl Struct + MappedStruct + serde::Serialize)) -> Result<()> {
        let socket = self.server_stream_writer.as_mut().ok_or(anyhow!("Socket already taken."))?;
        send(socket, obj).await
    }

}

impl Room {
    pub async fn send(&self, obj: &(impl Struct + MappedStruct + serde::Serialize)) {
        for player in self.players.iter() {
            let mut player = player.lock();
            if let Some(socket) = player.client_stream_writer.as_mut() {
                send(socket, obj).await.ok(); // Sender itself find stream already taken. Don't matter.
            }
        }
    }
}

#[macro_export]
macro_rules! unwrap_or_return {
    ( $e:expr ) => {
        match $e {
            Some(x) => x,
            None => return Ok(false),
        }
    }
}


/*
pub struct WrappedContext<'this, 'context, Request: Struct + MappedStruct, Attachment> {
    context: &'this mut Context<'context>,
    request: &'this mut Request,
    attachment: Attachment
}

impl<'this, 'context, Request: Struct + MappedStruct, Attachment> WrappedContext<'this, 'context, Request, Attachment> {
    pub fn new<F>(context: &mut Context<'context>, attachment_generator: Option<F>) where F: for <'a> Fn(&'a mut Context<'context>) -> Attachment, {
        let attachment = attachment_generator.as_ref().unwrap()(context);
    }
}
*/