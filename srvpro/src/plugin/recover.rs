use std::ops::Deref;
use std::sync::Arc;
use std::sync::LazyLock;

use arc_swap::ArcSwap;
use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::oneshot;

use ygopro::host::DuelHost;
use ygopro_core_wrapper::DuelSeed;
use ygopro_data::complex::Complex;
use ygopro_data::constants::CorePlayer;
use ygopro_data::constants::Hand;
use ygopro_data::constants::Netplayer;
use ygopro_data::data::Deck;
use ygopro_data::data::Replay;
use ygopro_data::message::ctos;
use ygopro_data::message::gm;
use ygopro_data::message::gm::GameMessage;
use ygopro_data::message::stoc;
use ygopro_data::string::FixedLengthString;
use ygopro_derive::Configuration;
use ygopro_handler::RoomProvider;
use ygopro_handler::StopFlag;

use crate::message as srvpro;
use crate::plugin::base::stage::Stage;
use crate::plugin::pass_command::PassCommandArguments;
use crate::plugin::pass_command::PassCommandHandler;
use crate::plugin::pass_command::PASS_COMMANDS;
use crate::plugin::register_dependencies;
use crate::plugin::virtual_password::OriginName;
use crate::mode::*;
use crate::room::*;

pub static NAME: &'static str = module_path!();

register_dependencies!(crate::plugin::reconnect::NAME);

#[async_trait]
pub trait Recover: Send + Sync {
    async fn list(&self, key: &str) -> Vec<String>;
    async fn recover(&self, id: &str) -> Option<Replay>;
}

#[distributed_slice]
pub static RECOVER_PROVIDERS: [(&'static str, fn() -> Box<dyn Recover>)];

static RECOVER_PROVIDER: LazyLock<ArcSwap<Option<Box<dyn Recover>>>> = LazyLock::new(|| {
    let provider = crate::configuration::get_configuration::<Configuration>().map(|configuration| configuration.provider);
    ArcSwap::from_pointee(RECOVER_PROVIDERS.iter().find(|(name, _)| provider.as_deref() == Some(*name)).map(|(_, make)| make()))
});

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    pub provider: String,
}

#[handler(srvpro::ConfigurationChanged)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as crate::GlobalHandler)]
fn on_configuration_changed() {
    let provider = crate::configuration::get_configuration::<Configuration>().map(|configuration| configuration.provider);
    RECOVER_PROVIDER.store(Arc::new(RECOVER_PROVIDERS.iter().find(|(name, _)| provider.as_deref() == Some(*name)).map(|(_, make)| make())));
}

struct RecoverRequest {
    id: String,
    from_turn: u32,
}

struct Recovered;

enum RecoverResult {
    NotRecover,
    Ready,
    Failed(&'static str),
}

struct RecoverCommand {
    answer_sender: Option<oneshot::Sender<RecoverResult>>,
}

impl RecoverCommand {
    fn answer(&mut self, result: RecoverResult) {
        self.answer_sender.take().and_then(|sender| sender.send(result).ok());
    }
}

impl<Res> ygopro_handler::FromRequest<ygopro_handler::extract::Request<Box<dyn std::any::Any + Send>, ()>, State, Res> for &mut RecoverCommand where Res: Send {
    fn from_request(bundle: &mut ygopro_handler::Bundle<ygopro_handler::extract::Request<Box<dyn std::any::Any + Send>, ()>, State, Res>) -> Option<Self> {
        bundle.request.message.downcast_mut::<RecoverCommand>().map(|command| unsafe { &mut *(command as *mut RecoverCommand) })
    }
}

struct VirtualPlayer {
    player: Player,
    server_to_client_stream: <DuelHost as RoomProvider<Complex<ctos::Message>, Complex<stoc::Message>>>::ServerToClientStream,
}

#[handler(Normal)]
#[register_to(MODES as ModeHandler)]
fn on_mode(config: &mut RoomConfiguration, part: &str) -> bool {
    if RECOVER_PROVIDER.load().is_none() { return false }
    if part != "RC" && part != "RECOVER" { return false }
    let Some((_, id)) = config.origin_name.split_once('#') else { return false };
    config.states.insert(RecoverRequest { id: id.to_string(), from_turn: 9999 });
    true
}

#[handler(Normal)]
#[register_to(MODES as ModeHandler)]
fn on_turn(config: &mut RoomConfiguration, part: &str) -> bool {
    let Some(turn) = part.strip_prefix('T').and_then(|turn| turn.parse::<u32>().ok()) else { return false };
    let Some(request) = config.states.get_mut::<RecoverRequest>() else { return false };
    request.from_turn = turn;
    true
}

#[command(name = "RC")]
#[register_to(PASS_COMMANDS as PassCommandHandler with &'static str)]
async fn replay_list(player: &mut Player, _arguments: &mut PassCommandArguments) {
    let key = player.states.get::<OriginName>().map(|origin_name| origin_name.0.clone()).unwrap_or_default();
    player.send_message("以下是您近期进行的决斗，密码处输入 RC,T回合数#决斗编号 即可创建复盘房间", ygopro_data::constants::Color::Babyblue);
    for (index, line) in list(&key).await.iter().enumerate() {
        player.send_message(&format!("<{}> {}", index + 1, line), ygopro_data::constants::Color::Babyblue);
    }
}

#[after(ctos::JoinGame)]
#[register_to(CTOS_PREHANDLERS as ClientToServerPrecursorHandler)]
async fn on_join_game(player: &mut Player, join_game: &ctos::JoinGame, stop: &mut StopFlag) -> &'static str {
    // On Handler(join_game) preprocessor, room is created, mode processor worked and put the flag.
    // But now we're after(join_game), we can't see anything inner player, we can only see there is a Player Move.
    // We can't put it to later time, If player move happened, all is late.
    // So we must do a query to judge if it is a RC room.
    let Some(room_name) = player.states.get::<srvpro::PlayerMove>().map(|player_move| player_move.room_name.clone()) else { return "continue" };
    let Some(room_sender) = ROOMS.read().get(&room_name).map(|room| room.sender.clone()) else { return "continue" };
    let (flag_sender, flag_receiver) = oneshot::channel();
    room_sender.unbounded_send(Request::Query(Box::new(move |_, states| {
        flag_sender.send(states.get::<RecoverRequest>().is_some() || states.get::<Recovered>().is_some()).ok();
    }))).ok();
    if !flag_receiver.await.unwrap_or(false) { return "continue" }
    
    let (answer_sender, answer_receiver) = oneshot::channel();
    room_sender.unbounded_send(Request::Command("recover_room", Box::new(RecoverCommand { answer_sender: Some(answer_sender) }))).ok();
    match answer_receiver.await {
        Ok(RecoverResult::NotRecover) => "continue",
        Ok(RecoverResult::Ready) => {
            let result = crate::plugin::reconnect::before_join_game(player, join_game, stop).await;
            if result == "cancel" { player.states.remove::<srvpro::PlayerMove>(); }
            result
        },
        Ok(RecoverResult::Failed(message)) => {
            player.send_message(message, ygopro_data::constants::Color::Red);
            "kick"
        },
        Err(error) => {
            log::warn!("cannot receive recover result for room {room_name}: {error}");
            "kick"
        },
    }
}

#[command]
#[register_to(SRVPRO_COMMANDS as CommandHandler with &'static str)]
async fn recover_room(room: &mut Room, states: &mut Anymap, command: &mut RecoverCommand) -> &'static str {
    // This is a room already recovered, but some one try to really reconnect in.
    if states.get::<Recovered>().is_some() {
        command.answer(RecoverResult::Ready);
        return "continue"
    }
    let Some(RecoverRequest { id, from_turn }) = states.remove::<RecoverRequest>() else {
        log::warn!("recover command was sent to non-recover room {}", room.name);
        command.answer(RecoverResult::NotRecover);
        return "continue"
    };
    let Some(replay) = recover(&id).await else {
        log::info!("recover replay {id} was not found");
        command.answer(RecoverResult::Failed("没有找到需要恢复的对局记录。"));
        return "terminate"
    };
    if replay.is_tag() {
        log::info!("recover replay {id} is a tag duel, which is not supported");
        command.answer(RecoverResult::Failed("暂不支持恢复双打对局。"));
        return "terminate"
    }
    let Some(configuration) = states.remove::<RoomProviderConfiguration>() else {
        log::warn!("recover room {} has no provider configuration", room.name);
        command.answer(RecoverResult::Failed("恢复对局失败：房间配置不存在。"));
        return "terminate"
    };
    let mut provider = create_provider(configuration, &replay);
    let Ok((mut host, mut client)) = start_duel(&replay, &mut provider).await else {
        command.answer(RecoverResult::Failed("恢复对局失败：无法启动对局。"));
        return "terminate"
    };
    if drive_replay(&mut host, &mut client, &replay, from_turn).await.is_err() {
        command.answer(RecoverResult::Failed("恢复对局失败：录像与对局状态不一致。"));
        return "terminate"
    }
    let host_name = replay.body.host_name.to_string();
    let client_name = replay.body.client_name.to_string();
    let host_position = install(room, host, host_name.clone(), replay.body.host_deck.clone().into(), Netplayer::Player(0), true);
    let client_position = install(room, client, client_name.clone(), replay.body.client_deck.clone().into(), Netplayer::Player(1), false);
    let mut slots = hashbrown::HashMap::new();
    for (position, name) in [(host_position, host_name), (client_position, client_name)] {
        let timeout = tokio::spawn(std::future::pending());
        slots.insert(position, crate::plugin::reconnect::DisconnectInfo { name, timeout });
    }
    let finish_signal = <DuelHost as RoomProvider<Complex<ctos::Message>, Complex<stoc::Message>>>::get_finish_signal(&mut provider);
    let room_sender = room.request_sender.clone();
    room.provider = Some(Provider::Embedded(provider));
    room.request_sender.unbounded_send(Request::Ex(srvpro::CreateProvider.into(), 0)).ok();
    states.insert(replay.host_info());
    states.insert(ProviderType::Embedded);
    states.insert(Stage { stage: ygopro_data::constants::DuelStage::Dueling });
    states.insert(crate::plugin::reconnect::Disconnected { slots });
    states.insert(Recovered);
    tokio::spawn(async move {
        finish_signal.await;
        room_sender.unbounded_send(Request::Ex(srvpro::ProviderTerminate.into(), 0)).ok();
    });
    command.answer(RecoverResult::Ready);
    "continue"
}

fn create_provider(mut configuration: RoomProviderConfiguration, replay: &Replay) -> DuelHost {
    let seed_sequence = replay.header.seed_sequence;
    configuration.ygopru_configuration.seed_generator = Some(Box::new(move |duel_count| if duel_count == 0 { DuelSeed::Complicated(seed_sequence) } else { DuelSeed::None }));
    configuration.ygopru_configuration.enable_plugin(ygopro::plugin::no_init_shuffle_deck::NAME);
    DuelHost::new(replay.host_info(), configuration.ygopru_configuration)
}

fn create_virtual_player(provider: &mut DuelHost) -> VirtualPlayer {
    let mut player = Player::allocate();
    let client_to_server_stream = player.client_to_server_stream_towards_provider.take().unwrap();
    let server_to_client_stream = provider.add(client_to_server_stream);
    VirtualPlayer { player, server_to_client_stream }
}

async fn start_duel(replay: &Replay, provider: &mut DuelHost) -> Result<(VirtualPlayer, VirtualPlayer), ()> {
    let mut host = create_virtual_player(provider);
    send(&host, ctos::PlayerInfo { name: replay.body.host_name.clone() }.into());
    send(&host, ctos::JoinGame { version: *ygopro::PRO_VERSION, gameid: 0, pass: FixedLengthString::allocate() }.into());
    wait_for(&mut host, stoc::MessageType::TypeChange).await?;
    let mut client = create_virtual_player(provider);
    send(&client, ctos::PlayerInfo { name: replay.body.client_name.clone() }.into());
    send(&client, ctos::JoinGame { version: *ygopro::PRO_VERSION, gameid: 0, pass: FixedLengthString::allocate() }.into());
    wait_for(&mut client, stoc::MessageType::TypeChange).await?;
    wait_for(&mut host, stoc::MessageType::HsPlayerEnter).await?;
    send(&host, ctos::UpdateDeck { deck: replay.body.host_deck.clone().into() }.into());
    send(&host, ctos::HsReady.into());
    send(&client, ctos::UpdateDeck { deck: replay.body.client_deck.clone().into() }.into());
    send(&client, ctos::HsReady.into());
    wait_for(&mut client, stoc::MessageType::HsPlayerChange).await?;
    send(&host, ctos::HsStart.into());
    wait_for(&mut host, stoc::MessageType::SelectHand).await?;
    send(&host, ctos::HandResult { res: Hand::Paper }.into());
    send(&client, ctos::HandResult { res: Hand::Rock }.into());
    wait_for(&mut host, stoc::MessageType::SelectTp).await?;
    send(&host, ctos::TpResult { result: CorePlayer::FirstAttackPlayer }.into());
    Ok((host, client))
}

async fn drive_replay(host: &mut VirtualPlayer, client: &mut VirtualPlayer, replay: &Replay, from_turn: u32) -> Result<(), ()> {
    if from_turn == 0 { return Ok(()) }
    let mut responses = replay.body.datas.iter().map(|data| data.data.clone());
    let mut turn = 0;
    loop {
        tokio::select! {
            message = host.server_to_client_stream.next() => {
                let message = message.ok_or(())?;
                if target_turn(&message, &mut turn, from_turn) { return Ok(()) }
                if respond(host, CorePlayer::FirstAttackPlayer, &message, &mut responses)? { return Ok(()) }
            },
            message = client.server_to_client_stream.next() => {
                let message = message.ok_or(())?;
                if respond(client, CorePlayer::SecondAttackPlayer, &message, &mut responses)? { return Ok(()) }
            },
        }
    }
}

fn target_turn(message: &Complex<stoc::Message>, turn: &mut u32, from_turn: u32) -> bool {
    if !matches!(message.deref(), stoc::Message::GameMessage(game_message) if matches!(game_message.message, gm::Message::NewTurn(_))) { return false }
    *turn += 1;
    *turn >= from_turn
}

fn respond(player: &VirtualPlayer, core_player: CorePlayer, message: &Complex<stoc::Message>, responses: &mut impl Iterator<Item = ygopro_data::data::Response>) -> Result<bool, ()> {
    match message.deref() {
        stoc::Message::TimeLimit(limit) if limit.player == core_player => send(player, ctos::TimeConfirm.into()),
        stoc::Message::GameMessage(game_message) if matches!(game_message.message, gm::Message::Retry(_)) => return Err(()),
        stoc::Message::GameMessage(game_message) if game_message.message.waiting_for().is_some() => {
            let Some(response) = responses.next() else { return Ok(true) };
            send(player, ctos::Response { response }.into());
        },
        _ => (),
    }
    Ok(false)
}

async fn wait_for(player: &mut VirtualPlayer, message_type: stoc::MessageType) -> Result<(), ()> {
    while let Some(message) = player.server_to_client_stream.next().await {
        if stoc::MessageType::from(message.deref()) == message_type { return Ok(()) }
    }
    Err(())
}

fn send(player: &VirtualPlayer, message: ctos::Message) {
    player.player.client_to_server_sink_towards_provider.unbounded_send(Complex::from_message(message)).ok();
}

fn install(room: &mut Room, mut virtual_player: VirtualPlayer, name: String, deck: Deck, netplayer: Netplayer, host: bool) -> usize {
    virtual_player.player.states.insert(name.clone());
    virtual_player.player.states.insert(OriginName(name));
    virtual_player.player.states.insert(vec![deck]);
    virtual_player.player.states.insert(netplayer);
    virtual_player.player.states.insert(host);
    let position = room.players.insert(virtual_player.player);
    let sender = room.request_sender.clone();
    tokio::spawn(async move {
        while let Some(message) = virtual_player.server_to_client_stream.next().await {
            sender.unbounded_send(Request::STOC(message, position)).ok();
        }
    });
    position
}

async fn recover(id: &str) -> Option<Replay> {
    let selected = RECOVER_PROVIDER.load_full();
    let provider = selected.as_ref().as_ref()?;
    provider.recover(id).await
}

async fn list(key: &str) -> Vec<String> {
    let selected = RECOVER_PROVIDER.load_full();
    let Some(provider) = selected.as_ref().as_ref() else { return Vec::new() };
    provider.list(key).await
}

/// Save replays to the local file system, and recover/list from local.
/// Please note that in this mode, no api is involved. If you want to save meta, please check
/// srvpro-api's [`duel_log`](srvpro_api::plugins::duel_log).
mod file {
    use std::io::Cursor;
    use std::path::Path;

    use async_trait::async_trait;
    use binrw::BinRead;
    use glob::Pattern;

    use ygopro_data::data::Replay;
    use ygopro_data::data::ReplayMode;
    use ygopro_data::message::gm;
    use ygopro_derive::Configuration;
    use ygopro_derive::handler;
    use ygopro_derive::register_to;
    use ygopro_handler::All;

    use crate::mode::MODES;
    use crate::mode::ModeHandler as Handler;
    use crate::mode::RoomConfiguration;
    use crate::plugin::recover::RECOVER_PROVIDERS;
    use crate::plugin::recover::Recover;

    pub static NAME: &'static str = module_path!();

    #[distributed_slice(RECOVER_PROVIDERS)]
    static FILE: (&'static str, fn() -> Box<dyn Recover>) = ("file", || Box::new(File));
    
    #[derive(Clone, Configuration)]
    #[config(sync)]
    pub struct Configuration {
        #[config(default = "\"replays\".to_string()")]
        pub path: String,
        pub machine: u16,
        #[config(default = "10")]
        pub list_limit: usize,
    }

    struct File;

    #[handler(All)]
    #[register_to(MODES)]
    fn on_configure(config: &mut RoomConfiguration) {
        let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return };
        config.ygopru_configuration.enable_plugin_with_inited::<ygopro::plugin::replay::Configuration>(ygopro::plugin::replay::NAME, |replay| {
            let replay = replay.into_mut();
            replay.mode |= ReplayMode::SaveInServer;
            replay.save_path = configuration.path;
            replay.file_template = "{id} {time} {players} Duel {duel}".to_string();
            replay.machine = configuration.machine;
            replay.format = ygopro::plugin::replay::Format::YgoproForge;
        });
    }

    #[async_trait]
    impl Recover for File {
        async fn list(&self, key: &str) -> Vec<String> {
            let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else {
                log::warn!("recover file provider has no configuration.");
                return Vec::new()
            };
            list_replays(&configuration.path, key, configuration.list_limit).await
        }

        async fn recover(&self, id: &str) -> Option<Replay> {
            let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else {
                log::warn!("recover file provider has no configuration.");
                return None
            };
            let id = Pattern::escape(id);
            let pattern = format!("{}/{id} *.yrp*", configuration.path);
            // glob::glob walks the directory with blocking syscalls; keep it off the runtime workers.
            let path = tokio::task::spawn_blocking(move || glob::glob(&pattern).ok()?.flatten().next()).await.ok().flatten()?;
            if path.extension().and_then(|extension| extension.to_str()) == Some("yrp") { read_replay(&path).await } else { read_forge_replay(&path).await }
        }
    }


    async fn list_replays(directory: &str, key: &str, limit: usize) -> Vec<String> {
        let key = Pattern::escape(key);
        let patterns = [format!("{directory}/* {key} VS *.yrp*"), format!("{directory}/* VS {key} *.yrp*")];
        let mut paths = Vec::new();
        for pattern in patterns {
            // glob::glob walks the directory with blocking syscalls; keep it off the runtime workers.
            let found = tokio::task::spawn_blocking(move || glob::glob(&pattern).map(|paths| paths.flatten().collect::<Vec<_>>()).unwrap_or_default()).await.unwrap_or_default();
            paths.extend(found);
        }
        paths.sort();
        paths.dedup();
        paths.reverse();
        paths.truncate(limit);
        paths.into_iter().filter_map(|path| id_of(&path)).collect()
    }

    fn id_of(path: &Path) -> Option<String> {
        let stem = path.file_stem()?.to_str()?;
        Some(stem.split_once(' ').map_or(stem, |(id, _)| id).to_string())
    }

    async fn read_replay(path: &Path) -> Option<Replay> {
        let bytes = match tokio::fs::read(path).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
            Err(error) => {
                log::warn!("cannot read replay {}: {error}", path.display());
                return None
            },
        };
        match Replay::read_le(&mut Cursor::new(bytes)) {
            Ok(replay) => Some(replay),
            Err(error) => {
                log::warn!("cannot parse replay {}: {error}", path.display());
                None
            },
        }
    }

    async fn read_forge_replay(path: &Path) -> Option<Replay> {
        let bytes = match tokio::fs::read(path).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
            Err(error) => {
                log::warn!("cannot read forge replay {}: {error}", path.display());
                return None
            },
        };
        let replay = match ygopro_data::data::forge::Replay::read_le(&mut Cursor::new(bytes)) {
            Ok(replay) => replay,
            Err(error) => {
                log::warn!("cannot parse forge replay {}: {error}", path.display());
                return None
            },
        };
        replay.messages.into_iter().find_map(|message| match message {
            gm::Message::SibylReplay(message) => Some(*message.replay),
            _ => None,
        })
    }
}

mod api {
    use std::io::Cursor;
    use std::time::Duration;

    use async_trait::async_trait;
    use binrw::BinRead;

    use ygopro_data::data::Replay;
    use ygopro_data::message::gm;
    use ygopro_derive::Configuration;

    use crate::plugin::recover::RECOVER_PROVIDERS;
    use crate::plugin::recover::Recover;

    pub static NAME: &'static str = module_path!();

    #[derive(Clone, Configuration)]
    #[config(sync)]
    pub struct Configuration {
        pub list_url: String,
        pub replay_url: String,
        pub auth: String,
    }

    struct Api;

    #[async_trait]
    impl Recover for Api {
        async fn list(&self, key: &str) -> Vec<String> {
            let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else {
                log::warn!("recover api provider has no configuration.");
                return Vec::new()
            };
            let Some(bytes) = fetch(&configuration.list_url.replace("{id}", key), &configuration.auth).await else { return Vec::new() };
            serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                log::warn!("cannot parse recover list: {error}");
                Vec::new()
            })
        }

        async fn recover(&self, id: &str) -> Option<Replay> {
            let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else {
                log::warn!("recover api provider has no configuration.");
                return None
            };
            let bytes = fetch(&configuration.replay_url.replace("{id}", id), &configuration.auth).await?;
            parse(&bytes)
        }
    }

    #[distributed_slice(RECOVER_PROVIDERS)]
    static API: (&'static str, fn() -> Box<dyn Recover>) = ("api", || Box::new(Api));

    async fn fetch(url: &str, auth: &str) -> Option<Vec<u8>> {
        let url = url.to_string();
        let auth = auth.to_string();
        tokio::task::spawn_blocking(move || fetch_blocking(&url, &auth)).await.ok().flatten()
    }

    fn fetch_blocking(url: &str, auth: &str) -> Option<Vec<u8>> {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).build().into();
        let mut request = agent.get(url);
        if !auth.is_empty() {
            request = request.header("Authorization", auth);
        }
        let mut response = request.call().map_err(|error| log::warn!("cannot fetch replay {url}: {error}")).ok()?;
        response.body_mut().with_config().limit(64 * 1024 * 1024).read_to_vec().map_err(|error| log::warn!("cannot read replay {url}: {error}")).ok()
    }

    fn parse(bytes: &[u8]) -> Option<Replay> {
        if bytes.starts_with(b"yrp1") || bytes.starts_with(b"yrp2") {
            return match Replay::read_le(&mut Cursor::new(bytes)) {
                Ok(replay) => Some(replay),
                Err(error) => {
                    log::warn!("cannot parse recovered yrp replay: {error}");
                    None
                },
            }
        }
        let replay = match ygopro_data::data::forge::Replay::read_le(&mut Cursor::new(bytes)) {
            Ok(replay) => replay,
            Err(error) => {
                log::warn!("cannot parse recovered forge replay: {error}");
                return None
            },
        };
        replay.messages.into_iter().find_map(|message| match message {
            gm::Message::SibylReplay(message) => Some(*message.replay),
            _ => None,
        })
    }
}
