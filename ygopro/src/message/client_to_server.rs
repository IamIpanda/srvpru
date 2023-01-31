use serde::Serialize;
use serde::Deserialize;
use serde::de::VariantAccess;
use srvpru_proc_macros::Message;

use crate::data::Deck;
use crate::serde::LengthDescribed;
use crate::utils::string::FixedLengthString;
use crate::utils::string::U16String;

use super::HostInfo;
use super::Message;
use super::utils::build_it;

include!(concat!(env!("OUT_DIR"), "/client_to_server.rs"));

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 2)]
#[repr(C)]
pub struct UpdateDeck {
    pub deck: Deck
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 3)]
#[repr(C)]
pub struct HandResult {
    pub res: u8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 4)]
#[repr(C)]
pub struct TpResult {
    pub res: u8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 16)]
#[repr(C)]
pub struct PlayerInfo {
    pub name: FixedLengthString<20>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 17)]
#[repr(C)]
pub struct CreateGame {
    pub info: HostInfo,
    pub name: FixedLengthString<20>,
    pub pass: FixedLengthString<20>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 18)]
#[repr(C)]
pub struct JoinGame {
    pub version: u16,
    pub align: u16,
    pub gameid: u32,
    pub pass: FixedLengthString<20>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 19)]
#[repr(C)]
pub struct LeaveGame;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 20)]
#[repr(C)]
pub struct Surrender;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 21)]
#[repr(C)]
pub struct TimeConfirm;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, no_length, flag = 22)]
#[repr(C)]
pub struct Chat {
    pub msg: U16String
}

impl LengthDescribed for Chat {
    fn sizeof(&self) -> usize {
        self.msg.sizeof()
    }
}

impl From<String> for Chat {
    fn from(value: String) -> Self {
        Self { msg: value.into() }
    }
}

impl<'s> From<&'s str> for Chat {
    fn from(value: &'s str) -> Self {
        Self { msg: value.into() }
    }
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 32)]
#[repr(C)]
pub struct HsToDuelist;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 33)]
#[repr(C)]
pub struct HsToOBServer;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 34)]
#[repr(C)]
pub struct HsReady;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 35)]
#[repr(C)]
pub struct HsNotReady;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 36)]
#[repr(C)]
pub struct HsKick {
    pub pos: crate::constants::Netplayer
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 37)]
#[repr(C)]
pub struct HsStart;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(ctos, flag = 48)]
#[repr(C)]
pub struct RequestField;
