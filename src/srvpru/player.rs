use std::fmt::Display;
use std::sync::Arc;
use std::net::SocketAddr;
use std::collections::HashMap;

use tokio::task::JoinHandle;
use tokio::net::TcpStream;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;

use parking_lot::RwLock;
use parking_lot::Mutex;

use crate::srvpru::ProcessorError;
use crate::ygopro::message;
use crate::srvpru::processor::Handler;
use crate::srvpru::room::Room;
use crate::srvpru::structs::PlayerDestroy;
use crate::srvpru::structs::PlayerMove;
use crate::srvpru::server::SOCKET_SERVER;
use crate::srvpru::room::ROOMS_BY_CLIENT_ADDR;
use crate::srvpru::server;

use super::structs::StocProcessError;

lazy_static! {
    pub static ref PLAYERS: RwLock<HashMap<SocketAddr, Arc<Mutex<Player>>>> = RwLock::new(HashMap::new());
    pub static ref PLAYER_PRECURSORS: RwLock<HashMap<SocketAddr, PlayerPrecursor>> = RwLock::new(HashMap::new());
}

pub struct PlayerPrecursor {
    pub name: String,
    pub data_cache: Vec<Vec<u8>>  
}

impl PlayerPrecursor {
    fn new(name: String, client_addr: SocketAddr, data: &[u8]) {
        let mut precursor = PlayerPrecursor {
            name,
            data_cache: Vec::new()
        };
        precursor.data_cache.push(data.to_vec());
        let mut precursors = PLAYER_PRECURSORS.write();
        precursors.insert(client_addr, precursor);
    }

    fn upgrade(self, client_addr: SocketAddr, room: Arc<Mutex<Room>>) -> (Player, Vec<Vec<u8>>) {
        (
            Player { 
                room, 
                name: self.name, 
                client_addr, 
                client_stream_writer: None, 
                server_stream_writer: None, 
                reader_handler: tokio::spawn(async {}) 
            },
            self.data_cache
        )
    }
}

pub fn upgrade_player_precursor(client_addr: SocketAddr, room: Arc<Mutex<Room>>) -> Option<(Player, Vec<Vec<u8>>)> {
    let mut player_precursors = PLAYER_PRECURSORS.write();
    let player_precursor = player_precursors.remove(&client_addr)?;
    Some(player_precursor.upgrade(client_addr, room))
}

#[derive(Debug)]
pub struct Player {
    pub room: Arc<Mutex<Room>>,
    pub name: String,
    pub client_addr: SocketAddr,
    pub client_stream_writer: Option<OwnedWriteHalf>,
    pub server_stream_writer: Option<OwnedWriteHalf>,
    pub reader_handler: JoinHandle<()>,
}

impl Player {
    pub async fn new(room: &Arc<Mutex<Room>>, client_addr: SocketAddr, client_stream_writer: OwnedWriteHalf) -> Option<Arc<Mutex<Player>>> {
        let room = room.clone();
        let server_addr = room.lock().server_addr.clone().unwrap();
        let stream = TcpStream::connect(server_addr).await.map_err(|err| error!("{:}", err)).ok()?;
        let (server_stream_reader, server_stream_writer) = stream.into_split();
        let (mut player, data_cache) = upgrade_player_precursor(client_addr, room.clone())?;
        player.client_stream_writer = Some(client_stream_writer);
        player.server_stream_writer = Some(server_stream_writer);
        let player = Arc::new(Mutex::new(player));
        {
            let mut _player = player.lock();
            _player.reader_handler = Player::follow_socket(&player, client_addr, server_stream_reader); 
        }
        {
            let mut _player = player.lock();
            for data in data_cache.iter() {
                _player.server_stream_writer.as_mut().unwrap().write_all(&data).await.ok()?
            }
        }
        PLAYERS.write().insert(client_addr, player.clone());
        Some(player)
    }

    fn follow_socket(this: &Arc<Mutex<Player>>, client_addr: SocketAddr, mut server_stream_reader: OwnedReadHalf) -> JoinHandle<()> {
        let this = this.clone();
        let configuration = crate::srvpru::get_configuration();
        let timeout = tokio::time::Duration::from_secs(configuration.timeout);
        tokio::spawn(async move {
            let mut buf = [0; 10240];
            loop {
                let data = match tokio::time::timeout(timeout, server_stream_reader.read(&mut buf)).await {
                    Ok(data) => data,
                    Err(_) => {
                        server::trigger_internal(client_addr, StocProcessError { error: ProcessorError::Timeout }).await.ok();
                        break
                    }
                };
                let n = match data {
                    Ok(n) if n == 0 => break,
                    Ok(n) => n,
                    Err(e) => {
                        server::trigger_internal(client_addr, StocProcessError { error: ProcessorError::Drop(anyhow::Error::new(e)) }).await.ok();
                        break
                    }
                };
                if n > 10240 { 
                    server::trigger_internal(client_addr, StocProcessError { error: ProcessorError::Oversize }).await.ok();
                    continue 
                }
                let mut socket = this.lock().client_stream_writer.take();
                let result = SOCKET_SERVER.get().unwrap().stoc_processor.process_multiple_messages(&mut socket, &client_addr, &buf[0..n]).await;
                if let Some(socket) = socket { this.lock().client_stream_writer.replace(socket); }
                if let Err(error) = result {
                    server::trigger_internal(client_addr, StocProcessError { error }).await.ok();
                }
            }
        })
    }

    pub fn get_player(addr: &SocketAddr) -> Option<Arc<Mutex<Player>>> {
        let players = PLAYERS.read();
        players.get(addr).map(|player| player.clone())
    }

    pub fn register_handlers() {
        // Precursor producer
        Handler::follow_message::<message::CTOSPlayerInfo, _>(1, "player_precursor_producer", |context, request| Box::pin(async move {
            let name = context.get_string(&request.name, "name")?;
            PlayerPrecursor::new(name.clone(), context.addr.clone(), &context.request_buffer);
            Ok(true)
        })).register();

        // Precursor buffer
        Handler::new(2, "player_precursor_recorder", |_| true, |context| Box::pin(async move {
            let players = PLAYERS.read();
            if ! players.contains_key(&context.addr) {
                Player::buffer_data_for_precursor(context.addr, context.request_buffer);
            }
            Ok(false)
        })).register();

        Handler::follow_message::<PlayerDestroy, _>(255, "player_dropper", |_, request| Box::pin(async move {
            Player::destroy(&request.player);
            Ok(false)
        })).register();

        Handler::follow_message::<PlayerMove, _>(1, "player_mover", |_, request| Box::pin(async move {
            let mut post_player = request.post_player.lock();
            let mut new_player = request.new_player.lock();
            // It's not possible to make ygopro server change socket.
            // So just take post player as new.
            post_player.client_addr = new_player.client_addr;
            if let Some(socket) = new_player.client_stream_writer.take() {
                post_player.client_stream_writer.replace(socket);
            }
            // Global query tables.
            let mut players = PLAYERS.write();
            players.remove(&post_player.client_addr);
            players.insert(new_player.client_addr, request.post_player.clone());
            let mut rooms = crate::srvpru::room::ROOMS_BY_CLIENT_ADDR.write();
            if let Some(player) = rooms.remove(&post_player.client_addr) {
                rooms.insert(new_player.client_addr, player);
            }
            Ok(false)
        })).register();

        Handler::register_handlers("player", message::Direction::CTOS, vec!("player_precursor_producer", "player_precursor_recorder"));
        Handler::register_handlers("player", message::Direction::SRVPRU, vec!("player_dropper", "player_mover"));
    }

    pub fn buffer_data_for_precursor(client_addr: &SocketAddr, data: &[u8]) -> bool {
        let mut precursors = PLAYER_PRECURSORS.write();
        if let Some(precursor) = precursors.get_mut(client_addr) {
            precursor.data_cache.push(data.to_vec());   
            true
        }
        else { false }
    }

    pub fn destroy(this: &Arc<Mutex<Player>>) {
        let _self = this.lock();
        info!("Destroying {}.", _self.name);
        _self.reader_handler.abort();
        {
            let mut room = _self.room.lock();
            if let Some(index) = room.players.iter().position(|player| Arc::ptr_eq(player, &this)) {
                room.players.remove(index);
            }
        }
        ROOMS_BY_CLIENT_ADDR.write().remove(&_self.client_addr);
    }

    pub fn to_string(&self) -> String {
        format!("{} [{}]", self.name, self.client_addr)
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        info!("Player {} dropped.", self.to_string());
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[macro_export]
macro_rules! player_attach {
    ($( $(#[$attr:meta])* $field:ident:$type:ty ),*) => {
        #[derive(Default, Debug)]
        #[doc(hidden)]
        pub struct PlayerAttachment {
            pub $($(#[$attr])* $field: $type,)*
        }

        #[doc(hidden)]
        type SocketAddr = std::net::SocketAddr;
        
        lazy_static! {
            #[doc(hidden)]
            pub static ref PLAYER_ATTACHMENTS: parking_lot::RwLock<std::collections::HashMap<SocketAddr, PlayerAttachment>> = parking_lot::RwLock::new(std::collections::HashMap::new());
        }

        #[doc(hidden)]
        pub fn contains_player_attachment(addr: &SocketAddr) -> bool {
            let player_attacher = PLAYER_ATTACHMENTS.read();
            player_attacher.contains_key(addr)
        }

        #[doc(hidden)]
        fn _get_player_attachment<'a, 'b>(context: &crate::srvpru::Context<'a>, sure: bool) -> Option<parking_lot::MappedRwLockWriteGuard<'b, PlayerAttachment>> {
            if !contains_player_attachment(context.addr) { 
                if !sure { return None; }
                let mut player_attacher = PLAYER_ATTACHMENTS.write();
                player_attacher.insert(context.addr.clone(), PlayerAttachment::default());
            }
            Some(parking_lot::RwLockWriteGuard::map(PLAYER_ATTACHMENTS.write(), |player_attacher| player_attacher.get_mut(context.addr).unwrap()))
        }
        
        /// get attached value on player for this plugin.
        #[allow(dead_code)]
        pub fn get_player_attachment<'a, 'b>(context: &crate::srvpru::Context<'a>) -> Option<parking_lot::MappedRwLockWriteGuard<'b, PlayerAttachment>> {
            _get_player_attachment(context, false)
        }
        
        /// get attached value on player for this plugin.
        /// will panic if room don't exist.
        #[allow(dead_code)]
        pub fn get_player_attachment_sure<'a, 'b>(context: &crate::srvpru::Context<'a>) -> parking_lot::MappedRwLockWriteGuard<'b, PlayerAttachment> {
            _get_player_attachment(context, true).unwrap()
        }

        #[doc(hidden)]
        #[allow(dead_code)]
        fn drop_player_attachment(player_destroy: &crate::srvpru::structs::PlayerDestroy) -> Option<PlayerAttachment> {
            let player = player_destroy.player.lock();
            PLAYER_ATTACHMENTS.write().remove(&player.client_addr)
        }

        #[doc(hidden)]
        #[allow(dead_code)]
        fn move_player_attachment(player_move: &crate::srvpru::structs::PlayerMove) {
            let post_player = player_move.post_player.lock();
            let new_player = player_move.new_player.lock();
            let mut player_attachments = PLAYER_ATTACHMENTS.write();
            if let Some(attachment) = player_attachments.remove(&post_player.client_addr) {
                player_attachments.insert(new_player.client_addr.clone(), attachment);
            }
        }

        #[doc(hidden)]
        #[allow(dead_code)]
        fn register_player_attachment_dropper() {
            srvpru_handler!(crate::srvpru::structs::PlayerDestroy, |_, request| {
                drop_player_attachment(request);
            }).register_as(&format!("{}_player_attachment_dropper", std::path::Path::new(file!()).file_stem().unwrap().to_str().unwrap()));
        }

        #[doc(hidden)]
        #[allow(dead_code)]
        fn register_player_attachment_mover() {
            srvpru_handler!(crate::srvpru::structs::PlayerMove, |_, request| {
                move_player_attachment(request);
            }).register_as(&format!("{}_player_attachment_mover", std::path::Path::new(file!()).file_stem().unwrap().to_str().unwrap()))
        }
    };
}