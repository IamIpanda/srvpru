#![allow(dead_code)]
#[macro_use] extern crate serde_big_array;
#[macro_use] extern crate downcast_rs;
#[macro_use] extern crate erased_serde;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate pretty_env_logger;

mod ygopro;
mod srvpru;

use crate::srvpru::room::Room;
use crate::srvpru::player::Player;
use crate::srvpru::server::Server;
use crate::srvpru::plugins;
use crate::srvpru::server::SOCKET_SERVER;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    Player::register_handlers();
    Room::register_processors();
    plugins::debugger::Debugger::register_handlers();
    let mut server = Server::new();
    server.register_handlers(&["room", "player", "ctos_debugger"], &["stoc_debugger"]);
    SOCKET_SERVER.set(server).ok().expect("Failed to set socket server");
    SOCKET_SERVER.get().unwrap().start().await.expect("Failed to start socket server");
    error!("Terminated server. Srvpru is going to down.")
}

/*
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    loop {
        let (mut socket, addr) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];
            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                if let Err(e) = socket.write_all(&buf[1..n]).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}

*/