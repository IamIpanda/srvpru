#![allow(dead_code)]
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use anyhow::Result;

use crate::ygopro::message::*;
use crate::ygopro::message::Struct;
use crate::ygopro::constants::Colors;
use crate::srvpru::processor::*;
use crate::srvpru::player::PLAYERS;
use crate::srvpru::player::Player;
use crate::srvpru::room::ROOMS_BY_SERVER_ADDR;
use crate::srvpru::room::ROOMS_BY_CLIENT_ADDR;
use crate::srvpru::i18n;

use super::Room;
use super::structs::SRVPRUMessageType;

pub async fn send<T: Struct + MappedStruct + serde::Serialize>(socket: &mut OwnedWriteHalf, obj: &T) -> Result<()> {
    send_data(socket, &T::message(), &(bincode::serialize(&obj)?)).await
}

pub async fn send_empty(socket: &mut OwnedWriteHalf, message_type: &MessageType) -> Result<()> {
    send_data(socket, message_type, &[]).await
}

pub async fn send_data(socket: &mut OwnedWriteHalf, message_type: &MessageType, data: &[u8]) -> Result<()> {
    socket.write_all(&wrap_data(message_type, data)).await?;
    Ok(())
}

pub async fn send_chat(socket: &mut OwnedWriteHalf, message: &str, region: &str, color: Colors) -> Result<()> {
    send_chat_raw(socket, &("[Server]: ".to_string() + &i18n::render(message, region)), color).await
}

pub async fn send_chat_raw(socket: &mut OwnedWriteHalf, message: &str, color: Colors) -> Result<()> {
    send(socket, &crate::ygopro::message::STOCChat {
        name: color.into(),
        msg: cast_to_c_array(&message)
    }).await
}

impl<'a> Context<'a> {
    pub fn block_message(&mut self) -> Result<bool> {
        self.response = None;
        Ok(true)
    }

    pub fn cast_request_to_type<F: Struct>(&mut self) -> Option<&mut F> {
        let _box = self.request.as_mut()?;
        _box.downcast_mut::<F>()
    }

    pub async fn send(&mut self, obj: &(impl Struct + MappedStruct + serde::Serialize)) -> Result<()> {
        let socket = self.socket.as_mut().ok_or(anyhow!("socket already taken."))?;
        send(socket, obj).await
    }

    pub async fn send_empty(&mut self, message_type: &MessageType) -> Result<()> {
        let socket = self.socket.as_mut().ok_or(anyhow!("socket already taken."))?;
        send_empty(socket, message_type).await
    }

    pub async fn send_data(&mut self, message_type: &MessageType, data: &[u8]) -> Result<()> {
        let socket = self.socket.as_mut().ok_or(anyhow!("socket already taken."))?;
        send_data(socket, message_type, data).await 
    }

    pub async fn send_chat(&mut self, message: &str, color: Colors) -> Result<()> {
        let socket = self.socket.as_mut().ok_or(anyhow!("socket already taken."))?;
        send_chat(socket, message, "zh-cn", color).await 
    }

    pub async fn send_chat_raw(&mut self, message: &str, color: Colors) -> Result<()> {
        let socket = self.socket.as_mut().ok_or(anyhow!("socket already taken."))?;
        send_chat_raw(socket, message, color).await 
    }

    pub fn get_room(&self) -> Option<Arc<Mutex<Room>>> {
        match self.direction {
            Direction::SRVPRU if self.message_type == Some(MessageType::SRVPRU(SRVPRUMessageType::RoomCreated))
                              || self.message_type == Some(MessageType::SRVPRU(SRVPRUMessageType::RoomDestroy)) => {
                let rooms = ROOMS_BY_SERVER_ADDR.read();
                rooms.get(self.addr).map(|room| room.clone()) 
            },
            _ => {
                let rooms = ROOMS_BY_CLIENT_ADDR.read();
                let room = rooms.get(self.addr).map(|room| room.clone());
                if room.is_some() { return room; }

                //if self.parameters.contains_key("room_name") { return Room::find_room_by_name(self.parameters.get("room_name").unwrap()) }

                if self.message_type == Some(MessageType::CTOS(CTOSMessageType::JoinGame)) {
                    let room_name = if let Some(request) = self.request.as_ref() {
                        let join_game = request.downcast_ref::<CTOSJoinGame>();
                        if join_game.is_none() { return None }
                        join_game.unwrap().pass
                    } else { bincode::deserialize::<CTOSJoinGame>(self.request_buffer).ok()?.pass };
                    let room_name = crate::ygopro::message::cast_to_string(&room_name)?;
                    let room = Room::find_room_by_name(&room_name);
                    return room;
                }
                None
            },
        }
    }

    pub fn get_player(&self) -> Option<Arc<Mutex<Player>>> {
        let players = PLAYERS.read();
        players.get(self.addr).map(|player| player.clone())
    }

    pub async fn send_chat_to_room(&mut self, message: &str, color: Colors) -> Result<()> {
        let room = self.get_room().ok_or(anyhow!("Cannot find the room"))?;
        let mut _room = room.lock();
        _room.send_chat(message, "zh-cn", color).await?;
        self.send_chat(message, color).await?;
        Ok(())
    }

    pub async fn send_raw_chat_to_room(&mut self, message: &str, color: Colors) -> Result<()> {
        let room = self.get_room().ok_or(anyhow!("Cannot find the room"))?;
        let mut _room = room.lock();
        _room.send_chat_raw(message, color).await?;
        self.send_chat(message, color).await?;
        Ok(())
    }
    
    pub fn get_string(&mut self, str: &[u16], name: &'static str) -> Result<&mut String> {
        if ! self.parameters.contains_key(name) {
            let string = crate::ygopro::message::cast_to_string(&str).ok_or(anyhow!("Failed to get string."))?;
            self.parameters.insert(name.to_string(), string);
        }
        Ok(self.parameters.get_mut(name).unwrap())
    }
}

impl Player {
    pub async fn send_chat(&mut self, message: &str, color: Colors) -> Result<()> {
        let socket = self.client_stream_writer.as_mut().ok_or(anyhow!("Socket already taken."))?;
        send_chat(socket, message, "zh-cn", color).await
    }

    pub async fn send_chat_raw(&mut self, message: &str, color: Colors) -> Result<()> {
        let socket = self.client_stream_writer.as_mut().ok_or(anyhow!("Socket already taken."))?;
        send_chat_raw(socket, message, color).await
    }

}

impl Room {
    pub async fn send_chat(&mut self, message: &str, region: &str, color: Colors) -> Result<()> {
        let message = &("[Server]: ".to_string() + &i18n::render(message, region));
        self.send_chat_raw(message, color).await
    }

    pub async fn send_chat_raw(&mut self, message: &str, color: Colors) -> Result<()> {
        for player in self.players.iter() {
            let mut player = player.lock();
            if let Some(socket) = player.client_stream_writer.as_mut() {
                send_chat_raw(socket, message, color).await.ok(); // Sender itself find stream already taken. Don't matter.
            }
        }
        Ok(())
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