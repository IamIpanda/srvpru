use crate::srvpru::processor::*;
use crate::srvpru::player::*;
use crate::ygopro::message::Direction;
use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;
use once_cell::sync::OnceCell;

pub static SOCKET_SERVER: OnceCell<Server> = OnceCell::new();

pub struct Server {
    pub stoc_processor: Processor,
    pub ctos_processor: Processor,
}

impl Server {
    pub fn new() -> Server {
        Server {
            stoc_processor: Processor {
                direction: Direction::STOC,
                handlers: Vec::new()
            },
            ctos_processor: Processor {
                direction: Direction::CTOS,
                handlers: Vec::new()
            }
        }
    }

    pub fn register_handlers(&mut self, ctos_handlers: &[&str], stoc_handlers: &[&str]) {
        let mut handler_library = HANDLER_LIBRARY.write();
        let mut handlers_library = HANDLER_LIBRARY_BY_PLUGIN.write();
        self.ctos_processor.handlers.clear();
        self.stoc_processor.handlers.clear(); 
        for handler_name in ctos_handlers.iter() {
            if let Some(handler) = handler_library.remove(&handler_name.to_string()) {
                self.ctos_processor.handlers.push(handler);
            }
            if let Some(mut handlers) = handlers_library.remove(&handler_name.to_string()) {
                self.ctos_processor.handlers.append(&mut handlers);
            }
        }
        for handler_name in stoc_handlers.iter() {
            if let Some(handler) = handler_library.remove(&handler_name.to_string()) {
                self.stoc_processor.handlers.push(handler);
            }
            if let Some(mut handlers) = handlers_library.remove(&handler_name.to_string()) {
                self.stoc_processor.handlers.append(&mut handlers);
            }
        }
        self.stoc_processor.sort();
        self.ctos_processor.sort();
    }

    pub async fn start(&'static self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("0.0.0.0:7933").await?;
        info!("Socket server started.");
        loop {
            let (socket, addr) = listener.accept().await?;
            let (mut reader, writer) = socket.into_split();
            let mut writer = Some(writer);
            let server = SOCKET_SERVER.get().expect("socket server not propered initialized");
            tokio::spawn(async move {
                let mut buf = [0; 10240];
                loop {
                    let n = match reader.read(&mut buf).await {
                        Ok(n) if n == 0 => break,
                        Ok(n) => n,
                        Err(_e) => break
                    };
                    let exist = { PLAYERS.read().contains_key(&addr) };
                    if exist {
                        let players = PLAYERS.read();
                        let player = players.get(&addr).unwrap();
                        let socket = &mut player.lock().server_stream_writer;
                        server.ctos_processor.process_multiple(socket, &addr, &buf[0..n]).await;
                    }
                    else {
                        server.ctos_processor.process_multiple(&mut writer, &addr, &buf[0..n]).await;
                    }
                };
                {
                    let mut players = PLAYERS.write();
                    if let Some(player) = players.remove(&addr) {
                        Player::destroy(player);
                    }
                }
            });
        }
    }
}