use log::debug;
use futures::StreamExt;

use ygopro_data::constants::Color;
use ygopro_data::message::ctos;
use ygopro_handler::extract::Response;
use ygopro_handler::RoomProvider;

use crate::message as srvpro;
use crate::mode::RoomProviderConfiguration;
use crate::room::*;

type Anymap = anymap3::Map<dyn std::any::Any + Send>;

#[handler(ctos::JoinGame, module = "srvpro::core")]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
async fn on_join_game_precursor(states: &mut Anymap, join_game: &ctos::JoinGame) {
    if states.get::<srvpro::PlayerMove>().is_some() { return; }
    let room_name = join_game.pass.to_string();
    let exists = ROOMS.read().contains_key(&room_name);
    if ! exists { 
        RoomHost::new(&room_name).await; 
        debug!("Room {} allocated.", room_name);
    }
    states.insert(srvpro::PlayerMove { room_name });
}

#[handler(srvpro::PlayerJoin, module = "srvpro::core")]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_player_join(room: &mut Room, player_join: &mut srvpro::PlayerJoin) {
    let Some(mut player) = player_join.player.take() else { return };
    let Some(position_sender) = player_join.position_sender.take() else { return };
    let Some(mut provider_stream) = player.client_to_server_stream_towards_provider.take() else { return };
    let index = room.players.insert(player);
    while let Ok(message) = provider_stream.try_recv() {
        room.request_sender.unbounded_send(Request::CTOS(message, index)).ok();
    }
    room.players[index].client_to_server_stream_towards_provider = Some(provider_stream);
    position_sender.send(index).ok();
}

#[handler(srvpro::ClientLeave, module = "srvpro::core")]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
fn on_client_leave(room: &mut Room, client_leave: &mut srvpro::ClientLeave) {
    room.players.try_remove(client_leave.position);
}

#[before(ctos::JoinGame, module = "srvpro::core")]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
async fn on_join_game_create(room: &mut Room, states: &mut Anymap) {
    if room.provider.is_some() { return }
    let Some(provider_configuration) = states.get_mut::<RoomProviderConfiguration>() else { return };
    let hostinfo = provider_configuration.hostinfo.clone();
    let provider_type = provider_configuration.provider;
    if let Ok(mut provider) = provider_configuration.create_room_provider(room.name.clone()).await {
        // we directly set hostinfo and provider here.
        // removing provider configuration is just for stopping fucking code agent (deepseek, I MEAN YOU!) extracting it.
        states.insert(hostinfo);
        states.insert(provider_type);
        states.remove::<RoomProviderConfiguration>();
        let room_sender = room.request_sender.clone();
        room_sender.unbounded_send(Request::Ex(srvpro::CreateProvider.into(), 0)).ok();
        let finish_signal = provider.get_finish_signal();
        room.provider = Some(provider);
        tokio::spawn(async move {
            finish_signal.await;
            room_sender.unbounded_send(Request::Ex(srvpro::ProviderTerminate.into(), 0)).ok();
        });
    } else {
        room.request_sender.unbounded_send(Request::Ex(srvpro::ProviderFail.into(), 0)).ok();
        return;
    }
}

#[handler(ctos::JoinGame, module = "srvpro::core")]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
async fn on_join_game(room: &mut Room, player: usize) {
    let request_sender = room.request_sender.clone();
    let Some(provider_stream) = room.players[player].client_to_server_stream_towards_provider.take() else { return };
    let Some(provider) = room.provider.as_mut() else { return };
    let mut server_to_client_stream = provider.add(provider_stream);
    tokio::spawn(async move {
        while let Some(message) = server_to_client_stream.next().await {
            request_sender.unbounded_send(Request::STOC(message, player)).ok();
        }
    });
}

#[handler(srvpro::ClientRefused, module = "srvpro::core")]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as crate::GlobalHandler)]
async fn on_client_refused() -> Response<srvpro::Message> {
    Response::Kick
}

#[handler(srvpro::ProviderTerminate, module = "srvpro::core")]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
async fn on_provider_terminate() -> &'static str {
    "terminate"
}

#[handler(srvpro::ProviderFail, module = "srvpro::core")]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
async fn on_provider_fail(room: &mut Room) -> &'static str {
    room.broadcast_message(Color::Red, "创建房间失败。");
    "terminate"
}

#[handler(srvpro::Terminate, module = "srvpro::core")]
#[register_to(SRVPRO_HANDLERS as SrvproMessageHandler)]
async fn on_terminate(room: &mut Room) {
    let mut rooms = ROOMS.write();
    rooms.remove(&room.name);
    log::debug!("Room {} terminated.", room.name)
}
