// ============================================================
// room_list
// ------------------------------------------------------------
//! Start a websocket server, broadcast room list to listeners.
// ============================================================

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ConnectInfo;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::routing;

use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use futures_util::StreamExt;

use parking_lot::Mutex;
use serde::Serialize;
use serde_json::Value;

use crate::srvpru::CommonError;
use crate::ygopro::Netplayer;
use crate::ygopro::message::Direction;
use crate::ygopro::message::HostInfo;
use crate::ygopro::message::ctos::JoinGame;

use crate::srvpru::Handler;
use crate::srvpru::Room;
use crate::srvpru::message::RoomDestroy;
use crate::srvpru::message::RoomCreated;
use crate::srvpru::plugins::plugin_enabled;
use crate::srvpru::plugins::base::api::register_api;

set_configuration! {
    #[serde(default="default_port")]
    port: u32
}

fn default_port() -> u32 { 7922 }

depend_on! {
    "api"
}

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_dependency()?;
    register_handlers();
    register_apis();
    Ok(())
}

fn register_handlers() {
    Handler::before_message::<RoomCreated, _>(255, "room_list_create_listener", |context, _| Box::pin(async move {
        broadcast_room("create".to_string(), context.get_room().ok_or(CommonError::RoomNotExist)?.clone());
        Ok(false)
    })).register();
    Handler::before_message::<RoomDestroy, _>(95, "room_list_destroy_listener", |context, _| Box::pin(async move {
        broadcast_room("delete".to_string(), context.get_room().ok_or(CommonError::RoomNotExist)?.clone());
        Ok(false)
    })).register();
    Handler::before_message::<JoinGame, _>(255, "room_list_update_listener", |context, _| Box::pin(async move {
        broadcast_room("update".to_string(), context.get_room().ok_or(CommonError::RoomNotExist)?.clone());
        Ok(false)
    })).register();

    Handler::register_handlers("room_list", Direction::SRVPRU, vec!["room_list_create_listener", "room_list_destroy_listener"]);
    Handler::register_handlers("room_list_update_listener", Direction::CTOS, vec!["room_list_update_listener"]);
}

#[derive(Serialize)]
struct RoomData {
    id: String,
    title: String,
    user: RoomDataUser,
    users: Vec<RoomDataUsers>,
    options: HostInfo,
    arena: String
}

#[derive(Serialize)]
struct RoomDataUser { username: String }
#[derive(Serialize)]
struct RoomDataUsers { 
    username: String,
    position: Netplayer    
}

#[derive(Serialize)]
struct WebSocketMessage {
    event: String,
    data: RoomData
}

impl Room {
    fn generate_room_list_data(&self) -> RoomData {
        RoomData { 
            id: self.name.clone(), 
            title: self.name.clone(), 
            user: RoomDataUser { username: self.name.clone() }, 
            users: Vec::new(),
            options: self.host_info.clone(), 
            arena: self.flags.get("arena").map(|arena| arena.clone()).unwrap_or_default()
        }
    }
}

lazy_static! {
    pub static ref WEBSOCKETS: Mutex<HashMap<SocketAddr, SplitSink<WebSocket, Message>>> = Mutex::new(HashMap::new());
}

fn register_apis() {
    if ! plugin_enabled("room_list") { return; }
    register_api(|router| router.route("/roomlist", routing::get(server_main_handler)));
}

async fn server_main_handler(ws: WebSocketUpgrade, ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    let socket_addr = addr;
    ws.on_upgrade(move |mut socket| async move {
        send_all_rooms(&mut socket).await;
        let (writer, mut reader) = socket.split();
        WEBSOCKETS.lock().insert(socket_addr, writer);
        loop {
            match reader.next().await {
                Some(message) => { debug!("Received a message from {:} websocket: {:?}", socket_addr, message) },
                None => break,
            }
        }
        WEBSOCKETS.lock().remove(&socket_addr);
    })
}

async fn send_all_rooms(socket: &mut WebSocket) {
    let rooms = crate::srvpru::ROOMS.read();
    let rooms_value: Vec<Value> = rooms.iter()
        .filter(|(_, room)| ! room.lock().flags.contains_key("private"))
        .map(|(_, room)| serde_json::to_value(room.lock().generate_room_list_data()).unwrap_or_default())
        .collect();
    let value = Value::Object(vec![
        ("event".to_string(), Value::String("init".to_string())),
        ("data".to_string(),  Value::Array(rooms_value))
    ].into_iter().collect());
    socket.send(Message::Text(serde_json::to_string(&value).unwrap_or_default())).await.ok();
}

fn broadcast_room(event: String, room: Arc<Mutex<Room>>) {
    if room.lock().flags.contains_key("private") { return; }
    tokio::spawn(async move {
        broadcast_message(serde_json::to_string(&WebSocketMessage { event, data: room.lock().generate_room_list_data() }).unwrap_or_default()).await;
    });
}

async fn broadcast_message(message: String) {
    let mut websockets = WEBSOCKETS.lock();
    for (_, websocket) in websockets.iter_mut() {
        websocket.send(Message::Text(message.clone())).await.ok(); // I don't care if websocket message is correctly sent.
    }
}