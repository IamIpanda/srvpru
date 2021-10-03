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
use parking_lot::RwLockReadGuard;
use parking_lot::MappedRwLockReadGuard;

use crate::ygopro::message::*;
use crate::ygopro::constants;
use crate::srvpru::processor::Handler;
use crate::srvpru::player::Player;
use crate::srvpru::player::PLAYERS;

lazy_static! {
    pub static ref ROOMS: RwLock<HashMap<String, Arc<Mutex<Room>>>> = RwLock::new(HashMap::new());
    pub static ref ROOMS_BY_CLIENT_ADDR: RwLock<HashMap<SocketAddr, Arc<Mutex<Room>>>> = RwLock::new(HashMap::new());
    pub static ref ROOMS_BY_SERVER_ADDR: RwLock<HashMap<SocketAddr, Arc<Mutex<Room>>>> = RwLock::new(HashMap::new());
}

impl HostInfo {
    fn new() -> HostInfo {
        HostInfo {
            lflist: 0,
            rule: 0,
            mode: 0,
            duel_rule: 5,
            no_check_deck: false,
            no_shuffle_deck: false,
            start_lp: 8000,
            start_hand: 5,
            draw_count: 1,
            time_limit: 233
        }
    }

    fn decide_host_info_from_name<'a>(&mut self, origin_name: &'a str) -> &'a str {
        let (controllers, name) = 
        if let Some(index) = origin_name.find("#") {
            (&origin_name[0..index as usize], &origin_name[(index + 1)..])
        }
        else { ("", origin_name) };
        for _controller in controllers.split(',') {
            let controller = _controller.trim();
            match controller {
                "M" | "MATCH" => { self.mode = constants::Mode::Match as u8 },
                "T" | "TAG" => { self.mode = constants::Mode::Tag as u8 },
                "OT" | "TCG" => { self.rule = 5 },
                "TO" | "TCGONLY" => { self.rule = 1 },
                "OO" | "OCGONLY" => { self.rule = 0 },
                "SC" | "CN" | "CCG" | "CHINESE" => { self.rule = 2 },
                "DIY" | "CUSTOM" => { self.rule = 3 },
                "NF" | "NOLFLIST" => { self.lflist = -1 },
                "NU" | "NOUNIQUE" => { self.rule = 4 },
                "NC" | "NOCHECK" => { self.no_check_deck = true },
                "NS" | "NOSHUFFLE" => { self.no_shuffle_deck = true },
                _ if controller.starts_with("TIME") => { self.time_limit = (&controller[4..]).parse::<u8>().unwrap_or(180) },
                _ if controller.starts_with("LP") => { self.start_lp = (&controller[2..]).parse::<u32>().unwrap_or(8000) },
                _ if controller.starts_with("START") => { self.start_hand = (&controller[5..]).parse::<u8>().unwrap_or(5) },
                _ if controller.starts_with("DRAW") => { self.draw_count = (&controller[4..]).parse::<u8>().unwrap_or(1) },
                _ if controller.starts_with("LFLIST") => { self.lflist = (&controller[6..]).parse::<i32>().unwrap_or(-1) },
                _ if controller.starts_with("MR") => { self.rule = (&controller[2..]).parse::<u8>().unwrap_or(5) },
                _ => ()
            }
        }
        name
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum RoomStatus {
    Starting,
    Established,
    Deleted
}

pub struct Room {
    pub host_info: HostInfo,
    pub origin_name: String,
    pub name: String,
    pub status: RoomStatus,
    pub server_addr: Option<SocketAddr>,
    pub server_process: Option<Child>,
    pub server_stderr_hanlder: Option<JoinHandle<()>>,
    pub players: Vec<Arc<Mutex<Player>>>,
}

impl Room {
    async fn spawn(this: &mut Arc<Mutex<Room>>, first_seed: i32) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut this = this.lock();
            let host_info = &(this.host_info);
            let mut process = Command::new("/Users/iami/Programming/mycard/srvpru/ygopro").args(&[
                "0",
                &(host_info.lflist.to_string()),
                &(host_info.rule.to_string()),
                &(host_info.mode.to_string()),
                &(host_info.duel_rule.to_string()),
                if host_info.no_check_deck {"T"} else {"F"},
                if host_info.no_shuffle_deck {"T"} else {"F"},
                &(host_info.start_lp.to_string()),
                &(host_info.start_hand.to_string()),
                &(host_info.draw_count.to_string()),
                &(host_info.time_limit.to_string()),
                &(first_seed.to_string()),
                // Here need 3 seeds
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
            let mut lines = BufReader::new(process.stdout.as_mut().unwrap()).lines();
            let port = if let Some(line) = lines.next_line().await? {
                line.parse::<u16>().unwrap_or(0)  
            } else { 0 };
            if port == 0 { return Err("Cannot determine port")?; }
            let addr = (format!("127.0.0.1:{}", port)).parse().unwrap();

            this.server_process = Some(process);
            this.server_addr = Some(addr);
            this.status = RoomStatus::Established;
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await; // Wait 10 ms, or player cannot join in because server not ready.
        }
        let server_stderr_hanlder = Some(Room::follow_process(this));
        let mut this = this.lock();
        this.server_stderr_hanlder = server_stderr_hanlder;
        info!("Room {} created, target {:?}", this.name, this.server_addr.as_ref().unwrap());
        Ok(())
    }

    fn follow_process(this: &mut Arc<Mutex<Room>>) -> JoinHandle<()> {
        let this = this.clone();
        let stderr = { this.lock().server_process.as_mut().unwrap().stderr.take().unwrap() };
        let mut lines = BufReader::new(stderr).lines();
        tokio::spawn(async move {
            while let Result::Ok(Some(line)) = lines.next_line().await {
                warn!("stderr from ygopro server {}", line)
            }
            Room::destroy(this);
        })
    }

    async fn join(this: Arc<Mutex<Room>>, client_addr: &SocketAddr, client_writer: OwnedWriteHalf) -> Option<()> {
        let player = Player::new(&this, client_addr.clone(), client_writer).await?;
        let mut room = this.lock();
        info!("Player {} join room {}", player.lock().name, room.name);
        room.players.push(player);
        ROOMS_BY_CLIENT_ADDR.write().insert(client_addr.clone(), this.clone());
        Some(())
    }

    pub fn destroy(this: Arc<Mutex<Room>>) {
        let mut this = this.lock();
        if this.status == RoomStatus::Deleted { return; }
        this.status = RoomStatus::Deleted;
        //if this.players.len() != 0 { 
        //    warn!("Room {} is going to be destroyed with player still alive.", this.to_string());
        //    while let Some(player) = this.players.pop() { Player::destroy(player); }
        //}
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

    pub fn to_string(&self) -> String {
        format!("{}", self.name)
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
        };
        let mut room = Arc::new(Mutex::new(_room));
        Room::spawn(&mut room, 0).await;
        room
    }

    pub async fn find_or_create_by_name<'a>(name: &str) -> MappedRwLockReadGuard<'a, Arc<Mutex<Room>>> {
        let name = name.to_string();
        let contains: bool = {
            let rooms = ROOMS.read();
            rooms.contains_key(&name)
        };
        if !contains {
            let room = Room::new(&name).await;
            let mut rooms = ROOMS.write();
            rooms.insert(name.clone(), room);
        }
        RwLockReadGuard::map(ROOMS.read(), move |rooms| rooms.get(&name).unwrap())
    }

    pub fn register_processors() {
        // Core switch handler
        let switch_handler = Handler::new(2, |_| true, |context| Box::pin(async move {
            let players = PLAYERS.read();
            if ! players.contains_key(&context.addr) {
                Player::buffer_data_for_precursor(context.addr, context.request_buffer);
            }
            false
        }));
        
        let join_game_handler = Handler::follow_message::<crate::ygopro::message::CTOSJoinGame, _>(10, MessageType::CTOS(CTOSMessageType::JoinGame), |context, request| Box::pin(async move { 
            let password = cast_to_string(&request.pass);
            if password.is_none() { context.response = None; return true; }
            let password = password.unwrap();
            let room = Room::find_or_create_by_name(&password).await;
            let socket = context.socket.take().unwrap();
            Room::join(room.clone(), &context.addr, socket).await;
            return false;
        }));

        Handler::register_handlers("room", vec!(switch_handler, join_game_handler));
    }
}

impl Drop for Room {
    fn drop(&mut self) {
        info!("Room {} dropped.", self.to_string());
    }
}