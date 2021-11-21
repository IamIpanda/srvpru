use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::process::Command;
use tokio::process::Child;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::task::JoinHandle;

use parking_lot::RwLock;
use parking_lot::Mutex;

use crate::ygopro::data::LFLISTS;
use crate::ygopro::message::*;

use crate::srvpru::server;
use crate::srvpru::player::Player;
use crate::srvpru::processor::Handler;

lazy_static! {
    pub static ref ROOMS: RwLock<HashMap<String, Arc<Mutex<Room>>>> = RwLock::new(HashMap::new());
    pub static ref ROOMS_BY_CLIENT_ADDR: RwLock<HashMap<SocketAddr, Arc<Mutex<Room>>>> = RwLock::new(HashMap::new());
    pub static ref ROOMS_BY_SERVER_ADDR: RwLock<HashMap<SocketAddr, Arc<Mutex<Room>>>> = RwLock::new(HashMap::new());
}

impl crate::ygopro::Mode {
    fn to_str(&self) -> &'static str {
        match *self {
            crate::ygopro::Mode::Single => "S",
            crate::ygopro::Mode::Match => "M",
            crate::ygopro::Mode::Tag => "T",
        }
    }
}

impl HostInfo {
    fn new() -> HostInfo {
        crate::srvpru::get_configuration().ygopro.host_info.clone() 
    }

    fn decide_host_info_from_name<'a>(&mut self, origin_name: &'a str) -> &'a str {
        let (controllers, name) = 
        if let Some(index) = origin_name.find("#") {
            (&origin_name[0..index as usize], &origin_name[(index + 1)..])
        }
        else { ("", origin_name) };
        let default_host_info = &crate::srvpru::get_configuration().ygopro.host_info;
        for _controller in controllers.split(',') {
            let controller = _controller.trim();
            match controller {
                "M" | "MATCH" => { self.mode = crate::ygopro::Mode::Match },
                "T" | "TAG" => { self.mode = crate::ygopro::Mode::Tag },
                "OT" | "TCG" => { self.rule = 5 },
                "TO" | "TCGONLY" => { self.rule = 1; self.lflist = LFLISTS.first_tcg() },
                "OO" | "OCGONLY" => { self.rule = 0; self.lflist = 0 },
                "SC" | "CN" | "CCG" | "CHINESE" => { self.rule = 2; self.lflist = -1; },
                "DIY" | "CUSTOM" => { self.rule = 3 },
                "NF" | "NOLFLIST" => { self.lflist = -1 },
                "NU" | "NOUNIQUE" => { self.rule = 4 },
                "NC" | "NOCHECK" => { self.no_check_deck = true },
                "NS" | "NOSHUFFLE" => { self.no_shuffle_deck = true },
                _ if controller.starts_with("TIME") => { self.time_limit = (&controller[4..]).parse().unwrap_or(default_host_info.time_limit) },
                _ if controller.starts_with("LP") => { self.start_lp = (&controller[2..]).parse().unwrap_or(default_host_info.start_lp) },
                _ if controller.starts_with("START") => { self.start_hand = (&controller[5..]).parse().unwrap_or(default_host_info.start_hand) },
                _ if controller.starts_with("DRAW") => { self.draw_count = (&controller[4..]).parse().unwrap_or(default_host_info.draw_count) },
                _ if controller.starts_with("LFLIST") => { self.lflist = (&controller[6..]).parse().unwrap_or(default_host_info.lflist) },
                _ if controller.starts_with("MR") => { self.rule = (&controller[2..]).parse().unwrap_or(default_host_info.rule) },
                _ => ()
            }
        }
        name
    }

    pub fn to_string(&self) -> String {
        let mut name_patterns = Vec::new();
        name_patterns.push(self.mode.to_str().to_string());
        name_patterns.push(format!("TIME{}", self.time_limit));
        name_patterns.push(format!("START{}", self.start_hand));
        name_patterns.push(format!("LP{}", self.start_lp));
        name_patterns.push(format!("DRAW{}", self.draw_count));
        name_patterns.push(format!("MR{}", self.rule));
        name_patterns.push(format!("LFLIST{}", self.lflist));
        if self.no_check_deck { name_patterns.push("NC".to_string()); }
        if self.no_shuffle_deck { name_patterns.push("NS".to_string()); }
        return name_patterns.join(",");
    }

    fn generate_process_args(&self) -> [String; 12] {
        return [
            "0".to_string(),
            self.lflist.to_string(),
            self.rule.to_string(),
            (self.mode as u8).to_string(),
            self.duel_rule.to_string(),
            if self.no_check_deck {"T".to_string()} else {"F".to_string()},
            if self.no_shuffle_deck {"T".to_string()} else {"F".to_string()},
            self.start_lp.to_string(),
            self.start_hand.to_string(),
            self.draw_count.to_string(),
            self.time_limit.to_string(),
            "0".to_string()
            // Here may need more 3 random seeds
        ]
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RoomStatus {
    Starting,
    Established,
    Deleted
}

#[derive(Debug)]
pub struct Room {
    pub host_info: HostInfo,
    /// The origin password used to join room
    pub origin_name: String,
    /// The room name
    pub name: String,
    pub status: RoomStatus,
    pub server_addr: Option<SocketAddr>,
    pub server_process: Option<Child>,
    pub server_stderr_hanlder: Option<JoinHandle<()>>,
    pub players: Vec<Arc<Mutex<Player>>>,
    pub flags: HashMap<String, String>
}

impl Room {
    // ----------------------------------------------------------------------------------------------------
    /// ## spawn 
    // ---------------------------------------------------------------------------------------------------- 
    /// Try to spawn a room to actual ygopro server.
    // ----------------------------------------------------------------------------------------------------
    async fn spawn(this: &mut Arc<Mutex<Room>>) -> anyhow::Result<()> {
        let configuration = crate::srvpru::get_configuration();
        let addr = {
            let mut this = this.lock();
            let host_info = &(this.host_info);
            let mut process = 
                Command::new(configuration.ygopro.binary.clone())
                .current_dir(configuration.ygopro.cwd.clone())
                .args(&host_info.generate_process_args())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;
            let mut lines = BufReader::new(process.stdout.as_mut().ok_or(anyhow!("Spawned room don't contains stdout."))?).lines();
            let port = if let Some(line) = lines.next_line().await? {
                line.parse::<u16>().unwrap_or(0)  
            } else { 0 };
            if port == 0 { Err(anyhow!("Cannot determine port"))?; }
            let addr = (format!("{}:{}", configuration.ygopro.address, port)).parse()?;

            this.server_process = Some(process);
            this.server_addr = Some(addr);
            this.status = RoomStatus::Established;
            if configuration.ygopro.wait_start > 0 {
                // Wait some time, or player cannot join in because server not ready. (Maybe docker latency)
                tokio::time::sleep(tokio::time::Duration::from_millis(configuration.ygopro.wait_start)).await; 
            }
            addr
        };
        ROOMS_BY_SERVER_ADDR.write().insert(addr, this.clone());
        let server_stderr_hanlder = Some(Room::follow_process(this.clone())?);
        let mut room = this.lock();
        room.server_stderr_hanlder = server_stderr_hanlder;
        info!("Room {} created, target {:?}", room.name, room.server_addr);
        Ok(())
    }

    fn follow_process(room: Arc<Mutex<Room>>) -> anyhow::Result<JoinHandle<()>> {
        let stderr = room.lock()
            .server_process.as_mut().ok_or(anyhow!("Room don't have server process."))?
            .stderr.take().ok_or(anyhow!("Room don't have STDERR"))?;
        let addr = room.lock().server_addr.ok_or(anyhow!("Room don't have a server addr."))?;
        let mut lines = BufReader::new(stderr).lines();
        Ok(tokio::spawn(async move {
            while let Result::Ok(Some(line)) = lines.next_line().await {
                warn!("stderr from ygopro server {}", line)
            }
            server::trigger_internal(addr, crate::ygopro::message::srvpru::RoomDestroy { room: room.clone() }).await;
            if Arc::strong_count(&room) > 2 + room.lock().players.len() {
                let room = room.lock();
                warn!("Room {} seems still exist reference when drop. This may lead to memory leak.", room.name);
            }
        }))
    }

    pub async fn join(this: Arc<Mutex<Room>>, client_addr: &SocketAddr, client_writer: OwnedWriteHalf) -> Option<()> {
        let player = Player::new(&this, client_addr.clone(), client_writer).await.ok()?;
        let mut room = this.lock();
        info!("Player {} join room {}", player.lock().name, room.name);
        room.players.push(player);
        ROOMS_BY_CLIENT_ADDR.write().insert(client_addr.clone(), this.clone());
        Some(())
    }

    // ----------------------------------------------------------------------------------------------------
    /// ## destroy
    // ---------------------------------------------------------------------------------------------------- 
    /// Try to drop that room.
    /// 
    /// `destroy` do following things:
    /// - mark this room as deleted.
    /// - remove itself from `ROOMS`.
    /// - remove itself from room query table.
    /// - stop stderr listener.
    /// 
    /// `destroy` **WON'T** do following things:
    /// - try to drop any player inner it. (Done by server)
    /// - drop itself. (RC)
    // ----------------------------------------------------------------------------------------------------
    pub fn destroy(this: &Arc<Mutex<Room>>) {
        let mut this = this.lock();
        if this.status == RoomStatus::Deleted { return; }
        this.status = RoomStatus::Deleted;
        {
            let mut rooms = ROOMS.write();
            rooms.remove(&this.origin_name);
        }
        if let Some(addr) = this.server_addr {
            let mut rooms_by_server_addr = ROOMS_BY_SERVER_ADDR.write();
            rooms_by_server_addr.remove(&addr);
        }
        if let Some(err_handler) = &this.server_stderr_hanlder {
            err_handler.abort();
        }
    }
}

impl std::fmt::Display for Room {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{:?}]", self.name, self.status)?;
        Ok(())
    }
}


impl Room {
    pub async fn new(name: &str) -> Arc<Mutex<Room>> {
        let mut host_info = HostInfo::new();
        let origin_name = String::from(name);
        let name = host_info.decide_host_info_from_name(&origin_name).to_string();
        let mut _room = Room {
            host_info,
            origin_name,
            name,
            status: RoomStatus::Starting,
            server_addr: None,
            server_process: None,
            server_stderr_hanlder: None,
            players: Vec::new(),
            flags: HashMap::new()
        };
        let mut room = Arc::new(Mutex::new(_room));
        if let Err(e) = Room::spawn(&mut room).await {
            error!("Error on spawn room {}: {:}", room.lock().to_string(), e);
        }
        let addr = { room.lock().server_addr.clone().unwrap() };
        let room_for_message = room.clone();
        server::trigger_internal_async(addr, crate::ygopro::message::srvpru::RoomCreated { room: room_for_message }); 
        room
    }

    pub async fn get_or_create_by_name<'a>(name: &str) -> Arc<Mutex<Room>> {
        if ! ROOMS.read().contains_key(name) {
            let room = Room::new(&name).await;
            let mut rooms = ROOMS.write();
            rooms.insert(name.to_string().clone(), room);
        }
        ROOMS.read().get(name).map(|room| room.clone()).unwrap()
    }

    pub fn get_room(name: &str) -> Option<Arc<Mutex<Room>>> {
        ROOMS.read().get(name).map(|room| room.clone())
    }

    pub fn get_room_by_client_addr(client_addr: SocketAddr) -> Option<Arc<Mutex<Room>>> {
        ROOMS_BY_CLIENT_ADDR.read().get(&client_addr).map(|room| room.clone())
    }

    pub fn get_room_by_server_addr(client_addr: SocketAddr) -> Option<Arc<Mutex<Room>>> {
        ROOMS_BY_SERVER_ADDR.read().get(&client_addr).map(|room| room.clone())
    }

    pub fn register_handlers() {
        Handler::before_message::<ctos::JoinGame, _>(10, "room_producer", |context, request| Box::pin(async move { 
            let password = context.get_string(&request.pass, "pass")?;
            let room = Room::get_or_create_by_name(password).await;
            let socket = context.socket.take().ok_or(anyhow!("Socket already taken."))?;
            Room::join(room, &context.addr, socket).await;
            Ok(false)
        })).register();

        Handler::before_message::<srvpru::RoomDestroy, _>(255, "room_dropper", |_, request| Box::pin(async move {
            Room::destroy(&request.room);
            Ok(false)
        })).register();

        Handler::register_handlers("room", Direction::CTOS, vec!("room_producer"));
        Handler::register_handlers("room", Direction::SRVPRU, vec!("room_dropper"))
    }

    pub fn init() -> anyhow::Result<()> {
        Room::register_handlers();
        Ok(())
    }
}

impl Drop for Room {
    fn drop(&mut self) {
        info!("Room {} dropped.", self.to_string());
    }
}

#[macro_export]
macro_rules! room_attach {
    ($( $(#[$attr:meta])* $field:ident:$type:ty ),*) => {
        #[derive(Default, Debug)]
        #[doc(hidden)]
        pub struct RoomAttachment {
            pub $($(#[$attr])* $field: $type,)*
        }
        
        lazy_static! {
            #[doc(hidden)]
            pub static ref ROOM_ATTACHMENTS: parking_lot::RwLock<std::collections::HashMap<String, RoomAttachment>> = parking_lot::RwLock::new(std::collections::HashMap::new());
        }

        #[doc(hidden)]
        pub fn contains_room_attachment(name: &str) -> bool {
            let room_attachments = ROOM_ATTACHMENTS.read();
            room_attachments.contains_key(name)
        }

        #[doc(hidden)]
        fn _get_room_attachment<'a, 'b>(context: &crate::srvpru::Context<'a>, sure: bool) -> Option<parking_lot::MappedRwLockWriteGuard<'b, RoomAttachment>> {
            let room_mutex = context.get_room()?;
            let room = room_mutex.lock();
            let name = &room.origin_name;
            
            if !contains_room_attachment(name) { 
                if !sure { return None; }
                let mut room_attachments = ROOM_ATTACHMENTS.write();
                room_attachments.insert(name.clone(), RoomAttachment::default());
            }
            Some(parking_lot::RwLockWriteGuard::map(ROOM_ATTACHMENTS.write(), |room_attachments| room_attachments.get_mut(name).unwrap()))
        }

        /// get attached value on room for this plugin.
        #[allow(dead_code)]
        pub fn get_room_attachment<'a, 'b>(context: &crate::srvpru::Context<'a>) -> Option<parking_lot::MappedRwLockWriteGuard<'b, RoomAttachment>> {
            _get_room_attachment(context, false)
        }
        
        /// get attached value on room for this plugin.
        /// will panic if room don't exist.
        #[allow(dead_code)]
        pub fn get_room_attachment_sure<'a, 'b>(context: &crate::srvpru::Context<'a>) -> parking_lot::MappedRwLockWriteGuard<'b, RoomAttachment> {
            _get_room_attachment(context, true).unwrap()
        }

        #[doc(hidden)]
        #[allow(dead_code)]
        pub fn get_attachment_by_name<'a>(context: &mut crate::srvpru::Context<'a>, request: &crate::ygopro::message::ctos::JoinGame) -> Option<parking_lot::MappedRwLockWriteGuard<'a, RoomAttachment>> {
            let name = context.get_string(&request.pass, "pass").ok()?;
            if contains_room_attachment(&name) {
                Some(parking_lot::RwLockWriteGuard::map(ROOM_ATTACHMENTS.write(), |room_attachments| room_attachments.get_mut(name).unwrap()))
            }
            else { None }
        }

        #[doc(hidden)]
        #[allow(dead_code)]
        fn drop_room_attachment(room_destroy: &crate::ygopro::message::srvpru::RoomDestroy) -> Option<RoomAttachment> {
            let room = room_destroy.room.lock();
            let name = &room.origin_name;
            ROOM_ATTACHMENTS.write().remove(name)
        }

        #[doc(hidden)]
        #[allow(dead_code)]
        fn register_room_attachement_dropper() {
            let plugin_name = std::path::Path::new(file!()).file_stem().unwrap().to_str().unwrap();
            let dropper_name = format!("{}_room_attachment_dropper", plugin_name);
            srvpru_handler!(crate::ygopro::message::srvpru::RoomDestroy, |_, request| {
                drop_room_attachment(request);
            }).register_as(&dropper_name);
            crate::srvpru::Handler::register_handlers(plugin_name, crate::ygopro::message::Direction::SRVPRU, vec![&dropper_name]);
        }
    };
}

macro_rules! room_attachment_return_type {
    () => { Option<parking_lot::MappedRwLockWriteGuard<'b, RoomAttachment>> };
    ($type: ty) => { $type }
}

#[macro_export]
macro_rules! export_room_attach_as {
    ($name: ident$(, $type: ty, $transformer: ident)?) => {
        #[doc(hidden)]
        #[allow(dead_code)]
        impl crate::srvpru::Room {
            pub fn $name<'b>(&self) -> room_attachment_return_type!($($type)?) {
                let result = 
                    if !contains_room_attachment(&self.origin_name) { None }
                    else { Some(parking_lot::RwLockWriteGuard::map(ROOM_ATTACHMENTS.write(), |room_attachments| room_attachments.get_mut(&self.origin_name).unwrap())) };
                $(let result = $transformer(result);)?
                result
            }
        }

        #[doc(hidden)]
        #[allow(dead_code)]
        impl<'a> crate::srvpru::Context<'a> {
            pub fn $name<'b>(&self) -> room_attachment_return_type!($($type)?) {
                let result = if let Some(room) = self.get_room() {
                    let _room = room.lock();
                    let name = &_room.origin_name;
                    if !contains_room_attachment(name) { None }
                    else { Some(parking_lot::RwLockWriteGuard::map(ROOM_ATTACHMENTS.write(), |room_attachments| room_attachments.get_mut(name).unwrap())) }
                } else { None };
                $(let result = $transformer(result);)?
                result 
            }
        }
    }
}

#[macro_export]
macro_rules! export_room_attach_in_join_game_as {
    ($name: ident$(, $type: ty, $transformer: ident)?) => {
        #[doc(hidden)]
        #[allow(dead_code)]
        impl<'a> crate::srvpru::Context<'a> {
            pub fn $name<'b>(&mut self, request: &crate::ygopro::message::ctos::JoinGame) -> room_attachment_return_type!($($type)?) {
                let result = if let Some(room) = self.get_room_in_join_game(request) {
                    let _room = room.lock();
                    let name = &_room.origin_name;
                    if !contains_room_attachment(name) { None }
                    else { Some(parking_lot::RwLockWriteGuard::map(ROOM_ATTACHMENTS.write(), |room_attachments| room_attachments.get_mut(name).unwrap())) }
                } else { None };
                $(let result = $transformer(result);)?
                result  
            }
        }
    }
}