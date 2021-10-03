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
use crate::srvpru::processor::Handler;
use crate::srvpru::room::Room;
use crate::srvpru::server::SOCKET_SERVER;
use crate::srvpru::room::ROOMS_BY_CLIENT_ADDR;

lazy_static! {
    pub static ref PLAYERS: RwLock<HashMap<SocketAddr, Arc<Mutex<Player>>>> = RwLock::new(HashMap::new());
    pub static ref PLAYER_PRECURSORS: RwLock<HashMap<SocketAddr, PlayerPrecursor>> = RwLock::new(HashMap::new());
}

pub struct PlayerPrecursor {
    name: String,
    data_cache: Vec<Vec<u8>>  
}

impl PlayerPrecursor {
    fn new(name: String, client_addr: &SocketAddr, data: &[u8]) {
        let mut precursor = PlayerPrecursor {
            name: name,
            data_cache: Vec::new()
        };
        precursor.data_cache.push(data.to_vec());
        let mut precursors = PLAYER_PRECURSORS.write();
        precursors.insert(client_addr.clone(), precursor);
    }
}

pub struct Player {
    pub room: Arc<Mutex<Room>>,
    pub name: String,
    pub client_addr: SocketAddr,
    pub client_stream_writer: Option<OwnedWriteHalf>,
    pub server_stream_writer: Option<OwnedWriteHalf>,
    pub reader_handler: JoinHandle<()>,
    pub saved_data: Vec<Vec<u8>>
}

impl Player {
    pub async fn new(room: &Arc<Mutex<Room>>, client_addr: SocketAddr, client_stream_writer: OwnedWriteHalf) -> Option<Arc<Mutex<Player>>> {
        let room = room.clone();
        let server_addr = room.lock().server_addr.clone().unwrap();
        let precursor = PLAYER_PRECURSORS.write().remove(&client_addr)?;
        let stream_resuilt = TcpStream::connect(server_addr).await;
        let stream = match stream_resuilt {
            Ok(stream) => stream,
            Err(err) => {
                println!("{:?}", err);
                return None;
            }
        };
        let (server_stream_reader, server_stream_writer) = stream.into_split();
        let _player = Player {
            room: room,
            name: precursor.name,
            client_addr,
            client_stream_writer: Some(client_stream_writer),
            server_stream_writer: Some(server_stream_writer),
            reader_handler: tokio::spawn(async {}),
            saved_data: Vec::new()
        };
        let player = Arc::new(Mutex::new(_player));
        {
            let mut _player = player.lock();
            _player.reader_handler = Player::follow_socket(&player, &client_addr, server_stream_reader); 
        }
        {
            let mut _player = player.lock();
            for data in precursor.data_cache.iter() {
                _player.server_stream_writer.as_mut().unwrap().write_all(&data).await.ok()?
            }
        }
        PLAYERS.write().insert(client_addr.clone(), player.clone());
        Some(player)
    }

    fn follow_socket(this: &Arc<Mutex<Player>>, client_addr: &SocketAddr, mut server_stream_reader: OwnedReadHalf) -> JoinHandle<()> {
        let this = this.clone();
        let addr = client_addr.clone();
        tokio::spawn(async move {
            let mut buf = [0; 10240];
            loop {
                let n = match server_stream_reader.read(&mut buf).await {
                    Ok(n) if n == 0 => break,
                    Ok(n) => n,
                    Err(_e) => break
                };
                let mut player = this.lock();
                SOCKET_SERVER.get().unwrap().stoc_processor.process_multiple(&mut player.client_stream_writer, &addr, &buf[0..n]).await;
            }
        })
    }

    pub fn register_handlers() {
        let player_info_handler = Handler::follow_message::<message::CTOSPlayerInfo, _>(1, message::MessageType::CTOS(message::CTOSMessageType::PlayerInfo), |context, request| Box::pin(async move {
            let name = message::cast_to_string(&request.name).unwrap_or_default();
            PlayerPrecursor::new(name, &context.addr, &context.request_buffer);
            return true;
        }));

        Handler::register_handlers("player", vec!(player_info_handler));
    }

    pub fn buffer_data_for_precursor(client_addr: &SocketAddr, data: &[u8]) -> bool {
        let mut precursors = PLAYER_PRECURSORS.write();
        let precursor = precursors.get_mut(client_addr);
        if let Some(precursor) = precursor {
            precursor.data_cache.push(data.to_vec());   
            true
        }
        else { false }
    }

    pub fn destroy(this: Arc<Mutex<Player>>) {
        let _self = this.lock();
        _self.reader_handler.abort();
        {
            let mut room = _self.room.lock();
            let index = room.players.iter().position(|player| Arc::ptr_eq(player, &this)).unwrap();
            room.players.remove(index);
        }
        {
            let mut query_table = ROOMS_BY_CLIENT_ADDR.write();
            query_table.remove(&_self.client_addr);
        }
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