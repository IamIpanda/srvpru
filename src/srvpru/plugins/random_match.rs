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

use crate::srvpru::processor::Context;
use crate::srvpru::processor::Handler;
use crate::srvpru::Room;
use crate::srvpru::structs::RoomDestroy;
use crate::ygopro::message::*;
use crate::ygopro::constants;

lazy_static! {
    pub static ref RANDOM_ROOMS: RwLock<HashMap<constants::Mode, Vec<Arc<Mutex<Room>>>>> = {
        let mut random_rooms = HashMap::new();
        random_rooms.insert(constants::Mode::Single, Vec::new());
        random_rooms.insert(constants::Mode::Match, Vec::new());
        random_rooms.insert(constants::Mode::Tag, Vec::new());
        RwLock::new(random_rooms)
    };

    pub static ref PLAYER_COUNT_OF_ROOM: HashMap<constants::Mode, usize> = {
        let mut player_count_of_room = HashMap::new();
        player_count_of_room.insert(constants::Mode::Single, 2);
        player_count_of_room.insert(constants::Mode::Match, 2);
        player_count_of_room.insert(constants::Mode::Tag, 4);
        player_count_of_room
    };
}

fn can_join_room(_context: &mut Context, room: &Arc<Mutex<Room>>) -> bool {
    let room = room.lock();
    let current_count = room.players.len();
    let need_count = PLAYER_COUNT_OF_ROOM.get(&room.host_info.mode).unwrap();
    current_count < *need_count
}

fn remove_room_from_pool(room: &Arc<Mutex<Room>>) {
    let mut random_rooms = RANDOM_ROOMS.write();
    let actual_room = room.lock();
    let room_pool = random_rooms.get_mut(&actual_room.host_info.mode).unwrap();
    if let Some(index) = room_pool.iter().position(|pooled| Arc::ptr_eq(pooled, &room)) {
        room_pool.remove(index);
    }
}

fn register_handlers() {
    Handler::follow_message::<CTOSJoinGame, _>(9, "random_matcher_join_game", |context, request| Box::pin(async move { 
        let mut password = context.get_string(&request.pass, "pass")?.clone();
        if password == "" { password = "M".to_string(); }
        let mode = match password.as_str() {
            "S" => constants::Mode::Single,
            "M" => constants::Mode::Match,
            "T" => constants::Mode::Tag,
            _ => return Ok(false)
        };

        let room = {
            let mut random_rooms = RANDOM_ROOMS.write();
            let room_pool = random_rooms.get_mut(&mode).unwrap();
            let room_position = room_pool.iter().position(|room| can_join_room(context, room));
            if room_position.is_none() {
                let room_name = password + "#random_match_" + &chrono::offset::Local::now().timestamp_millis().to_string();
                let room = Room::find_or_create_by_name(&room_name).await.clone();
                room_pool.push(room.clone());
                room
            }
            else { room_pool.get(room_position.unwrap()).unwrap().clone() }
        };

        let socket = context.socket.take().unwrap();
        Room::join(room.clone(), &context.addr, socket).await;

        return Ok(true);
    })).register();

    Handler::new(9, "random_matcher_dropper_on_duel_start", |context| context.message_type == Some(MessageType::STOC(STOCMessageType::DuelStart)), |context| Box::pin(async move {
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

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}