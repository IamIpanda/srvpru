// ============================================================
//  message 
// ------------------------------------------------------------
//! Srvpru message structs.
// ============================================================

use std::sync::Arc;

use num_enum::TryFromPrimitive;
use num_enum::IntoPrimitive;
use parking_lot::Mutex;
use serde::Serialize;
use serde::Deserialize;
use serde::ser::Serializer;
use serde::de::Deserializer;

use crate::srvpru::player::Player;
use crate::srvpru::room::Room;
use crate::srvpru::processor::ProcessorError;
use crate::srvpru::processor::ListenError;

macro_rules! not_serde_class {
    ($type: ident) => {
        impl Serialize for $type {
            fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error> where S: Serializer { panic!(concat!("Try to serialize a ", stringify!($type))); }
        }

        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(_: D) -> Result<Self, D::Error> where D: Deserializer<'de> { panic!(concat!("Try to deserialize a ", stringify!($type))); }
        }
    };
}

not_serde_class!(Player);
not_serde_class!(Room);
not_serde_class!(ProcessorError);
not_serde_class!(ListenError);

pub fn generate_message_type(_type: MessageType) -> crate::ygopro::message::MessageType {
    crate::ygopro::message::MessageType::SRVPRU(_type)
}

#[derive(Copy, Clone, TryFromPrimitive, IntoPrimitive, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
#[repr(u8)]
pub enum MessageType {
    StructSequence,    
    ServerStart,
    Reload,

    RoomCreated,
    PlayerDestroy,
    PlayerMove,
    RoomDestroy,
    STOCProcessError,
    CTOSProcessError,
    SRVPRUProcessError,
    StocListenError,
    CtosListenError,

    LpChange,
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct ServerStart;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct Reload;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[srvpru]
pub struct RoomCreated {
    pub room: Arc<Mutex<Room>>
}

#[derive(Clone, Serialize, Deserialize, Debug, Struct)]
// #[srvpru]
pub struct PlayerDestroy {
    pub player: Arc<Mutex<Player>>
}

#[derive(Clone, Serialize, Deserialize, Debug, Struct)]
// #[srvpru]
pub struct RoomDestroy {
    pub room: Arc<Mutex<Room>>
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[srvpru]
pub struct PlayerMove {
    pub post_player: Arc<Mutex<Player>>,
    pub new_player: Arc<Mutex<Player>>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct STOCProcessError {
    pub error: ProcessorError
}

impl crate::ygopro::message::Struct for STOCProcessError {}
impl crate::ygopro::message::MappedStruct for STOCProcessError {
    fn message() -> crate::ygopro::message::MessageType {
        generate_message_type(MessageType::STOCProcessError)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CTOSProcessError {
    pub error: ProcessorError
}

impl crate::ygopro::message::Struct for CTOSProcessError {}
impl crate::ygopro::message::MappedStruct for CTOSProcessError {
    fn message() -> crate::ygopro::message::MessageType {
        generate_message_type(MessageType::CTOSProcessError)
    }
}

#[derive(Serialize, Deserialize, Debug)]
// #[srvpru]
pub struct SRVPRUProcessError {
    pub error: ProcessorError
}

impl crate::ygopro::message::Struct for SRVPRUProcessError {}
impl crate::ygopro::message::MappedStruct for SRVPRUProcessError {
    fn message() -> crate::ygopro::message::MessageType {
        generate_message_type(MessageType::SRVPRUProcessError)
    }
}
#[derive(Serialize, Deserialize, Debug, Struct)]
// #[srvpru]
pub struct StocListenError {
    pub error: ListenError
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[srvpru]
pub struct CtosListenError {
    pub error: ListenError
}
#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct LpChange {
    pub player: Arc<Mutex<Player>>,
    pub lp: i32
}
