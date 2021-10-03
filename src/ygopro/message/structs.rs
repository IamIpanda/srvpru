use serde::{Serialize, Deserialize};
use super::constants::{CTOSMessageType,STOCMessageType,MessageType};
include!("greedy_vec.rs");

big_array! { B90; 90 }
big_array! { B255; 255 }
greedy_vec! { 90, 255, }


pub trait Struct : erased_serde::Serialize + downcast_rs::DowncastSync + Sync + std::fmt::Debug + 'static { }
impl_downcast!(sync Struct);
serialize_trait_object!(Struct);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct HostInfo {
    pub lflist: i32,
    pub rule: u8,
    pub mode: u8,
    pub duel_rule: u8,
    pub no_check_deck: bool,
    pub no_shuffle_deck: bool,
    pub start_lp: u32,
    pub start_hand: u8,
    pub draw_count: u8,
    pub time_limit: u8
}
impl Struct for HostInfo {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct HostPacket {
    pub identifier: u16,
    pub version: u16,
    pub port: u16,
    pub ipaddr: u32,
    pub name: [u16; 20],
    pub host: HostInfo
}
impl Struct for HostPacket {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct HostRequest {
    pub identifier: u16
}
impl Struct for HostRequest {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct CTOSHandResult {
    pub res: u8
}
impl Struct for CTOSHandResult {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct CTOSTPResult {
    pub res: u8
}
impl Struct for CTOSTPResult {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct CTOSPlayerInfo {
    pub name: [u16; 20]
}
impl Struct for CTOSPlayerInfo {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct CTOSCreateGame {
    pub info: HostInfo,
    pub name: [u16; 20],
    pub pass: [u16; 20]
}
impl Struct for CTOSCreateGame {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct CTOSJoinGame {
    pub version: u16,
    pub align: u16,
    pub gameid: u32,
    pub pass: [u16; 20]
}
impl Struct for CTOSJoinGame {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct CTOSKick {
    pub pos: u8
}
impl Struct for CTOSKick {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCErrorMsg {
    pub msg: u8,
    pub align1: u8,
    pub align2: u8,
    pub align3: u8,
    pub code: u32
}
impl Struct for STOCErrorMsg {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCHandResult {
    pub res1: u8,
    pub res2: u8
}
impl Struct for STOCHandResult {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCCreateGame {
    pub gameid: u32
}
impl Struct for STOCCreateGame {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCJoinGame {
    pub info: HostInfo
}
impl Struct for STOCJoinGame {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCTypeChange {
    pub _type: u8
}
impl Struct for STOCTypeChange {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCExitgame {
    pub pos: u8
}
impl Struct for STOCExitgame {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCTimeLimit {
    pub player: u8,
    pub left_time: u16
}
impl Struct for STOCTimeLimit {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCChat {
    pub name: u16,
    #[serde(with = "GreedyVec::<255>")]
    pub msg: Vec<u16>
}
impl Struct for STOCChat {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCHSPlayerEnter {
    pub name: [u16; 20],
    pub pos: u8
}
impl Struct for STOCHSPlayerEnter {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCHSPlayerChange {
    pub status: u8
}
impl Struct for STOCHSPlayerChange {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCHSWatchChange {
    pub match_count: u16
}
impl Struct for STOCHSWatchChange {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GameMsgHintCardonly {
    pub curmsg: u8,
    pub _type: u8,
    pub player: u8,
    pub data: u16
}
impl Struct for GameMsgHintCardonly {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct UpdateDeck {
    pub mainc: u32,
    pub sidec: u32,
    #[serde(with = "GreedyVec::<90>")]
    pub deckbuf: Vec<u32>
}

impl Struct for UpdateDeck {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct CTOSChat {
    #[serde(with = "GreedyVec::<255>")]
    pub msg: Vec<u16>
}

impl Struct for CTOSChat {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct STOCDeckCount {
    pub mainc_s: u16,
    pub sidec_s: u16,
    pub extrac_s: u16,
    pub mainc_o: u16,
    pub sidec_o: u16,
    pub extrac_o: u16
}
impl Struct for STOCDeckCount {}

fn deserialize_struct<'a, T>(data: &'a [u8]) -> Option<Box<dyn Struct>> where T: serde::de::Deserialize<'a> + Struct {
    Some(Box::new(bincode::deserialize::<T>(data).unwrap()))
}

fn try_deserialize_struct<'a, T>(data: &'a [u8]) -> Option<Box<dyn Struct>> where T: serde::de::Deserialize<'a> + Struct {
    let data = bincode::deserialize::<T>(data);
    if let Ok(inner_data) = data { return Some(Box::new(inner_data)) }
    else { None }
}

pub fn deserialize_component(direction: MessageType, data: &[u8]) -> Option<Box<dyn Struct>> {
    match direction {
        MessageType::CTOS(ctos_type) => {
            match ctos_type {
                CTOSMessageType::HandResult => deserialize_struct::<CTOSHandResult>(data),
                CTOSMessageType::TpResult => deserialize_struct::<CTOSTPResult>(data),
                CTOSMessageType::PlayerInfo => deserialize_struct::<CTOSPlayerInfo>(data),
                CTOSMessageType::JoinGame => deserialize_struct::<CTOSJoinGame>(data),
                CTOSMessageType::HsKick => deserialize_struct::<CTOSKick>(data),
                CTOSMessageType::UpdateDeck => deserialize_struct::<UpdateDeck>(data),
                CTOSMessageType::Chat => deserialize_struct::<CTOSChat>(data),
                _ => Option::None
            }
        }
        MessageType::STOC(stoc_type) => {
            match stoc_type {
                STOCMessageType::JoinGame => deserialize_struct::<STOCJoinGame>(data),
                STOCMessageType::HsWatchChange => deserialize_struct::<STOCHSWatchChange>(data),
                STOCMessageType::TypeChange => deserialize_struct::<STOCTypeChange>(data),
                STOCMessageType::HsPlayerChange => deserialize_struct::<STOCHSPlayerChange>(data),
                STOCMessageType::HsPlayerEnter => deserialize_struct::<STOCHSPlayerEnter>(data),
                STOCMessageType::ErrorMessage => deserialize_struct::<STOCErrorMsg>(data),
                STOCMessageType::GameMessage => try_deserialize_struct::<GameMsgHintCardonly>(data),
                STOCMessageType::HandResult => deserialize_struct::<STOCHandResult>(data),
                STOCMessageType::TimeLimit => deserialize_struct::<STOCTimeLimit>(data),
                STOCMessageType::Chat => deserialize_struct::<STOCChat>(data),
                STOCMessageType::DeckCount => deserialize_struct::<STOCDeckCount>(data),
                _ => Option::None
            }
        }
    }
}

pub fn cast_to_string(array: &[u16]) -> Option<String> {
    let mut str = array;
    if let Some(index) = array.iter().position(|&i| i == 0) {
        str = &str[0..index as usize];
    }
    else { return None }
    let body = unsafe { std::slice::from_raw_parts(str.as_ptr() as *const u8, str.len() * 2) };
    let (cow, _, had_errors) = encoding_rs::UTF_16LE.decode(&body);
    if had_errors { None }
    else { Some(cow.to_string()) }
}