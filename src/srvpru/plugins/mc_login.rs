// ============================================================
// mycard_login
// ------------------------------------------------------------
//! Offer password decryption for Mycard.
// ============================================================

use num_enum::TryFromPrimitive;

use crate::srvpru::Handler;
use crate::ygopro::message::Direction;
use crate::ygopro::message::CTOSJoinGame;
use crate::ygopro::message::HostInfo;

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum Action {
    CreatePublicRoom = 1,
    CreatePrivateRoom = 2,
    JoinRoomById = 3,
    CreateOrJoinRoomById = 4,
    JoinRoomByTitle = 5
}

fn register_handlers() {
    Handler::follow_message::<CTOSJoinGame, _>(9, "mycard_room", |context, request| Box::pin(async move {
        let encrypted = context.get_string(&request.pass, "pass")?;
        let bytes = encrypted.as_bytes();
        let first_byte = bytes[0];
        let action = Action::try_from(first_byte >> 4)?;
        match action {
            Action::CreatePublicRoom | Action::CreatePrivateRoom => {
                let opt0 = bytes[1] & 0xF;
                let opt1 = bytes[2];
                let opt2 = u16::from_le_bytes([bytes[3], bytes[4]]);
                let opt3 = bytes[5];
                let hostinfo = HostInfo {
                    lflist: 1,
                    time_limit: 240,
                    rule: (opt1 >> 5) & 0x7,
                    mode: crate::ygopro::constants::Mode::try_from((opt1 >> 3) & 0x3)?,
                    padding: [0; 3],
                    duel_rule: (opt0 >> 1) | 0x5,
                    no_check_deck: (opt1 >> 1) & 1 == 1,
                    no_shuffle_deck: opt1 & 1 == 1,
                    start_lp: opt2 as u32,
                    start_hand: opt3 >> 4,
                    draw_count: opt3 & 0xF,
                };
                context.parameters.insert("pass".to_string(), hostinfo.to_string());
            },
            Action::JoinRoomById => {},
            Action::CreateOrJoinRoomById => {},
            Action::JoinRoomByTitle => {},
        }

        Ok(false)
    })).register();

    Handler::register_handlers("mycard_login", Direction::CTOS, vec!["mycard_room"]);
}