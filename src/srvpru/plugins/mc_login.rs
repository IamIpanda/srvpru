// ============================================================
// mycard_login
// ------------------------------------------------------------
//! Offer password decryption for Mycard.
//! 
//! **Attention** \
//! mycard_login will break any normal room logics.
// ============================================================

use std::collections::HashMap;
use std::io::Cursor;

use num_enum::TryFromPrimitive;
use parking_lot::RwLock;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;

use crate::srvpru::Handler;
use crate::srvpru::CommonError;
use crate::srvpru::ProcessorError;
use crate::srvpru::Room;
use crate::srvpru::generate_chat;
use crate::ygopro::Colors;
use crate::ygopro::message::ctos::JoinGame;
use crate::ygopro::message::HostInfo;
use crate::ygopro::message::string::cast_to_string;

set_configuration! {
    auth_base_url: String,
    auth_access_key: String,
    permit_url: Option<String>,
    arena: Option<String>
}


use_http_client!();

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum Action {
    CreatePublicRoom = 1,
    CreatePrivateRoom = 2,
    JoinRoomById = 3,
    JoinMatch = 4,
    JoinRoomByTitle = 5
}

lazy_static! {
    pub static ref UIDS: RwLock<HashMap<String, u32>> = RwLock::new(HashMap::new());
}

fn register_handlers() {
    Handler::before_message::<JoinGame, _>(8, "mycard_login", |context, message| Box::pin(async move {
        let pass = context.get_string(&message.pass, "pass")?.clone();
        let player = context.get_player().ok_or(CommonError::PlayerNotExist)?.clone();
        let mut _player = player.lock(); 
        context.send(&generate_chat("{loading_user_info}", Colors::Babyblue, context.get_region())).await.ok();
        let uid = match read_user(&_player.name).await {
            Ok(id) => id,
            Err(_) => return context.refuse_join_game(Some("{load_user_info_fail}")).await,
        };
        let secret: u16 = (uid % 65535 + 1).try_into().unwrap();
        let encrypted = base64::decode(pass[0..8].to_string())?;
        let mut checksum_reader = Cursor::new(encrypted);
        let checksum = vec![0u8; 6];
        let mut checksum_writer = Cursor::new(checksum);
        for _ in [0..3] {
            checksum_writer.write_u16::<byteorder::LittleEndian>(checksum_reader.read_u16::<byteorder::LittleEndian>()? ^ secret)?;
        }
        if check_checksum(&checksum_writer) {
            context.set_parameter("operation", checksum_writer.into_inner());
        }
        else {
            context.send(&generate_chat("{invalid_password_checksum}", Colors::Red, context.get_region())).await.ok();
            Err(ProcessorError::Abort)?
        }
        // Set origin name, to tell other plugins this user is verified.
        let name = _player.name.clone();
        _player.try_set_origin_name(name);
        Ok(false)
    })).register_for_plugin("mc_login");

    Handler::before_message::<JoinGame, _>(9, "mycard_room", |context, message| Box::pin(async move {
        let bytes = context.get_parameter::<Vec<u8>>("operation").ok_or(CommonError::IllegalType)?;
        let first_byte = bytes[0];
        let action = Action::try_from(first_byte >> 4)?;
        let hostname = match action {
            Action::CreatePublicRoom | Action::CreatePrivateRoom => {
                let opt0 = bytes[1] & 0xF;
                let opt1 = bytes[2];
                let opt2 = u16::from_le_bytes([bytes[3], bytes[4]]);
                let opt3 = bytes[5];
                let hostinfo = HostInfo {
                    lflist: 1,
                    time_limit: 240,
                    rule: (opt1 >> 5) & 0x7,
                    mode: crate::ygopro::Mode::try_from((opt1 >> 3) & 0x3)?,
                    padding: [0; 3],
                    duel_rule: (opt0 >> 1) | 0x5,
                    no_check_deck: (opt1 >> 1) & 1 == 1,
                    no_shuffle_deck: opt1 & 1 == 1,
                    start_lp: opt2 as u32,
                    start_hand: opt3 >> 4,
                    draw_count: opt3 & 0xF,
                };
                let pass = hostinfo.to_string();
                let room_name = cast_to_string(&message.pass[8..]).ok_or(CommonError::IllegalString)?;
                if matches!(action, Action::CreatePrivateRoom) {
                    context.set_parameter("flag_private", true);
                }
                pass + "," + &room_name
            },
            Action::JoinMatch => {
                context.set_parameter("flag_arena", true);
                format!("M#{:}", cast_to_string(&message.pass[8..]).ok_or(CommonError::IllegalString)?)
            },
            // I can't understand why title and name are different in srvpro.
            Action::JoinRoomByTitle | Action::JoinRoomById => {
                let room_name = cast_to_string(&message.pass[8..]).ok_or(CommonError::IllegalString)?;
                if ! Room::exist(&room_name) {
                    return context.refuse_join_game(Some("{invalid_password_not_found}")).await;
                }
                room_name
            },
        };
        context.set_parameter("pass", hostname);
        Ok(false)
    })).register_for_plugin("mc_login");
}

async fn read_user(username: &String) -> anyhow::Result<u32> {
    if let Some(uid) = UIDS.read().get(username) { return Ok(*uid); }
    let configuration = get_configuration();
    let user: MyCardUserWrapper = get_http_client()
        .get(format!("{}/users/{}.json?api_key={}&skip_track_visit=true", configuration.auth_base_url, urlencoding::encode(username), configuration.auth_access_key))
        .send().await?
        .json().await?;
    let uid = user.user.id;
    UIDS.write().insert(username.clone(), uid);
    Ok(uid)
}

fn check_checksum(cursor: &Cursor<Vec<u8>>) -> bool {
    let sum: u64 = cursor.get_ref().iter().map(|i| *i as u64).sum();
    sum % 0x100 == 0
}

#[derive(serde::Deserialize)]
struct MycardUser {
    id: u32,
    //avatar: String,
    //email: String,
    //username: String
}

#[derive(serde::Deserialize)]
struct MyCardUserWrapper {
    user: MycardUser
}