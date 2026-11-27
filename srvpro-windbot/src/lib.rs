#![doc = include_str!("../README.md")]

use std::error::Error;
use std::ffi::CStr;
use std::ffi::c_char;
use std::ffi::c_void;
use std::fmt;

use serde::Serialize;
use ygopro_data::data::Card;

pub use srvpro::managers;
pub mod plugin;

#[derive(Serialize)]
struct CardRecord {
    name: String,
    desc: String,
    hint: Vec<String>,
    code: u32,
    alias: u32,
    setcode: Vec<u16>,
    card_type: u32,
    level: u32,
    attribute: u32,
    race: u32,
    attack: i32,
    defense: i32,
    lscale: u32,
    rscale: u32,
    link_marker: u32,
    ot: u8,
    category: u32,
}

impl From<&Card> for CardRecord {
    fn from(card: &Card) -> Self {
        Self {
            name: card.name.clone(),
            desc: card.text.clone(),
            hint: card.desc.to_vec(),
            code: card.code,
            alias: card.alias,
            setcode: card.setcode.to_vec(),
            card_type: card.card_type.bits(),
            level: card.level,
            attribute: card.attribute.bits(),
            race: card.race.bits(),
            attack: card.attack,
            defense: card.defense,
            lscale: card.left_scale,
            rscale: card.right_scale,
            link_marker: card.link_marker.bits(),
            ot: card.ot.bits(),
            category: card.category.bits(),
        }
    }
}

/// Failure reported by the embedded WindBot library.
#[derive(Debug)]
pub struct WindBotError(String);

impl fmt::Display for WindBotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for WindBotError {}

unsafe extern "C" {
    fn windbot_load_cards(json: *const u8, length: usize) -> i32;
    fn windbot_run(json: *const u8, length: usize) -> i32;
    fn windbot_list() -> *mut c_char;
    fn windbot_last_error() -> *mut c_char;
    fn windbot_free(value: *mut c_void);
}

/// Replaces the card database used by every bot started afterwards.
///
/// Fails while any bot is running.
pub fn load_cards<'a>(cards: impl IntoIterator<Item = &'a Card>) -> Result<(), WindBotError> {
    let records: Vec<CardRecord> = cards.into_iter().map(CardRecord::from).collect();
    let json: Vec<u8> = serde_json::to_vec(&records).map_err(|error: serde_json::Error| WindBotError(error.to_string()))?;
    check_status(unsafe { windbot_load_cards(json.as_ptr(), json.len()) })
}

/// Returns one JSON row per deck AI: display name, AI name, embedded deck,
/// dialog set and level.
pub fn list_bots() -> Result<String, WindBotError> {
    let pointer: *mut c_char = unsafe { windbot_list() };
    if pointer.is_null() { return Err(last_error()) }
    take_string(pointer)
}

/// Runs one bot until it disconnects, blocking the calling thread.
///
/// `options` is a JSON object using `WindBotInfo` field names; `None` connects
/// to `127.0.0.1:7911` with a random `Normal` deck. [`load_cards`] must have
/// been called first.
pub fn run(options: Option<&str>) -> Result<(), WindBotError> {
    let json: &str = options.unwrap_or("{}");
    check_status(unsafe { windbot_run(json.as_ptr(), json.len()) })
}

fn check_status(status: i32) -> Result<(), WindBotError> {
    if status == 0 { Ok(()) } else { Err(last_error()) }
}

fn last_error() -> WindBotError {
    let pointer: *mut c_char = unsafe { windbot_last_error() };
    if pointer.is_null() {
        return WindBotError("WindBot returned no error message.".into());
    }
    match take_string(pointer) {
        Ok(message) => WindBotError(message),
        Err(error) => error,
    }
}

fn take_string(pointer: *mut c_char) -> Result<String, WindBotError> {
    let result: Result<String, WindBotError> = unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .map(str::to_owned)
        .map_err(|error: std::str::Utf8Error| WindBotError(error.to_string()));
    unsafe { windbot_free(pointer.cast()) };
    result
}
