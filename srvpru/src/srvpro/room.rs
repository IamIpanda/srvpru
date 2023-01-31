use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Weak;

use srvpru_proc_macros::before;
use tokio::io::AsyncBufReadExt;
use tokio::net::TcpStream;
use tokio::process::Child;
use tokio::process::Command;
use tokio::task::JoinHandle;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use parking_lot::RwLock;
use anyhow::Result;

use ygopro::message::HostInfo;
use ygopro::message::Message;
use ygopro::message::client_to_server::JoinGame;

use super::Configuration;
use super::Player;
use super::PlayerPrecusor;
use super::SimplyContinue;
use super::FromRequest;
use super::YgoproConfiguration;

use crate::srvpro::message::DestroyRoom;
use crate::srvpro::process_with_instance;

pub static ROOMS:                Lazy<RwLock<HashMap<String,      Arc<Mutex<Room>>>>> = Lazy::new(|| RwLock::new(HashMap::new()));
pub static ROOMS_BY_CLIENT_ADDR: Lazy<RwLock<HashMap<SocketAddr, Weak<Mutex<Room>>>>> = Lazy::new(|| RwLock::new(HashMap::new()));
pub static ROOMS_BY_SERVER_ADDR: Lazy<RwLock<HashMap<SocketAddr, Weak<Mutex<Room>>>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Debug)]
pub struct Room<P: YgoproProvider = YgoproFromShell> {
    pub host_info: HostInfo,
    /// The origin password used to join room
    pub origin_name: String,
    /// The room name
    pub name: String,
    /// Ygopro server address binded to this room.
    pub server: Option<P>,
    /// Players in the room
    pub players: Vec<Arc<Mutex<Player>>>,
    /// Additional meta message other plugins add to room.
    pub flags: HashMap<String, String>
}

impl<P: YgoproProvider> Room<P> {
    fn new(name: &str) -> Arc<Mutex<Self>> {
        let room = name.into();
        return Arc::new(Mutex::new(room));
    }

    async fn spawn(this: Arc<Mutex<Room<P>>>) -> Result<()> {
        let room = this.clone();
        let mut this = this.lock();
        this.server = Some(P::spawn(&this.host_info, room).await?);
        Ok(())
    }

    async fn new_spawn(name: &str) -> Result<Arc<Mutex<Self>>> {
        let room = Arc::new(Mutex::new(name.into()));
        Room::spawn(room.clone()).await?;
        return Ok(room);
    }
}

impl Room {
    fn get_by_name(name: &str) -> Option<Arc<Mutex<Room>>> {
        ROOMS.read().get(name).map(|f| f.clone())
    }

    async fn get_or_create(name: &str) -> Result<Arc<Mutex<Room>>> {
        if let Some(room) = Room::get_by_name(name) {
            return Ok(room)
        };
        let room = Self::new_spawn(name).await?;
        ROOMS.write().insert(name.to_string(), room.clone());
        Ok(room)
    }

    pub fn get_by_client_addr(addr: &SocketAddr) -> Option<Arc<Mutex<Room>>> {
        ROOMS_BY_CLIENT_ADDR.read().get(&addr).map(|f| f.clone().upgrade()).flatten()
    }

    fn get_by_server_addr(addr: &SocketAddr) -> Option<Arc<Mutex<Room>>> {
        ROOMS_BY_SERVER_ADDR.read().get(&addr).map(|f| f.clone().upgrade()).flatten()
    }

    async fn join(this: Arc<Mutex<Self>>, player: PlayerPrecusor) -> Result<()> {
        let mut room = this.lock();
        let server = match room.server.as_mut() {
            Some(server) => server,
            None => return Err(anyhow!("Player {} tried join to room {} which is not prepared.", player.name, room.name))
        };
        let stream = server.get_socket().await?;
        let player = player.upgrade(this.clone(), stream); 
        info!("Player {} joined room {}", player.lock().name, room.name);
        ROOMS_BY_CLIENT_ADDR.write().insert(player.lock().client_addr, Arc::downgrade(&this));
        room.players.push(player);
        Ok(())
    }
}

impl<S> FromRequest<S> for Arc<Mutex<Room>> where S: Message {
    type Rejection = SimplyContinue;

    fn from_request(request: &mut super::Bundle<S>) -> Result<Self, Self::Rejection> {
        match &request.1.room {
            Some(room) => Ok(room.clone()),
            None => {
                let room_hash = match S::message_type() {
                    ygopro::message::MessageType::STOC(_) => &ROOMS_BY_CLIENT_ADDR,
                    ygopro::message::MessageType::CTOS(_) => &ROOMS_BY_CLIENT_ADDR,
                    ygopro::message::MessageType::GM(_) => &ROOMS_BY_CLIENT_ADDR,
                    ygopro::message::MessageType::Other(_, _) => &ROOMS_BY_CLIENT_ADDR,
                };
                room_hash
                .read()
                .get(&request.0.socket_addr)
                .map(|f| f.clone().upgrade())
                .flatten()
                .ok_or(SimplyContinue)
            }
        }
    }
}

impl Deref for Room {
    type Target = HostInfo;
    fn deref(&self) -> &Self::Target {
        &self.host_info
    }
}

impl<P: YgoproProvider> From<&str> for Room<P> {
    fn from(value: &str) -> Self {
        let (controllers, name) = value.split_once('#').unwrap_or(("", value));
        let mut host_info = ygopro::message::HostInfo::default();
        for controller in controllers.split(',') {
            let controller = controller.trim();
            match controller {
                "M" | "MATCH" => { host_info.mode = ygopro::constants::Mode::Match },
                "T" | "TAG" => { host_info.mode = ygopro::constants::Mode::Tag },
                "OT" | "TCG" => { host_info.rule = 5 },
                "TO" | "TCGONLY" => { host_info.rule = 1; host_info.lflist = ygopro::data::LFList::first_tcg() },
                "OO" | "OCGONLY" => { host_info.rule = 0; host_info.lflist = 0 },
                "SC" | "CN" | "CCG" | "CHINESE" => { host_info.rule = 2; host_info.lflist = -1; },
                "DIY" | "CUSTOM" => { host_info.rule = 3 },
                "NF" | "NOLFLIST" => { host_info.lflist = -1 },
                "NU" | "NOUNIQUE" => { host_info.rule = 4 },
                "NC" | "NOCHECK" => { host_info.no_check_deck = true },
                "NS" | "NOSHUFFLE" => { host_info.no_shuffle_deck = true },
                _ if controller.starts_with("TIME") =>   { controller[4..].parse().ok().map(|v| host_info.time_limit = v); },
                _ if controller.starts_with("LP") =>     { controller[2..].parse().ok().map(|v| host_info.start_lp = v); },
                _ if controller.starts_with("START") =>  { controller[5..].parse().ok().map(|v| host_info.start_hand = v); },
                _ if controller.starts_with("DRAW") =>   { controller[4..].parse().ok().map(|v| host_info.draw_count = v); },
                _ if controller.starts_with("LFLIST") => { controller[6..].parse().ok().map(|v| host_info.lflist = v); },
                _ if controller.starts_with("MR") =>     { controller[2..].parse().ok().map(|v| host_info.rule = v); },
                _ => ()
            };
        }
        Room { 
            host_info, 
            origin_name: name.to_string(),
            name: name.to_string(),
            server: None,
            players: Vec::new(), 
            flags: HashMap::new()
        }
    }
}

impl ToString for Room {
    fn to_string(&self) -> String {
        let mut name_patterns = Vec::new();
        name_patterns.push(match self.mode {
            ygopro::constants::Mode::Single => "S",
            ygopro::constants::Mode::Match  => "M",
            ygopro::constants::Mode::Tag    => "T",
        }.to_string());
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
}

#[async_trait]
pub trait YgoproProvider: std::fmt::Display + Sized {
    type Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin;

    async fn spawn(host_info: &HostInfo, owner: Arc<Mutex<Room<Self>>>) -> Result<Self>;
    async fn get_socket(&self) -> Result<Self::Stream>;
}

#[derive(Debug)]
pub struct YgoproFromShell {
    process: Child,
    server_addr: SocketAddr,
    error_handler: JoinHandle<()>
}

impl YgoproFromShell {
    fn generate_process_args(host_info: &HostInfo) -> [String; 12] {
        return [
            "0".to_string(),
            host_info.lflist.to_string(),
            host_info.rule.to_string(),
            (host_info.mode as u8).to_string(),
            host_info.duel_rule.to_string(),
            if host_info.no_check_deck {"T".to_string()} else {"F".to_string()},
            if host_info.no_shuffle_deck {"T".to_string()} else {"F".to_string()},
            host_info.start_lp.to_string(),
            host_info.start_hand.to_string(),
            host_info.draw_count.to_string(),
            host_info.time_limit.to_string(),
            "0".to_string()
            // Here may need more 3 random seeds
        ]
    }
}

#[async_trait]
impl YgoproProvider for YgoproFromShell {
    type Stream = TcpStream;

    async fn spawn(host_info: &HostInfo, owner: Arc<Mutex<Room<Self>>>) -> Result<Self> {
        let config = YgoproConfiguration::get();
        let mut process = Command::new(config.binary.clone())
            .current_dir(config.cwd.clone())
            .args(YgoproFromShell::generate_process_args(host_info))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdout_lines = tokio::io::BufReader::new(process.stdout.as_mut().ok_or(anyhow!("STDOUT don't exist"))?).lines();
        let mut stderr_lines = tokio::io::BufReader::new(process.stderr.take().ok_or(anyhow!("STDERR don't exist"))?).lines();
        let port = match stdout_lines.next_line().await {
            Ok(Some(line)) => line.parse()?,
            _ => 0u16
        };
        if port == 0 {
            return Err(anyhow!("Can't determine room port."))
        }
        let server_addr = format!("{}:{}", config.address, port).parse()?;
        ROOMS_BY_SERVER_ADDR.write().insert(server_addr, Arc::downgrade(&owner));
        debug!("Spawn room listening on {}", server_addr);
        tokio::time::sleep(tokio::time::Duration::from_millis(config.wait_start)).await;
        Ok(Self {
            process, 
            server_addr,
            error_handler: tokio::spawn(async move {
                while let Ok(Some(line)) = stderr_lines.next_line().await {
                    warn!("[{}] STDERR - {}",  owner.lock().name ,line);
                };
                process_with_instance("0.0.0.0:1".parse().unwrap(), DestroyRoom { room: owner.clone() }).await;
                if let Err(room) = Arc::try_unwrap(owner) {
                    let count = Arc::strong_count(&room);
                    warn!("Room {} is leak: still remain {} references.", room.lock().origin_name, count );
                }
            }) 
        })
    }
    
    async fn get_socket(&self) -> Result<TcpStream> {
        Ok(TcpStream::connect(self.server_addr).await?)
    } 
}

impl std::fmt::Display for YgoproFromShell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:}", self.server_addr)
    }
}

#[before]
async fn join_game(message: &JoinGame, player: PlayerPrecusor) -> Result<()> {
    let room = Room::get_or_create(&message.pass).await?;
    Room::join(room, player).await?;
    Ok(())
}

#[before]
fn drop_room(message: &DestroyRoom) {
    let room = message.room.lock();
    ROOMS.write().remove(&room.name);
    let mut rooms_by_client_addr = ROOMS_BY_CLIENT_ADDR.write();
    for player in &room.players {
        let client_addr = player.lock().client_addr;
        rooms_by_client_addr.remove(&client_addr);
    }
}

#[tokio::test]
async fn spawn_room() {
    let stream = TcpStream::connect("192.168.3.12:22").await.unwrap();
    let (_, writer) = stream.into_split();
    let player = PlayerPrecusor {
        name: "test".to_string(),
        origin_name: "test".to_string(),
        client_addr: "127.0.0.1:8080".parse().unwrap(),
        client_stream_writer: writer,
        data_cache: vec![]
    };
    let j = JoinGame { version: 1, align: 0, gameid: 0, pass: "M#a".into() };
    join_game(&j, player).await.ok();
}

#[tokio::test]
async fn spawn_room2() {
    use ygopro::message::MessageEnum;
    use ygopro::serde::LengthWrapper;
    use ygopro::message::client_to_server;
    use ygopro::message::client_to_server::PlayerInfo;
    use ygopro::message::client_to_server::JoinGame;
    use ygopro::serde::ser::serialize;
    use ygopro::serde::de::deserialize;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
     
    let room = Room::<YgoproFromShell>::new_spawn("test").await.unwrap();
    println!("Start room on {:?}", room.lock().server.as_ref().unwrap().server_addr);
    let stream = room.lock().server.as_ref().unwrap().get_socket().await.unwrap(); 
     
    let (mut r, mut w) = stream.into_split();
    let init = vec!(
        LengthWrapper(MessageEnum::CTOS(client_to_server::MessageEnum::PlayerInfo(PlayerInfo { name: "Player".into() }))),
        LengthWrapper(MessageEnum::CTOS(client_to_server::MessageEnum::JoinGame(JoinGame { version: 1, align: 0, gameid: 0, pass: "a".into() } )))
    );
    let init = serialize(&init).unwrap();
    let init = vec![0x29, 0x00, 0x10, 0x50, 0x00, 0x6c, 0x00, 0x61, 0x00, 0x79, 0x00, 0x65, 0x00, 0x72, 0x00, 0x00, 0x00, 0x56, 0x74, 0x72, 0xe6, 0x4d, 0xc0, 0xfe, 0xff, 0xff, 0xff, 0xf0, 0xfb, 0x57, 0x01, 0x1e, 0x5f, 0x2f, 0x77, 0x01, 0x00, 0x00, 0x00, 0x50, 0xa3, 0x58, 0x0d, 0x31, 0x00, 0x12, 0x54, 0x13, 0x6c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x72, 0x00, 0x00, 0x00, 0x56, 0x74, 0x72, 0xe6, 0x4d, 0xc0, 0xfe, 0xff, 0xff, 0xff, 0xf0, 0xfb, 0x57, 0x01, 0x1e, 0x5f, 0x2f, 0x77, 0x01, 0x00, 0x00, 0x00, 0x50, 0xa3, 0x58, 0x0d, 0x80, 0x97, 0x58, 0x0d, 0x10, 0xa1, 0x58, 0x0d];
    let start : Vec<LengthWrapper<client_to_server::MessageEnum>> = deserialize(&init).unwrap();
    println!("Write {} bytes: {:?}", init.len(), init);
    tokio::spawn(async move {
        let mut buf = [0; 10240];
        loop {
            let n = r.read(&mut buf).await.unwrap();
            println!("{:?}", &buf[0..n]);
            if n == 0 { break; }
        }
        println!("Reader finish");
    });
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    w.write_all(&init).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1800)).await;
}
