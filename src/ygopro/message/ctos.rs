// ============================================================
//  ctos
// ------------------------------------------------------------
/// Message types sent from client to server.
// ============================================================


use serde::Serialize;
use serde::Deserialize;
use num_enum::TryFromPrimitive;
use num_enum::IntoPrimitive;

use crate::ygopro::Netplayer;
use crate::ygopro::message::HostInfo;
use crate::ygopro::message::GreedyVector;

#[derive(Copy, Clone, TryFromPrimitive, IntoPrimitive, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
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
    HsToDuelist = 32,
    HsToOBServer = 33,
    HsReady = 34,
    HsNotReady = 35,
    HsKick = 36,
    HsStart = 37,
    RequestField = 48
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct UpdateDeck {
    pub deck: crate::ygopro::data::Deck
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct HandResult {
    pub res: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct TpResult {
    pub res: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct PlayerInfo {
    pub name: [u16; 20]
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct CreateGame {
    pub info: HostInfo,
    pub name: [u16; 20],
    pub pass: [u16; 20]
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct JoinGame {
    pub version: u16,
    pub align: u16,
    pub gameid: u32,
    pub pass: [u16; 20]
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct LeaveGame;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct Surrender;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct TimeConfirm;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct Chat {
    #[serde(with = "GreedyVector::<255>")]
    pub msg: Vec<u16>
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct HsToDuelist;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct HsToOBServer;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct HsReady;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct HsNotReady;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct HsKick {
    pub pos: Netplayer
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct HsStart;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct RequestField;

pub fn generate_message_type(_type: MessageType) -> crate::ygopro::message::MessageType {
    crate::ygopro::message::MessageType::CTOS(_type)
}
