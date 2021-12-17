// ============================================================
//  struct
// ------------------------------------------------------------
/// Offer basic contracts and structs for message.
// ============================================================
use serde::Serialize;
use serde::Deserialize;
use crate::ygopro::message::MessageType;

// ------------------------------------------------------------
//  Struct
// ------------------------------------------------------------
/// Every part of the ygopro message need to impl Struct.
/// 
/// No actual content.
// ------------------------------------------------------------
pub trait Struct : erased_serde::Serialize + downcast_rs::DowncastSync + Sync + std::fmt::Debug + 'static { 
    
}

// ------------------------------------------------------------
//  MappedStruct
// ------------------------------------------------------------
/// MappedStruct means this struct can be mapped to a specific
/// [MessageType].
// ------------------------------------------------------------
pub trait MappedStruct: Struct {
    fn message() -> MessageType;
}

impl_downcast!(sync Struct);
serialize_trait_object!(Struct);


// ------------------------------------------------------------
//  Empty
// ------------------------------------------------------------
/// [`Struct`] null.
// ------------------------------------------------------------
#[derive(Serialize, Deserialize, Debug)]
pub struct Empty {

}
impl Struct for Empty { }

#[derive(Serialize, Deserialize, Clone, Debug, serde_default)]
#[repr(C)]
pub struct HostInfo {
    #[serde(default="HostInfo::default_lflist")]
    pub lflist: i32,
    #[serde(default="HostInfo::default_rule")]
    pub rule: u8,
    #[serde(default="HostInfo::default_mode")]
    pub mode: crate::ygopro::constants::Mode,
    #[serde(default="HostInfo::default_duel_rule")]
    pub duel_rule: u8,
    #[serde(default)]
    pub no_check_deck: bool,
    #[serde(default)]
    pub no_shuffle_deck: bool,
    #[serde(default="HostInfo::default_padding")]
    pub padding: [u8; 3],
    #[serde(default="HostInfo::default_start_lp")]
    pub start_lp: u32,
    #[serde(default="HostInfo::default_start_hand")]
    pub start_hand: u8,
    #[serde(default="HostInfo::default_draw_count")]
    pub draw_count: u8,
    #[serde(default="HostInfo::default_time_limit")]
    pub time_limit: u16
}
impl Struct for HostInfo {}
impl HostInfo {
    fn default_lflist() -> i32 { 0 }
    fn default_rule() -> u8 { 0 }
    fn default_mode() -> crate::ygopro::constants::Mode { crate::ygopro::constants::Mode::Match }
    fn default_duel_rule() -> u8 { 5 }
    fn default_padding() -> [u8; 3] { [0; 3] }
    fn default_start_lp() -> u32 { 8000 }
    fn default_start_hand() -> u8 { 5 }
    fn default_draw_count() -> u8 { 1 }
    fn default_time_limit() -> u16 { 180 }
}

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
