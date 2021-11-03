// ============================================================
// random_match
// ------------------------------------------------------------
//! Enable empty, S, M, T as password,
//! for a random match.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use parking_lot::RwLock;

use crate::srvpru::Context;
use crate::srvpru::Handler;
use crate::srvpru::Room;

use crate::srvpru::generate_chat;
use crate::ygopro::Colors;
use crate::ygopro::Mode;
use crate::ygopro::message::Direction;
use crate::ygopro::message::ctos::JoinGame;
use crate::ygopro::message::stoc::DuelStart;
use crate::ygopro::message::srvpru::RoomDestroy;

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}

fn register_handlers() {
    Handler::follow_message::<JoinGame, _>(9, "random_matcher_join_game", |context, request| Box::pin(async move { 
        let mut password = context.get_string(&request.pass, "pass")?.clone();
        if password == "" { password = "M".to_string(); }
        let mode = match password.as_str() {
            "S" => Mode::Single,
            "M" => Mode::Match,
            "T" => Mode::Tag,
            _ => return Ok(false)
        };

        let room = {
            let mut random_rooms = RANDOM_ROOMS.write();
            let room_pool = random_rooms.entry(mode).or_insert(Vec::new());
            if let Some(room_position) = room_pool.iter().position(|room| can_join_room(context, room)) {
                room_pool[room_position].clone()
            }
            else {
                let room_name = password + "#random_match_" + &chrono::offset::Local::now().timestamp_millis().to_string();
                let room = Room::get_or_create_by_name(&room_name).await.clone();
                room.lock().flags.insert("random_match".to_string(), "".to_string());
                room_pool.push(room.clone());
                room
            }
        };
        let socket = context.socket.take().ok_or(anyhow!("Socket already taken."))?;
        Room::join(room.clone(), &context.addr, socket).await;
        context.send_back(&generate_chat(match mode {
            Mode::Single => "{random_duel_enter_room_single}",
            Mode::Match => "{random_duel_enter_room_match}",
            Mode::Tag => "{random_duel_enter_room_tag}",
        }, Colors::Pink, context.get_region())).await.ok();
        return Ok(true);
    })).register();

    Handler::follow_message::<DuelStart, _>(9, "random_matcher_dropper_on_duel_start", |context, _| Box::pin(async move {
        let room = context.get_room().ok_or(anyhow!("Cannot get room"))?;
        remove_room_from_pool(&room);
        Ok(false)
    })).register();

    Handler::follow_message::<RoomDestroy, _>(9, "random_matcher_dropper_on_room_termination", |_, request| Box::pin(async move {
        remove_room_from_pool(&request.room);
        Ok(false)
    })).register();

    Handler::register_handlers("random_match", Direction::CTOS, vec!("random_matcher_join_game", "random_matcher_dropper_on_duel_start"));
    Handler::register_handlers("random_match", Direction::SRVPRU, vec!("random_matcher_dropper_on_room_termination"));
}


lazy_static! {
    pub static ref RANDOM_ROOMS: RwLock<HashMap<Mode, Vec<Arc<Mutex<Room>>>>> = {
        let mut random_rooms = HashMap::new();
        random_rooms.insert(Mode::Single, Vec::new());
        random_rooms.insert(Mode::Match, Vec::new());
        random_rooms.insert(Mode::Tag, Vec::new());
        RwLock::new(random_rooms)
    };

    pub static ref PLAYER_COUNT_OF_ROOM: HashMap<Mode, usize> = {
        let mut player_count_of_room = HashMap::new();
        player_count_of_room.insert(Mode::Single, 2);
        player_count_of_room.insert(Mode::Match, 2);
        player_count_of_room.insert(Mode::Tag, 4);
        player_count_of_room
    };
}

fn can_join_room(_context: &mut Context, room: &Arc<Mutex<Room>>) -> bool {
    let room = room.lock();
    let current_count = room.players.len();
    let need_count = PLAYER_COUNT_OF_ROOM.get(&room.host_info.mode).unwrap_or(&0usize);
    current_count < *need_count
}

fn remove_room_from_pool(room: &Arc<Mutex<Room>>) {
    let mut random_rooms = RANDOM_ROOMS.write();
    let actual_room = room.lock();
    let room_pool = random_rooms.entry(actual_room.host_info.mode).or_insert(Vec::new());
    if let Some(index) = room_pool.iter().position(|pooled| Arc::ptr_eq(pooled, &room)) {
        room_pool.remove(index);
    }
}
