use serde::{Serialize, Deserialize};
use super::constants::{CTOSMessageType,STOCMessageType,MessageType};
include!("greedy_vec.rs");
greedy_vec! { 90, 255 }

pub trait Struct : erased_serde::Serialize + downcast_rs::DowncastSync + Sync + std::fmt::Debug + 'static { }
pub trait MappedStruct {
    fn message() -> MessageType;
}

impl_downcast!(sync Struct);
serialize_trait_object!(Struct);

#[derive(Serialize, Deserialize, Debug)]
pub struct Empty {

}
impl Struct for Empty { }

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub struct HostInfo {
    pub lflist: i32,
    pub rule: u8,
    pub mode: crate::ygopro::constants::Mode,
    pub duel_rule: u8,
    pub no_check_deck: bool,
    pub no_shuffle_deck: bool,
    pub padding: [u8; 3],
    pub start_lp: u32,
    pub start_hand: u8,
    pub draw_count: u8,
    pub time_limit: u16
}
impl Struct for HostInfo {}

#[derive(Serialize, Deserialize, Debug)]
pub struct HostPacket {
    pub identifier: u16,
    pub version: u16,
    pub port: u16,
    pub ipaddr: u32,
    pub name: [u16; 20],
    pub host: HostInfo
}
impl Struct for HostPacket {}

#[derive(Serialize, Deserialize, Debug)]
pub struct HostRequest {
    pub identifier: u16
}
impl Struct for HostRequest {}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct CTOSHandResult {
    pub res: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct CTOSTpResult {
    pub res: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct CTOSPlayerInfo {
    pub name: [u16; 20]
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct CTOSCreateGame {
    pub info: HostInfo,
    pub name: [u16; 20],
    pub pass: [u16; 20]
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct CTOSJoinGame {
    pub version: u16,
    pub align: u16,
    pub gameid: u32,
    pub pass: [u16; 20]
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct CTOSHsKick {
    pub pos: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct STOCErrorMessage {
    pub msg: u8,
    pub align1: u8,
    pub align2: u8,
    pub align3: u8,
    pub code: u32
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct STOCHandResult {
    pub res1: u8,
    pub res2: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct STOCCreateGame {
    pub gameid: u32
}
#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct STOCJoinGame {
    pub info: HostInfo
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct STOCTypeChange {
    pub _type: u8
}
 
#[derive(Serialize, Deserialize, Debug)]
pub struct STOCExitgame {
    pub pos: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct STOCTimeLimit {
    pub player: u8,
    pub left_time: u16
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct STOCChat {
    pub name: u16,
    #[serde(with = "GreedyVec::<255>")]
    pub msg: Vec<u16>
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct STOCHsPlayerEnter {
    pub name: [u16; 20],
    pub pos: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct STOCHsPlayerChange {
    pub status: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct STOCHsWatchChange {
    pub match_count: u16
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct CTOSUpdateDeck {
    pub mainc: usize,
    pub sidec: usize,
    #[serde(with = "GreedyVec::<90>")]
    pub deckbuf: Vec<u32>
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct CTOSChat {
    #[serde(with = "GreedyVec::<255>")]
    pub msg: Vec<u16>
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct STOCDeckCount {
    pub mainc_s: u16,
    pub sidec_s: u16,
    pub extrac_s: u16,
    pub mainc_o: u16,
    pub sidec_o: u16,
    pub extrac_o: u16
}

include!("game_message.rs");

pub fn deserialize_struct<'a, T>(data: &'a [u8]) -> Option<Box<dyn Struct>> where T: serde::de::Deserialize<'a> + Struct {
    Some(Box::new(bincode::deserialize::<T>(data).unwrap()))
}

fn try_deserialize_struct<'a, T>(data: &'a [u8]) -> Option<Box<dyn Struct>> where T: serde::de::Deserialize<'a> + Struct {
    let data = bincode::deserialize::<T>(data);
    if let Ok(inner_data) = data { return Some(Box::new(inner_data)) }
    else { None }
}

pub fn deserialize_struct_by_type(direction: MessageType, data: &[u8]) -> Option<Box<dyn Struct>> {
    match direction {
        MessageType::CTOS(ctos_type) => {
            match ctos_type {
                CTOSMessageType::HandResult => deserialize_struct::<CTOSHandResult>(data),
                CTOSMessageType::TpResult => deserialize_struct::<CTOSTpResult>(data),
                CTOSMessageType::PlayerInfo => deserialize_struct::<CTOSPlayerInfo>(data),
                CTOSMessageType::JoinGame => deserialize_struct::<CTOSJoinGame>(data),
                CTOSMessageType::HsKick => deserialize_struct::<CTOSHsKick>(data),
                CTOSMessageType::UpdateDeck => deserialize_struct::<CTOSUpdateDeck>(data),
                CTOSMessageType::Chat => deserialize_struct::<CTOSChat>(data),
                _ => Option::None
            }
        }
        MessageType::STOC(stoc_type) => {
            match stoc_type {
                STOCMessageType::JoinGame => deserialize_struct::<STOCJoinGame>(data),
                STOCMessageType::HsWatchChange => deserialize_struct::<STOCHsWatchChange>(data),
                STOCMessageType::TypeChange => deserialize_struct::<STOCTypeChange>(data),
                STOCMessageType::HsPlayerChange => deserialize_struct::<STOCHsPlayerChange>(data),
                STOCMessageType::HsPlayerEnter => deserialize_struct::<STOCHsPlayerEnter>(data),
                STOCMessageType::ErrorMessage => deserialize_struct::<STOCErrorMessage>(data),
                STOCMessageType::GameMessage => try_deserialize_struct::<STOCGameMessage>(data),
                STOCMessageType::HandResult => deserialize_struct::<STOCHandResult>(data),
                STOCMessageType::TimeLimit => deserialize_struct::<STOCTimeLimit>(data),
                STOCMessageType::Chat => deserialize_struct::<STOCChat>(data),
                STOCMessageType::DeckCount => deserialize_struct::<STOCDeckCount>(data),
                _ => Option::None
            }
        }
        _ => { panic!("Try to deserialize an unreal message type.") }
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

pub fn cast_to_c_array(message: &str) -> Vec<u16> {
    let mut vector: Vec<u16> = message.encode_utf16().collect();
    vector.push(0);
    vector
}

pub fn cast_to_array<const N: usize>(message: &str) -> [u16; N] {
    let mut data = [0u16; N];
    for (index, chr) in message.encode_utf16().enumerate() {
        data[index] = chr;
    }
    data
}


pub fn wrap_mapped_struct<T: Struct + MappedStruct + serde::Serialize>(data: &T) -> Vec<u8> {
    return wrap_data(&T::message(), &bincode::serialize(&data).unwrap());
}

pub fn wrap_struct(message_type: &MessageType, data: &(impl Struct + serde::Serialize)) -> Vec<u8> {
    return wrap_data(message_type, &bincode::serialize(&data).unwrap());
}

pub fn wrap_data(message_type: &MessageType, data: &[u8]) -> Vec<u8> {
    let size = data.len() + 1;
    let type_code: u8 = message_type.into();
    let mut message = vec!((size % 256) as u8, (size / 256) as u8, type_code);
    message.extend_from_slice(data);
    return message;
}