// Fake message structs.

use std::sync::Arc;

use num_enum::TryFromPrimitive;
use num_enum::IntoPrimitive;
use parking_lot::Mutex;
use serde::Serialize;
use serde::Deserialize;
use serde::ser::Serializer;
use serde::de::Deserializer;

use crate::ygopro::message::Struct;
use crate::ygopro::message::MappedStruct;
use crate::ygopro::message::MessageType;
use crate::srvpru::player::Player;
use crate::srvpru::room::Room;
use crate::srvpru::processor::ProcessorError;


impl Serialize for Player {
    fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error> where S: Serializer { panic!("Try to serialize a player"); }
}

impl Serialize for Room {
    fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error> where S: Serializer { panic!("Try to serialize a room"); }
}

impl Serialize for ProcessorError {
    fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error> where S: Serializer { panic!("Try to serialize a processor error"); }
}


impl<'de> Deserialize<'de> for Player {
    fn deserialize<D>(_: D) -> Result<Self, D::Error> where D: Deserializer<'de> { panic!("Try to deserialize a player"); }
}

impl<'de> Deserialize<'de> for Room {
    fn deserialize<D>(_: D) -> Result<Self, D::Error> where D: Deserializer<'de> { panic!("Try to deserialize a room"); }
}

impl<'de> Deserialize<'de> for ProcessorError {
    fn deserialize<D>(_: D) -> Result<Self, D::Error> where D: Deserializer<'de> { panic!("Try to deserialize a processor error"); }
}

#[derive(Copy, Clone, TryFromPrimitive, IntoPrimitive, Eq, PartialEq, Debug, Hash)]
#[repr(u8)]
pub enum SRVPRUMessageType {
    RoomCreated,
    PlayerDestroy,
    PlayerMove,
    RoomDestroy,
    StocProcessError,
    CtosProcessError,
    InternalProcessError,
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct RoomCreated {
    pub room: Arc<Mutex<Room>>
}

#[derive(Clone, Serialize, Deserialize, Debug, Struct)]
pub struct PlayerDestroy {
    pub player: Arc<Mutex<Player>>
}

#[derive(Clone, Serialize, Deserialize, Debug, Struct)]
pub struct RoomDestroy {
    pub room: Arc<Mutex<Room>>
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct PlayerMove {
    pub post_player: Arc<Mutex<Player>>,
    pub new_player: Arc<Mutex<Player>>
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct StocProcessError {
    pub error: ProcessorError
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct CtosProcessError {
    pub error: ProcessorError
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct InternalProcessError {
    pub error: ProcessorError
}
