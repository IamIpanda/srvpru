// ============================================================
// position_recorder
// ------------------------------------------------------------
//! Record which position is player in.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;

use crate::srvpru::Handler;
use crate::srvpru::Player;
use crate::srvpru::Room;

use crate::ygopro::Netplayer;
use crate::ygopro::message::stoc::TypeChange;

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}

player_attach! {
    position: Netplayer,
    is_host: bool
}

export_player_attach_as!(get_position, Netplayer, position_transformer);
export_player_attach_as!(is_host, bool, host_transformer);


pub fn register_handlers() {
    Handler::follow_message(100, "position_recorder", |context, request: &TypeChange| Box::pin(async move {
        let mut attachment = get_player_attachment_sure(context);
        if let Ok(position) = Netplayer::try_from(request._type & 0xf) {
            attachment.position = position;
        }
        attachment.is_host = request._type & 0xf0 > 0;
        Ok(false)
    })).register();

    register_player_attachment_dropper();
    Handler::register_handlers("position_recorder", crate::ygopro::message::Direction::STOC, vec!["position_recorder"]);
}

impl Room {
    #[allow(dead_code)]
    pub fn get_players_in_order(&self) -> Vec<Arc<Mutex<Player>>> {
        let mut players: Vec<Arc<Mutex<Player>>> = self.players.iter().map(|player| player.clone()).collect();
        players.sort_by(|player_a, player_b| 
            player_a.lock().get_position().partial_cmp(&player_b.lock().get_position()).unwrap()
        );
        players
    }

    #[allow(dead_code)]
    pub fn get_players_in_hashmap(&self) -> HashMap<Netplayer, Arc<Mutex<Player>>> {
        self.players.iter()
            .map(|player| (player.lock().get_position(), player.clone()))
            .collect()
    }

    pub fn get_host(&self) -> Option<Arc<Mutex<Player>>> {
        self.players.iter().find(|player| player.lock().is_host()).map(|player| player.clone())
    }

    pub fn is_full(&self) -> bool {
        let positions: HashMap<Netplayer, bool> = self.players.iter().map(|player| (player.lock().get_position(), true)).collect();
        match self.host_info.mode {
            crate::ygopro::Mode::Single | crate::ygopro::Mode::Match => 
                   positions.contains_key(&Netplayer::Player1) 
                && positions.contains_key(&Netplayer::Player2),
            crate::ygopro::Mode::Tag => 
                   positions.contains_key(&Netplayer::Player1) 
                && positions.contains_key(&Netplayer::Player2) 
                && positions.contains_key(&Netplayer::Player3) 
                && positions.contains_key(&Netplayer::Player4),
        }
    }
}

fn position_transformer<'b> (attachment: Option<parking_lot::MappedRwLockWriteGuard<'b, PlayerAttachment>>) -> Netplayer {
    attachment.map(|attach| attach.position).unwrap_or(Netplayer::Observer)
}

fn host_transformer<'b> (attachment: Option<parking_lot::MappedRwLockWriteGuard<'b, PlayerAttachment>>) -> bool {
    attachment.map(|attach| attach.is_host).unwrap_or(false)
}
