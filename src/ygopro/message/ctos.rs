use serde::Serialize;
use serde::Deserialize;
use num_enum::TryFromPrimitive;
use num_enum::IntoPrimitive;

use crate::ygopro::Netplayer;
use crate::ygopro::message::HostInfo;
use crate::ygopro::message::GreedyVector;

#[derive(Copy, Clone, TryFromPrimitive, IntoPrimitive, Eq, PartialEq, Debug, Hash)]
#[repr(u8)]
pub enum MessageType {
    Response = 1,
    UpdateDeck = 2,
    HandResult = 3,
    TpResult = 4,
    PlayerInfo = 16,
    CreateGame = 17,
    JoinGame = 18,
    LeaveGame = 19,
    Surrender = 20,
    TimeConfirm = 21,
    Chat = 22,
    HsTodueList = 32,
    HsToOBServer = 33,
    HsReady = 34,
    HsNotReady = 35,
    HsKick = 36,
    HsStart = 37,
    RequestField = 48
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct UpdateDeck {
    pub mainc: usize,
    pub sidec: usize,
    #[serde(with = "GreedyVector::<90>")]
    pub deckbuf: Vec<u32>
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct HandResult {
    pub res: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct TpResult {
    pub res: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct PlayerInfo {
    pub name: [u16; 20]
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct CreateGame {
    pub info: HostInfo,
    pub name: [u16; 20],
    pub pass: [u16; 20]
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct JoinGame {
    pub version: u16,
    pub align: u16,
    pub gameid: u32,
    pub pass: [u16; 20]
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct LeaveGame;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct Surrender;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct TimeConfirm;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct Chat {
    #[serde(with = "GreedyVector::<255>")]
    pub msg: Vec<u16>
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct HsTodueList;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct HsToOBServer;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct HsReady;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct HsNotReady;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct HsKick {
    pub pos: Netplayer
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct HsStart;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[ctos]
pub struct RequestField;

pub fn generate_message_type(_type: MessageType) -> crate::ygopro::message::MessageType {
    crate::ygopro::message::MessageType::CTOS(_type)
}
