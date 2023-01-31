use std::collections::HashMap;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use std::net::SocketAddr;

use async_trait::async_trait;
use anyhow::Result;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use parking_lot::RwLock;
use serde::Serialize;
use srvpru_proc_macros::before;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::task::JoinHandle;

use ygopro::constants::Colors;
use ygopro::message::client_to_server::PlayerInfo;
use ygopro::message::server_to_client;
use ygopro::serde::LengthWrapper;
use ygopro::serde::ser::serialize;

use super::FromRequest;
use super::SimplyContinue;
use super::Room;
use super::process; 
use crate::srvpro::message::{DestroyPlayer, MovePlayer};

pub static PLAYERS:           Lazy<RwLock<HashMap<SocketAddr, Arc<Mutex<Player>>>>>         = Lazy::new(|| RwLock::new(HashMap::new()));
pub static PLAYER_PRECURSORS: Lazy<RwLock<HashMap<SocketAddr, Arc<Mutex<PlayerPrecusor>>>>> = Lazy::new(|| RwLock::new(HashMap::new()));
pub static PLAYER_STREAMS:    Lazy<RwLock<HashMap<SocketAddr, OwnedWriteHalf>>>             = Lazy::new(|| RwLock::new(HashMap::new()));

#[async_trait]
pub trait PlayerLike {
    async fn write_to_server(&mut self, data: &[u8]) -> Result<()>;
    async fn write_to_client(&mut self, data: &[u8]) -> Result<()>;
}

#[async_trait]
impl PlayerLike for OwnedWriteHalf {
    async fn write_to_server(&mut self, _: &[u8]) -> Result<()> {
        Err(anyhow!("Try to write to server before Player Info"))
    }
    async fn write_to_client(&mut self, data: &[u8]) -> Result<()> {
        self.write_all(data).await?;
        Ok(())
    }
}

impl<S> FromRequest<S> for OwnedWriteHalf {
    type Rejection = SimplyContinue;

    fn from_request(request: &mut super::Bundle<S>) -> std::result::Result<Self, Self::Rejection> {
        PLAYER_STREAMS.write().remove(&request.0.socket_addr).ok_or(SimplyContinue)
    }
}

#[derive(Debug)]
pub struct PlayerPrecusor<W = OwnedWriteHalf> {
    pub name: String,
    pub origin_name: String,
    pub client_addr: SocketAddr,
    pub client_stream_writer: W,
    pub data_cache: Vec<u8>
}

impl PlayerPrecusor  {
    fn register(name: &str, client_addr: SocketAddr, client_stream_writer: OwnedWriteHalf) -> Arc<Mutex<PlayerPrecusor>> {
        let precusor = PlayerPrecusor {
            name: name.to_string(),
            origin_name: name.to_string(),
            client_addr,
            client_stream_writer,
            data_cache: Vec::new(),
        };
        let precursor = Arc::new(Mutex::new(precusor));
        PLAYER_PRECURSORS.write().insert(client_addr, precursor.clone());
        precursor
    }

    pub fn upgrade(self, room: Arc<Mutex<Room>>, server_stream: TcpStream) -> Arc<Mutex<Player>> {
        let (server_stream_reader, server_stream_writer) = server_stream.into_split();
        let player = Player {
            room,
            name: self.name,
            origin_name: self.origin_name,
            client_addr: self.client_addr,
            client_stream_writer: self.client_stream_writer,
            server_stream_writer,
            server_stream_reader_handler: tokio::spawn(async {})
        };
        let player = Arc::new(Mutex::new(player));
        PLAYERS.write().insert(self.client_addr, player.clone());
        Player::follow_reader(player.clone(), server_stream_reader);
        let mut this = player.clone();
        trace!("Write remaining {} bytes.", self.data_cache.len());
        tokio::spawn(async move { this.write_to_server(&self.data_cache).await.ok(); });
        player
    }
}

#[async_trait]
impl PlayerLike for PlayerPrecusor {
    async fn write_to_server(&mut self, data: &[u8]) -> Result<()> {
        self.data_cache.extend(data);
        Ok(())
    }
    async fn write_to_client(&mut self, data: &[u8]) -> Result<()> {
        self.client_stream_writer.write_all(&data).await?;
        Ok(())
    }
}

#[async_trait]
impl PlayerLike for Arc<Mutex<PlayerPrecusor>> {
    async fn write_to_server(&mut self, _: &[u8]) -> Result<()> {
        Err(anyhow!("Try to write to server before Join Game."))
    }
    async fn write_to_client(&mut self, data: &[u8]) -> Result<()> {
        self.lock().client_stream_writer.write_all(&data).await?;
        Ok(())
    }
}

impl<S> FromRequest<S> for PlayerPrecusor {
    type Rejection = SimplyContinue;

    fn from_request(request: &mut super::Bundle<S>) -> Result<Self, Self::Rejection> {
        let player_precusor = PLAYER_PRECURSORS.write()
            .remove(&request.0.socket_addr)
            .ok_or(SimplyContinue)?;
        match Arc::try_unwrap(player_precusor) {
            Ok(player_precursor) => Ok(player_precursor.into_inner()),
            Err(player_precusor_arc) => {
                warn!("Try to get a player precursor, but it's still referencced.");
                PLAYER_PRECURSORS.write().insert(request.0.socket_addr, player_precusor_arc);
                Err(SimplyContinue)
            }
        }
    }
}

#[derive(Debug)]
pub struct Player<W = OwnedWriteHalf> where W: AsyncWrite + Send + Unpin {
    pub room: Arc<Mutex<Room>>,
    pub name: String,
    pub origin_name: String,
    pub client_addr: SocketAddr,
    pub client_stream_writer: W,
    pub server_stream_writer: W,
    pub server_stream_reader_handler: JoinHandle<()>,
}

impl Player {
    pub fn follow_reader<R>(this: Arc<Mutex<Self>>, mut reader: R)
        where R: AsyncRead + Send + Unpin + 'static {
        let client_addr = this.lock().client_addr;
        let name = this.lock().name.clone();
        let join_handle = tokio::spawn(async move {
            let mut buf = [0u8; 10240];
            loop {
                let n = match reader.read(&mut buf).await {
                    Ok(n) if n == 0 => break,
                    Ok(n) => n,
                    Err(_) => continue
                };
                process::<server_to_client::MessageType>(client_addr, &mut buf[0..n]).await;
            }
            info!("Player {:} stream to server Drop", name);
        });
        this.lock().server_stream_reader_handler = join_handle;
    }
}

impl<S> FromRequest<S> for Arc<Mutex<Player>> {
    type Rejection = SimplyContinue;

    fn from_request(request: &mut super::Bundle<S>) -> Result<Self, Self::Rejection> {
        match &request.1.player {
            Some(player) => Ok(player.clone()),
            None => match PLAYERS.read().get(&request.0.socket_addr) {
                Some(player) => {
                    request.1.player = Some(player.clone());
                    Ok(player.clone())
                },
                None => Err(SimplyContinue),
            }
        }
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Player {}", self.origin_name)
    }
}

#[async_trait]
impl PlayerLike for Player {
    async fn write_to_server(&mut self, data: &[u8]) -> Result<()> {
        self.server_stream_writer.write_all(data).await?;
        Ok(())
    }
    async fn write_to_client(&mut self, data: &[u8]) -> Result<()> {
        self.client_stream_writer.write_all(data).await?;
        Ok(())
    }
}

#[async_trait]
impl PlayerLike for Arc<Mutex<Player>> {
    async fn write_to_server(&mut self, data: &[u8]) -> Result<()> {
        self.lock().server_stream_writer.write_all(data).await?;
        Ok(())
    }
    async fn write_to_client(&mut self, data: &[u8]) -> Result<()> {
        self.lock().client_stream_writer.write_all(data).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum PlayerEnum {
    Stream(OwnedWriteHalf),
    Precursor(Arc<Mutex<PlayerPrecusor>>),
    Player(Arc<Mutex<Player>>),
    None
}

#[async_trait]
impl PlayerLike for PlayerEnum {
    async fn write_to_server(&mut self, data: &[u8]) -> Result<()> {
        match self {
            PlayerEnum::Stream(stream) => stream.write_to_server(data).await,
            PlayerEnum::Precursor(precursor) => precursor.write_to_server(data).await,
            PlayerEnum::Player(player) => player.write_to_server(data).await,
            PlayerEnum::None => Err(anyhow!("Try to write to client inner scratch"))
        }
    }
    async fn write_to_client(&mut self, data: &[u8]) -> Result<()> {
        match self {
            PlayerEnum::Player(player) => player.write_to_client(data).await,
            _ => Err(anyhow!("Try to write to server before Join Game"))
        }
    }
}

pub fn get_player_enum(client_addr: SocketAddr) -> PlayerEnum {
     if let Some(player) = PLAYERS.read().get(&client_addr) {
        return PlayerEnum::Player(player.clone());
    }
    if let Some(player_precursor) = PLAYER_PRECURSORS.read().get(&client_addr) {
        return PlayerEnum::Precursor(player_precursor.clone());
    }
    if let Some(_) = PLAYER_STREAMS.read().get(&client_addr) {
        // warn!("Try to write to a raw stream.")
    }
    PlayerEnum::None 
}

pub fn remove_player_enum(client_addr: SocketAddr) -> PlayerEnum {
    if let Some(player) = PLAYERS.write().remove(&client_addr) {
        return PlayerEnum::Player(player);
    }
    if let Some(player_precursor) = PLAYER_PRECURSORS.write().remove(&client_addr) {
        return PlayerEnum::Precursor(player_precursor);
    }
    if let Some(stream) = PLAYER_STREAMS.write().remove(&client_addr) {
        return PlayerEnum::Stream(stream);
    }
    PlayerEnum::None  
}

pub fn register_stream(client_addr: SocketAddr, stream: OwnedWriteHalf) {
    PLAYER_STREAMS.write().insert(client_addr, stream);
}

impl Player {
    pub async fn send_to_server<M: Serialize>(&mut self, message: &M) -> anyhow::Result<()> {
        let bytes = serialize(message)?;
        self.server_stream_writer.write_all(&bytes).await?;
        Ok(())
    }

    pub async fn send_to_client<M: Serialize>(&mut self, message: &M) -> anyhow::Result<()> {
        let bytes = serialize(message)?;
        self.client_stream_writer.write_all(&bytes).await?;
        Ok(())
    }

    pub async fn send_chat_to_server(&mut self, message: String) -> anyhow::Result<()> {
        let chat: ygopro::message::client_to_server::Chat = message.into();
        self.send_to_server(&LengthWrapper(ygopro::message::client_to_server::MessageEnum::Chat(chat))).await
    }

    pub async fn send_chat_to_client(&mut self, color: Colors, message: String) -> anyhow::Result<()> {
        let chat = ygopro::message::server_to_client::Chat {
            name: color as u16,
            msg: message.into()
        };
        self.send_to_client(&LengthWrapper(ygopro::message::server_to_client::MessageEnum::Chat(chat))).await
    }
}

#[before]
fn new_player(message: &PlayerInfo, socket_addr: SocketAddr, client_stream_writer: OwnedWriteHalf) {
    PlayerPrecusor::register(message.name.deref(), socket_addr, client_stream_writer);
}

#[before]
fn drop_player(message: &DestroyPlayer) {
    let player = message.player.lock();
    player.server_stream_reader_handler.abort();
    PLAYERS.write().remove(&player.client_addr);
    if let Some(room) = Room::get_by_client_addr(&player.client_addr) {
        room.lock().players.retain(|x| ! Arc::ptr_eq(x, &message.player));
    }
}

#[before]
fn move_player(message: &MovePlayer) {
    
}
