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

use crate::ygopro::message;
use crate::ygopro::message::ctos;
use crate::ygopro::message::srvpru;

use crate::srvpru::ListenError;
use crate::srvpru::processor::Handler;
use crate::srvpru::room::Room;
use crate::srvpru::room::ROOMS_BY_CLIENT_ADDR;
use crate::srvpru::server;

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

    fn _upgrade(self, client_addr: SocketAddr, room: Arc<Mutex<Room>>) -> (Player, Vec<Vec<u8>>) {
        (
            Player { 
                room, 
                name: self.name, 
                client_addr, 
                client_stream_writer: None, 
                server_stream_writer: None, 
                reader_handler: tokio::spawn(async {}),
                
                region: "zh-cn",
                timeout_exempt: false
            },
            self.data_cache
        )
    }

    pub fn upgrade(client_addr: SocketAddr, room: Arc<Mutex<Room>>) -> Option<(Player, Vec<Vec<u8>>)> {
        let mut player_precursors = PLAYER_PRECURSORS.write();
        let player_precursor = player_precursors.remove(&client_addr)?;
        Some(player_precursor._upgrade(client_addr, room))
    }

    pub fn exist(client_addr: SocketAddr) {
        PLAYER_PRECURSORS.read().contains_key(&client_addr);
    }
}


#[derive(Debug)]
pub struct Player {
    pub room: Arc<Mutex<Room>>,
    pub name: String,
    pub client_addr: SocketAddr,
    pub client_stream_writer: Option<OwnedWriteHalf>,
    pub server_stream_writer: Option<OwnedWriteHalf>,
    pub reader_handler: JoinHandle<()>,

    // These fields are not core.
    // They should be in plugin recorders,
    // But it's too wisely used in plugins, 
    // or too complicated to make it in plugin.
    pub region: &'static str,
    pub timeout_exempt: bool
}

impl Player {
    pub fn init() -> anyhow::Result<()> {
        Player::register_handlers();
        Ok(())
    }

    pub async fn new(room: &Arc<Mutex<Room>>, client_addr: SocketAddr, client_stream_writer: OwnedWriteHalf) -> anyhow::Result<Arc<Mutex<Player>>> {
        let room = room.clone();
        let server_addr = room.lock().server_addr.clone().ok_or(anyhow!("Room don't have a server addr"))?;
        let stream = TcpStream::connect(server_addr).await?;
        let (server_stream_reader, mut server_stream_writer) = stream.into_split();
        let (mut player, data_cache) = PlayerPrecursor::upgrade(client_addr, room.clone()).ok_or(anyhow!("Cannot find precursor"))?;
        for data in data_cache.iter() {
            server_stream_writer.write_all(&data).await?;
        }
        player.client_stream_writer = Some(client_stream_writer);
        player.server_stream_writer = Some(server_stream_writer);
        let player = Arc::new(Mutex::new(player));
        player.lock().reader_handler = Player::follow_socket(&player, client_addr, server_stream_reader); 
        PLAYERS.write().insert(client_addr, player.clone());
        Ok(player)
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
                        server::trigger_internal(client_addr, srvpru::StocListenError { error: ListenError::Timeout }).await.ok();
                        break
                    }
                };
                let n = match data {
                    Ok(n) if n == 0 => break,
                    Ok(n) => n,
                    Err(e) => {
                        server::trigger_internal(client_addr, srvpru::StocListenError { error: ListenError::Drop(anyhow::Error::new(e)) }).await.ok();
                        break
                    }
                };
                if n > 10240 { 
                    server::trigger_internal(client_addr, srvpru::StocListenError { error: ListenError::Oversize }).await.ok();
                    continue 
                }
                let mut socket = this.lock().client_stream_writer.take();
                let result = crate::srvpru::get_server().stoc_processor.process_multiple_messages(&mut socket, &client_addr, &buf[0..n]).await;
                if let Some(socket) = socket { this.lock().client_stream_writer.replace(socket); }
                if let Err(error) = result {
                    server::trigger_internal(client_addr, srvpru::StocProcessError { error }).await.ok();
                }
            }
        })
    }

    fn register_handlers() {
        // Precursor producer
        Handler::follow_message::<ctos::PlayerInfo, _>(10, "player_precursor_producer", |context, request| Box::pin(async move {
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

        Handler::follow_message::<srvpru::PlayerDestroy, _>(100, "player_dropper", |_, request| Box::pin(async move {
            Player::destroy(&request.player);
            Ok(false)
        })).register();

        Handler::follow_message::<srvpru::PlayerMove, _>(1, "player_mover", |_, request| Box::pin(async move {
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

    // ----------------------------------------------------------------------------------------------------
    /// ## destroy
    // ----------------------------------------------------------------------------------------------------
    /// Try to drop that player.
    /// 
    /// `destroy` do following things:
    /// - stop read from ygopro server.
    /// - remove itself from room.
    /// - remove itself from room query table.
    /// 
    /// `destroy` **WON'T** do following things:
    /// - remove itself from PLAYERS. (Done by Server)
    /// - drop itself. (RC)
    // ----------------------------------------------------------------------------------------------------
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

    // ----------------------------------------------------------------------------------------------------
    /// ## expel 
    // ----------------------------------------------------------------------------------------------------
    /// Kick player out.
    /// return `true` if it may success.
    /// 
    /// There is no way to break the `client_stream_reader`, it's owned by socket `Server`.
    /// Also we don't want to force abort client reader thread, as there is cleanup code after the loop.
    /// But according to tokio designs, if the writer half is drop, the stream will shutdown normally.
    /// The writer was already stolen to Player, so you can stop it by drop `client_stream_writer`.
    /// Then, client stream reader will stop, raise an zero length, and the gear moves.
    // ------------------------------------------------------------------------------------------------------
    pub fn expel(&mut self) -> bool {
        self.client_stream_writer.take().is_some()
        // No need to shut down here, tokio document say:
        // `Dropping the write half will shutdown the write half of the TCP stream.`
    }

    pub fn get_player(client_addr: SocketAddr) -> Option<Arc<Mutex<Player>>> {
        return PLAYERS.read().get(&client_addr).map(|player| player.clone())
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        info!("Player {} dropped.", self.to_string());
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[macro_export]
macro_rules! player_attach {
    ($( $(#[$attr:meta])* $field:ident:$type:ty ),*) => {
        #[derive(Default, Debug)]
        #[doc(hidden)]
        pub struct PlayerAttachment {
            $($(#[$attr])* pub $field: $type,)*
        }

        #[doc(hidden)]
        type SocketAddr = std::net::SocketAddr;
        
        lazy_static! {
            #[doc(hidden)]
            pub static ref PLAYER_ATTACHMENTS: parking_lot::RwLock<std::collections::HashMap<SocketAddr, PlayerAttachment>> = parking_lot::RwLock::new(std::collections::HashMap::new());
        }

        #[doc(hidden)]
        pub fn contains_player_attachment(addr: &SocketAddr) -> bool {
            PLAYER_ATTACHMENTS.read().contains_key(addr)
        }

        #[doc(hidden)]
        #[allow(dead_code)]
        fn insert_player_attachment<'a>(context: &crate::srvpru::Context<'a>, $($field: $type,)*) {
            PLAYER_ATTACHMENTS.write().insert(context.addr.clone(), PlayerAttachment { $($field,)* });
        }

        #[doc(hidden)]
        fn _get_player_attachment<'a, 'b>(context: &crate::srvpru::Context<'a>, sure: bool) -> Option<parking_lot::MappedRwLockWriteGuard<'b, PlayerAttachment>> {
            if !contains_player_attachment(context.addr) { 
                if !sure { return None; }
                PLAYER_ATTACHMENTS.write().insert(context.addr.clone(), PlayerAttachment::default());
            }
            Some(parking_lot::RwLockWriteGuard::map(PLAYER_ATTACHMENTS.write(), |player_attachments| player_attachments.get_mut(context.addr).unwrap()))
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
        fn drop_player_attachment(player_destroy: &crate::ygopro::message::srvpru::PlayerDestroy) -> Option<PlayerAttachment> {
            let player = player_destroy.player.lock();
            PLAYER_ATTACHMENTS.write().remove(&player.client_addr)
        }

        #[doc(hidden)]
        #[allow(dead_code)]
        fn move_player_attachment(player_move: &crate::ygopro::message::srvpru::PlayerMove) {
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
            let plugin_name = std::path::Path::new(file!()).file_stem().unwrap().to_str().unwrap();
            let dropper_name = format!("{}_player_attachment_dropper", plugin_name);
            srvpru_handler!(crate::ygopro::message::srvpru::PlayerDestroy, |_, request| {
                drop_player_attachment(request);
            }).register_as(&dropper_name);
            crate::srvpru::Handler::register_handlers(plugin_name, crate::ygopro::message::Direction::SRVPRU, vec![&dropper_name]);
        }

        #[doc(hidden)]
        #[allow(dead_code)]
        fn register_player_attachment_mover() {
            let plugin_name = std::path::Path::new(file!()).file_stem().unwrap().to_str().unwrap();
            let mover_name = format!("{}_player_attachment_mover", plugin_name);
            srvpru_handler!(crate::ygopro::message::srvpru::PlayerMove, |_, request| {
                move_player_attachment(request);
            }).register_as(&mover_name);
            crate::srvpru::Handler::register_handlers(plugin_name, crate::ygopro::message::Direction::SRVPRU, vec![&mover_name])
        }
    };
}

macro_rules! player_attachment_return_type {
    () => { Option<parking_lot::MappedRwLockWriteGuard<'b, PlayerAttachment>> };
    ($type: ty) => { $type }
}

#[macro_export]
macro_rules! export_player_attach_as {
    ($name: ident$(, $type: ty, $transformer: ident)?) => {
        impl crate::srvpru::Player {
            #[doc(hidden)]
            #[allow(dead_code)]
            pub fn $name<'b>(&self) -> player_attachment_return_type!($($type)?) {
                let result = if !contains_player_attachment(&self.client_addr) { None }
                else { Some(parking_lot::RwLockWriteGuard::map(PLAYER_ATTACHMENTS.write(), |player_attachments| player_attachments.get_mut(&self.client_addr).unwrap())) };
                $(let result = $transformer(result);)?
                result
            }
        }

        impl<'a> crate::srvpru::Context<'a> {
            #[doc(hidden)]
            #[allow(dead_code)]
            pub fn $name<'b>(&self) -> player_attachment_return_type!($($type)?) {
                let result = if !contains_player_attachment(&self.addr) { None }
                else { Some(parking_lot::RwLockWriteGuard::map(PLAYER_ATTACHMENTS.write(), |player_attachments| player_attachments.get_mut(self.addr).unwrap())) };
                $(let result = $transformer(result);)?
                result
            }
        }
    };
}