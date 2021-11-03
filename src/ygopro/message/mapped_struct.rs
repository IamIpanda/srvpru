use serde::Serialize;
use serde::Deserialize;
use crate::ygopro::message::MessageType;

pub trait Struct : erased_serde::Serialize + downcast_rs::DowncastSync + Sync + std::fmt::Debug + 'static { 
    
}
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
