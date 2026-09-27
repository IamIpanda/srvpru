use std::sync::Arc;
use std::sync::LazyLock;

use arc_swap::ArcSwap;
use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;

use ygopro_data::complex::Complex;
use ygopro_data::constants::Color;
use ygopro_data::constants::ErrorMessage;
use ygopro_data::constants::JoinError;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Configuration;
use ygopro_derive::before;
use ygopro_derive::handler;
use ygopro_derive::register_to;
use ygopro_handler::StopFlag;

use crate::message as srvpro;
use crate::mode::MODES;
use crate::mode::ModeHandler as Handler;
use crate::mode::Normal;
use crate::mode::RoomConfiguration;
use crate::room::Anymap;
use crate::room::CTOS_HANDLERS;
use crate::room::ClientToServerHandler;
use crate::room::Room;

pub static NAME: &'static str = module_path!();

pub type Replay = BoxStream<'static, Complex<stoc::Message>>;

#[async_trait]
pub trait CloudReplay: Send + Sync {
    async fn open(&self, id: &str) -> Option<Replay>;
}

#[distributed_slice]
pub static CLOUD_REPLAY_PROVIDERS: [(&'static str, fn() -> Box<dyn CloudReplay>)];

static CLOUD_REPLAY_PROVIDER: LazyLock<ArcSwap<Option<Box<dyn CloudReplay>>>> = LazyLock::new(|| {
    let provider = crate::configuration::get_configuration::<Configuration>().map(|configuration| configuration.provider);
    ArcSwap::from_pointee(CLOUD_REPLAY_PROVIDERS.iter().find(|(name, _)| provider.as_deref() == Some(*name)).map(|(_, make)| make()))
});

struct CloudReplayId(String);

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    pub provider: String,
}

#[handler(srvpro::ConfigurationChanged)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as crate::GlobalHandler)]
fn on_configuration_changed() {
    let provider = crate::configuration::get_configuration::<Configuration>().map(|configuration| configuration.provider);
    CLOUD_REPLAY_PROVIDER.store(Arc::new(CLOUD_REPLAY_PROVIDERS.iter().find(|(name, _)| provider.as_deref() == Some(*name)).map(|(_, make)| make())));
}

#[handler(Normal)]
#[register_to(MODES)]
fn on_mode(config: &mut RoomConfiguration, part: &str) -> bool {
    if part != "R" || CLOUD_REPLAY_PROVIDER.load().is_none() { return false }
    let Some((_, id)) = config.origin_name.split_once('#') else { return false };
    config.states.insert(CloudReplayId(id.to_string()));
    true
}

#[before(ctos::JoinGame, priority = 16)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn on_join_game(room: &mut Room, states: &mut Anymap, index: usize, stop: &mut StopFlag) -> bool {
    let Some(CloudReplayId(id)) = states.remove::<CloudReplayId>() else { return false };
    stop.0 = true;
    let server_to_client_sink = room.players[index].server_to_client_sink_towards_network.clone();
    tokio::spawn(async move {
        if let Some(mut replay) = open(&id).await {
            while let Some(message) = replay.next().await {
                server_to_client_sink.unbounded_send(message).ok();
            }
        } else {
            server_to_client_sink.unbounded_send(Complex::from_message(stoc::Chat {
                player: Color::Red.into(),
                msg: "[Server]: 没有找到录像".into(),
            }.into())).ok();
            server_to_client_sink.unbounded_send(Complex::from_message(stoc::ErrorMessage {
                err: ErrorMessage::JoinError(JoinError::HostRefused),
            }.into())).ok();
        }
    });
    true
}

async fn open(id: &str) -> Option<Replay> {
    let selected = CLOUD_REPLAY_PROVIDER.load_full();
    let provider = selected.as_ref().as_ref()?;
    provider.open(id).await
}

mod file {
    use std::io::Cursor;
    use std::path::Path;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    use async_trait::async_trait;
    use binrw::BinRead;
    use futures::StreamExt;
    use futures::stream;

    use ygopro_data::complex::Complex;
    use ygopro_data::constants::Netplayer;
    use ygopro_data::data::ReplayMode;
    use ygopro_data::message::forge_to_stoc;
    use ygopro_data::message::gm;
    use ygopro_data::message::gm::GameMessage;
    use ygopro_data::message::stoc;
    use ygopro_derive::Configuration;
    use ygopro_derive::handler;
    use ygopro_derive::register_to;
    use ygopro_handler::All;

    use crate::mode::MODES;
    use crate::mode::ModeHandler as Handler;
    use crate::mode::RoomConfiguration;
    use crate::plugin::cloud_replay::CLOUD_REPLAY_PROVIDERS;
    use crate::plugin::cloud_replay::CloudReplay;
    use crate::plugin::cloud_replay::Replay;

    const EPOCH: u64 = 1_600_000_000_000;
    const MACHINE_BITS: u64 = 10;
    const SEQUENCE_BITS: u64 = 12;
    const MACHINE_MASK: u64 = (1 << MACHINE_BITS) - 1;
    const SEQUENCE_MASK: u64 = (1 << SEQUENCE_BITS) - 1;

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    pub static NAME: &'static str = module_path!();

    #[derive(Clone, Configuration)]
    #[config(sync)]
    pub struct Configuration {
        #[config(default = "\"replays\".to_string()")]
        pub path: String,
        pub machine: u16,
    }

    struct File;

    #[async_trait]
    impl CloudReplay for File {
        async fn open(&self, id: &str) -> Option<Replay> {
            let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else {
                log::warn!("cloud replay file provider has no configuration.");
                return None
            };
            let path = Path::new(&configuration.path).join(id).with_extension("yrp3d");
            let bytes = match tokio::fs::read(&path).await {
                Ok(bytes) => bytes,
                Err(error) => {
                    log::warn!("cannot read cloud replay {}: {error}", path.display());
                    return None
                }
            };
            let replay = match ygopro_data::data::forge::Replay::read_le(&mut Cursor::new(bytes)) {
                Ok(replay) => replay,
                Err(error) => {
                    log::warn!("cannot parse cloud replay {}: {error}", path.display());
                    return None
                }
            };
            let records: Vec<gm::Message> = replay.messages.into_iter().filter(|message| message.waiting_for().is_none()).collect();
            let mut messages = forge_to_stoc(&records);
            messages.insert(1, stoc::Message::from(stoc::TypeChange { player: Netplayer::Observer(255), host: false }));
            Some(stream::iter(messages).map(Complex::from_message).boxed())
        }
    }

    #[distributed_slice(CLOUD_REPLAY_PROVIDERS)]
    static FILE: (&'static str, fn() -> Box<dyn CloudReplay>) = ("file", || Box::new(File));

    #[handler(All)]
    #[register_to(MODES)]
    fn on_configure(config: &mut RoomConfiguration) {
        let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else { return };
        config.ygopru_configuration.enable_plugin_with_configuration(ygopro::plugin::replay::NAME, ygopro::plugin::replay::Configuration {
            mode: ReplayMode::SaveInServer,
            save_path: configuration.path,
            file_template: replay_id(configuration.machine),
            format: ygopro::plugin::replay::Format::YgoproForge,
        });
    }

    fn replay_id(machine: u16) -> String {
        let mut id = snowflake_id(machine);
        let mut characters = Vec::new();
        loop {
            characters.push(char::from_digit((id % 36) as u32, 36).unwrap_or('0'));
            id /= 36;
            if id == 0 { break }
        }
        characters.into_iter().rev().collect()
    }

    fn snowflake_id(machine: u16) -> u64 {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_millis() as u64).unwrap_or(0).saturating_sub(EPOCH);
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed) & SEQUENCE_MASK;
        (timestamp << (MACHINE_BITS + SEQUENCE_BITS)) | ((machine as u64 & MACHINE_MASK) << SEQUENCE_BITS) | sequence
    }
}

mod api {
    use std::io::Cursor;
    use std::time::Duration;

    use async_trait::async_trait;
    use binrw::BinRead;
    use futures::StreamExt;
    use futures::stream;

    use ygopro_data::complex::Complex;
    use ygopro_data::constants::Netplayer;
    use ygopro_data::message::forge_to_stoc;
    use ygopro_data::message::gm;
    use ygopro_data::message::gm::GameMessage;
    use ygopro_data::message::stoc;
    use ygopro_derive::Configuration;

    use crate::plugin::cloud_replay::CLOUD_REPLAY_PROVIDERS;
    use crate::plugin::cloud_replay::CloudReplay;
    use crate::plugin::cloud_replay::Replay;

    pub static NAME: &'static str = module_path!();

    #[derive(Clone, Configuration)]
    #[config(sync)]
    pub struct Configuration {
        pub url: String,
        pub auth: String,
    }

    struct Api;

    #[async_trait]
    impl CloudReplay for Api {
        async fn open(&self, id: &str) -> Option<Replay> {
            let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else {
                log::warn!("cloud replay api provider has no configuration.");
                return None
            };
            let bytes = fetch(&configuration, id).await?;
            if bytes.starts_with(b"yrp1") || bytes.starts_with(b"yrp2") {
                log::warn!("cloud replay api provider got a yrp replay, which carries no game message.");
                return None
            }
            let replay = match ygopro_data::data::forge::Replay::read_le(&mut Cursor::new(bytes)) {
                Ok(replay) => replay,
                Err(error) => {
                    log::warn!("cannot parse cloud replay {id}: {error}");
                    return None
                },
            };
            let records: Vec<gm::Message> = replay.messages.into_iter().filter(|message| message.waiting_for().is_none()).collect();
            let mut messages = forge_to_stoc(&records);
            messages.insert(1, stoc::Message::from(stoc::TypeChange { player: Netplayer::Observer(255), host: false }));
            Some(stream::iter(messages).map(Complex::from_message).boxed())
        }
    }

    #[distributed_slice(CLOUD_REPLAY_PROVIDERS)]
    static API: (&'static str, fn() -> Box<dyn CloudReplay>) = ("api", || Box::new(Api));

    async fn fetch(configuration: &Configuration, id: &str) -> Option<Vec<u8>> {
        let url = configuration.url.replace("{id}", id);
        let auth = configuration.auth.clone();
        tokio::task::spawn_blocking(move || fetch_blocking(&url, &auth)).await.ok().flatten()
    }

    fn fetch_blocking(url: &str, auth: &str) -> Option<Vec<u8>> {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).build().into();
        let mut request = agent.get(url);
        if !auth.is_empty() {
            request = request.header("Authorization", auth);
        }
        let mut response = request.call().map_err(|error| log::warn!("cannot fetch cloud replay {url}: {error}")).ok()?;
        response.body_mut().with_config().limit(64 * 1024 * 1024).read_to_vec().map_err(|error| log::warn!("cannot read cloud replay {url}: {error}")).ok()
    }
}
