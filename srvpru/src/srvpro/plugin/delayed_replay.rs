use std::sync::Arc;

use parking_lot::Mutex;
use tokio::io::AsyncWriteExt;
use ygopro::constants::Colors;
use ygopro::message::server_to_client::Replay;
use ygopro::message::server_to_client::DuelEnd;

use crate::srvpro::Player;
use crate::srvpro::Response;
use crate::srvpro::Room;


#[derive(Default, PlayerAttachment)]
struct DelayedReplayPlayerAttachment {
    replays: Vec<Vec<u8>>
}

static DELAYED_REPLAY_PLAYER_ATTACHMENT: once_cell::sync::Lazy<parking_lot::RwLock<std::collections::HashMap<std::net::SocketAddr, Arc<Mutex<DelayedReplayPlayerAttachment>>>>> = once_cell::sync::Lazy::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

/*
impl crate::srvpro::Attachment for DelayedReplayPlayerAttachment {
    fn get_static_hash() -> &'static once_cell::sync::Lazy<parking_lot::RwLock<std::collections::HashMap<std::net::SocketAddr, Arc<Mutex<Self>>>>> {
        return &DELAYED_REPLAY_PLAYER_ATTACHMENT;
    }
}
impl<S> GetSocket<S> for DelayedReplayPlayerAttachment {
    fn get_socket(rqeuest: &mut crate::srvpro::Bundle<S>) -> std::net::SocketAddr {
        todo!()
    }
}

#[before(Replay)]
fn block_replay(data: Vec<u8>, attachment: Arc<Mutex<DelayedReplayPlayerAttachment>>, room: Arc<Mutex<Room>>, response: &mut Response) {
    attachment.lock().replays.push(data);
    // response.block_message = true
}

#[before(DuelEnd)]
async fn send_replay(attachment: Arc<Mutex<DelayedReplayPlayerAttachment>>, player: Arc<Mutex<Player>>) {
    let attachment = attachment.lock();
    let mut player = player.lock();
    for (index, data) in attachment.replays.iter().enumerate() {
        player.send_chat_to_client(Colors::Babyblue, format!("正在发送第 {} 场录像", index)).await.ok();
        player.client_stream_writer.write_all(&data).await.ok();
    };
}
*/
