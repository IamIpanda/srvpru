use std::convert::Infallible;
use std::sync::Arc;

use parking_lot::Mutex;

use srvpru_proc_macros::Serialize;
use srvpru_proc_macros::Deserialize;
use ygopro::message::MessageType;

use super::FromRequest;
use super::Player;
use super::Room;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 0)]
pub struct ServerStart;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 1)]
pub struct Reload;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 10)]
pub struct RoomCreated {
    pub room: Arc<Mutex<Room>>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 21)]
pub struct DestroyPlayer {
    pub player: Arc<Mutex<Player>>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 31)]
pub struct DestroyRoom {
    pub room: Arc<Mutex<Room>>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 22)]
pub struct MovePlayer {
    pub post_player: Arc<Mutex<Player>>,
    pub new_player: Arc<Mutex<Player>>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 101)]
pub struct STOCProcessError {
    pub error: Infallible
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 102)]
pub struct CTOSProcessError {
    pub error: Infallible
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 104)]
pub struct SRVPRUProcessError {
    pub error: Infallible
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 105)]
pub struct StocListenError {
    pub error: Infallible 
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 106)]
pub struct CtosListenError {
    pub error: Infallible
}
#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 201)]
pub struct LPChange {
    pub player: Arc<Mutex<Player>>,
    pub lp: i32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(srvpru, mod_name = "ygopro", flag = 255)]
pub struct AnyMessage {
    pub message_type: MessageType
}

impl<'bytes> From<&ygopro::message::UndeserializedBytes<'bytes, MessageType>> for AnyMessage {
    fn from(value: &ygopro::message::UndeserializedBytes<'bytes, MessageType>) -> Self {
        AnyMessage { message_type: value.message_type }
    }
}

impl FromRequest<AnyMessage> for MessageType {
    type Rejection = Infallible;

    fn from_request(request: &mut super::Bundle<AnyMessage>) -> Result<Self, Self::Rejection> {
        Ok(request.1.get_message().unwrap().message_type)
    }
}
