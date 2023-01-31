use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;
use httparse::{Request, EMPTY_HEADER};
use ygopro::message::client_to_server;

use super::Configuration;
use super::SrvpruConfiguration;
use super::process;
use super::process_with_instance;
use super::register_stream;
use super::remove_player_enum;
use super::message::DestroyPlayer;
use super::message::ServerStart;

pub async fn serve() {
    let addr = {
        let config = SrvpruConfiguration::get();
        format!("{}:{}", config.host, config.port).parse().expect("Cannot parse the listening socket.")
    };
    let listener = TcpListener::bind(addr).await.expect("Failed to bind the port");
    process_with_instance(addr, ServerStart).await;
    loop {
        let (socket, client_addr) = listener.accept().await.expect("");
        debug!("Income socket from {}", client_addr);
        let (mut reader, writer) = socket.into_split();
        register_stream(client_addr, writer);
        tokio::spawn(async move {
            let mut buf = [0; 10240];
            loop {
                let n = match reader.read(&mut buf).await {
                    Ok(n) if n == 0 => break,
                    Ok(n) => n,
                    Err(_) => continue
                };
                let data = &mut buf[0..n];
                if data.starts_with(b"GET")
                    || data.starts_with(b"POST")
                    || data.starts_with(b"PUT")
                    || data.starts_with(b"DELETE")
                    || data.starts_with(b"OPTION") {
                    let mut headers = [EMPTY_HEADER; 16];
                    let mut request = Request::new(&mut headers);
                    if matches!(request.parse(data), Err(_)) {
                        continue;
                    }
                    todo!();
                }
                else {
                    process::<client_to_server::MessageType>(client_addr, data).await;   
                }
            }
            {
                debug!("Break socket from {}", client_addr);
                let player_enum = remove_player_enum(client_addr);
                match player_enum {
                    super::PlayerEnum::Stream(_) => (),
                    super::PlayerEnum::Precursor(player_precursor) => {
                        if let Err(player_precursor) = Arc::try_unwrap(player_precursor) {
                            let count = Arc::strong_count(&player_precursor);
                            warn!("Player precursor {} is leak: still remain {} references.", player_precursor.lock().origin_name, count );
                        } 
                    },
                    super::PlayerEnum::Player(player) => {
                        process_with_instance(client_addr, DestroyPlayer { player: player.clone() }).await;
                        if let Err(player) = Arc::try_unwrap(player) {
                            let count = Arc::strong_count(&player);
                            warn!("Player {} is leak: still remain {} references.", player.lock().origin_name, count );
                        }
                    },
                    super::PlayerEnum::None => (),
                }
            }
        });
    }
}

#[before(ServerStart)]
fn server_start(addr: SocketAddr) {
    info!("Server starting on {:}", addr);
}
