use serde::Serialize;
use serde::Deserialize;
use serde::de::VariantAccess;
use srvpru_proc_macros::Message;

use crate::constants::Netplayer;
use crate::constants::PlayerChange;
use crate::serde::LengthDescribed;
use crate::utils::string::FixedLengthString;
use crate::utils::string::U16String;

use super::utils::build_it;
use super::HostInfo;
use super::Message;

include!(concat!(env!("OUT_DIR"), "/server_to_client.rs"));

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 2)]
#[repr(C)]
pub struct ErrorMessage {
    pub msg: crate::constants::ErrorMessage,
    pub align: [u8; 3],
    pub code: u32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 3)]
#[repr(C)]
pub struct SelectHand;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 4)]
#[repr(C)]
pub struct SelectTp;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 5)]
#[repr(C)]
pub struct HandResult {
    pub res1: u8,
    pub res2: u8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 6)]
#[repr(C)]
pub struct TpResult {
    pub res: u8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 7)]
#[repr(C)]
pub struct ChangeSide;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 8)]
#[repr(C)]
pub struct WaitingSide;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 9)]
#[repr(C)]
pub struct DeckCount {
    pub mainc_s: u16,
    pub sidec_s: u16,
    pub extrac_s: u16,
    pub mainc_o: u16,
    pub sidec_o: u16,
    pub extrac_o: u16
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 17)]
#[repr(C)]
pub struct CreateGame {
    pub gameid: u32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 18)]
#[repr(C)]
pub struct JoinGame {
    pub info: HostInfo
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 19)]
#[repr(C)]
pub struct TypeChange {
    pub _type: u8
}
 
#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 20)]
#[repr(C)]
pub struct LeaveGame {
    pub pos: Netplayer
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 21)]
#[repr(C)]
pub struct DuelStart;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 22)]
#[repr(C)]
pub struct DuelEnd;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 23)]
#[repr(C)]
pub struct Replay {
    pub data: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 24)]
#[repr(C)]
pub struct TimeLimit {
    pub player: Netplayer,
    pub left_time: u16
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, no_length, flag = 25)]
#[repr(C)]
pub struct Chat {
    pub name: u16,
    pub msg: U16String
}

impl LengthDescribed for Chat {
    fn sizeof(&self) -> usize {
        self.msg.sizeof() + 2
    }
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 32)]
#[repr(C)]
pub struct HsPlayerEnter {
    pub name: FixedLengthString<20>,
    pub pos: Netplayer 
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 33)]
#[repr(C)]
pub struct HsPlayerChange {
    pub status: PlayerChange
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 34)]
#[repr(C)]
pub struct HsWatchChange {
    pub match_count: u16
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(stoc, flag = 48)]
#[repr(C)]
pub struct FieldFinish;

mod test {
    #![allow(unused_imports)]
    use crate::message::stoc::MessageEnum;
    use crate::message::stoc::HandResult;
    use crate::message::stoc::FieldFinish;

    #[test]
    fn message_enum_serialize() {
        let hand_result = MessageEnum::HandResult(HandResult { res1: 3, res2: 7 });
        let field_finish = MessageEnum::FieldFinish(FieldFinish {});
        assert_eq!(crate::serde::ser::serialize(&field_finish).unwrap(), vec![48, 0]);
        assert_eq!(crate::serde::ser::serialize(&hand_result).unwrap(), vec![5, 0, 3, 7])
    }

    #[test]
    fn message_enum_deserialize() {
        let hand_result: [u8; 4] = [5, 0, 3, 7];
        let field_finish: [u8; 2] = [48, 0];
        let hand_result: MessageEnum = crate::serde::de::deserialize(&hand_result).unwrap();
        let field_finish: MessageEnum = crate::serde::de::deserialize(&field_finish).unwrap();
        assert!(matches!(hand_result, MessageEnum::HandResult(_)));
        assert!(matches!(field_finish, MessageEnum::FieldFinish(_)));
    }
}
