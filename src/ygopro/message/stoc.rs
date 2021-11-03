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
    GameMessage = 1,
    ErrorMessage = 2,
    SelectHand = 3,
    SelectTp = 4,
    HandResult = 5,
    TpResult = 6,
    ChangeSide = 7,
    WaitingSide = 8,
    DeckCount = 9,
    CreateGame = 17,
    JoinGame = 18,
    TypeChange = 19,
    LeaveGame = 20,
    DuelStart = 21,
    DuelEnd = 22,
    Replay = 23,
    TimeLimit = 24,
    Chat = 25,
    HsPlayerEnter = 32,
    HsPlayerChange = 33,
    HsWatchChange = 34,
    FieldFinish = 48
}

pub type GameMessage = super::game_message::GameMessage;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct ErrorMessage {
    pub msg: crate::ygopro::ErrorMessage,
    pub align: [u8; 3],
    pub code: u32
}
#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct SelectHand;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct SelectTp;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct HandResult {
    pub res1: u8,
    pub res2: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct TpResult {
    pub res: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct ChangeSide;

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct WaitingSide;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct CreateGame {
    pub gameid: u32
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct JoinGame {
    pub info: HostInfo
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct TypeChange {
    pub _type: u8
}
 
#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct LeaveGame {
    pub pos: Netplayer
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct DuelStart;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct DuelEnd;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct Replay {
    #[serde(with = "GreedyVector::<65536>")]
    pub data: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct TimeLimit {
    pub player: Netplayer,
    pub left_time: u16
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct Chat {
    pub name: u16,
    #[serde(with = "GreedyVector::<255>")]
    pub msg: Vec<u16>
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct HsPlayerEnter {
    pub name: [u16; 20],
    pub pos: u8 
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct HsPlayerChange {
    pub status: u8 
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct HsWatchChange {
    pub match_count: u16
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct FieldFinish;

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[stoc]
pub struct DeckCount {
    pub mainc_s: u16,
    pub sidec_s: u16,
    pub extrac_s: u16,
    pub mainc_o: u16,
    pub sidec_o: u16,
    pub extrac_o: u16
}

pub fn generate_message_type(_type: MessageType) -> crate::ygopro::message::MessageType {
    crate::ygopro::message::MessageType::STOC(_type)
}
