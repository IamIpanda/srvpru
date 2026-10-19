//! Cloud replay: watch a saved duel on the server without downloading the replay file.

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
use crate::plugin::pass_command::PassCommandArguments;
use crate::plugin::pass_command::PassCommandHandler;
use crate::plugin::pass_command::PASS_COMMANDS;
use crate::plugin::virtual_password::OriginName;
use crate::room::Anymap;
use crate::room::CTOS_HANDLERS;
use crate::room::ClientToServerHandler;
use crate::room::Player;
use crate::room::Room;

pub static NAME: &'static str = module_path!();

pub type Replay = BoxStream<'static, Complex<stoc::Message>>;

#[async_trait]
pub trait CloudReplay: Send + Sync {
    async fn list(&self, key: &str) -> Vec<String>;
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
    /// Name of the cloud replay provider to load.
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

#[command(name = "R")]
#[register_to(PASS_COMMANDS as PassCommandHandler with &'static str)]
async fn replay_list(player: &mut Player, _arguments: &mut PassCommandArguments) {
    let key = player.states.get::<OriginName>().map(|origin_name| origin_name.0.clone()).unwrap_or_default();
    player.send_message("以下是您近期的云录像，密码处输入 R#录像编号 即可观看", Color::Babyblue);
    for (index, line) in list(&key).await.iter().enumerate() {
        player.send_message(&format!("<{}> {}", index + 1, line), Color::Babyblue);
    }
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

async fn list(key: &str) -> Vec<String> {
    let selected = CLOUD_REPLAY_PROVIDER.load_full();
    let Some(provider) = selected.as_ref().as_ref() else { return Vec::new() };
    provider.list(key).await
}

/// Serve cloud replays stored on the local file system.
pub mod file {
    use std::io::Cursor;
    use std::path::Path;

    use async_trait::async_trait;
    use binrw::BinRead;
    use futures::StreamExt;
    use futures::stream;
    use glob::Pattern;

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

    /// Name for activitating this module in the plugin system.
    pub static NAME: &'static str = module_path!();

    #[derive(Clone, Configuration)]
    #[config(sync)]
    pub struct Configuration {
        /// Directory holding saved `.yrp3d` replays.
        #[config(default = "\"replays\".to_string()")]
        pub path: String,
        /// Machine id embedded into generated replay ids to keep them unique across hosts.
        pub machine: u16,
        /// Maximum number of replays returned by the listing.
        #[config(default = "10")]
        pub list_limit: usize,
    }

    struct File;

    #[async_trait]
    impl CloudReplay for File {
        async fn list(&self, key: &str) -> Vec<String> {
            let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else {
                log::warn!("cloud replay file provider has no configuration.");
                return Vec::new()
            };
            list_replays(&configuration.path, key, configuration.list_limit).await
        }

        async fn open(&self, id: &str) -> Option<Replay> {
            let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else {
                log::warn!("cloud replay file provider has no configuration.");
                return None
            };
            let id = Pattern::escape(id);
            let pattern = format!("{}/{id} *.yrp3d", configuration.path);
            // glob::glob walks the directory with blocking syscalls; keep it off the runtime workers.
            let path = tokio::task::spawn_blocking(move || glob::glob(&pattern).ok()?.flatten().next()).await.ok().flatten()?;
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

    async fn list_replays(directory: &str, key: &str, limit: usize) -> Vec<String> {
        let key = Pattern::escape(key);
        let patterns = [format!("{directory}/* {key} VS *.yrp3d"), format!("{directory}/* VS {key} *.yrp3d")];
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
}

/// Serve cloud replays fetched over HTTP.
pub mod api {
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

    /// Name for activitating this module in the plugin system.
    pub static NAME: &'static str = module_path!();

    #[derive(Clone, Configuration)]
    #[config(sync)]
    pub struct Configuration {
        /// Listing URL template; `{id}` is replaced with the player's key.
        pub list_url: String,
        /// Replay URL template; `{id}` is replaced with the requested replay id.
        pub replay_url: String,
        /// Authorization header sent with both requests; empty sends none.
        pub auth: String,
    }

    struct Api;

    #[async_trait]
    impl CloudReplay for Api {
        async fn list(&self, key: &str) -> Vec<String> {
            let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else {
                log::warn!("cloud replay api provider has no configuration.");
                return Vec::new()
            };
            let Some(bytes) = fetch(&configuration.list_url.replace("{id}", key), &configuration.auth).await else { return Vec::new() };
            serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                log::warn!("cannot parse cloud replay list: {error}");
                Vec::new()
            })
        }

        async fn open(&self, id: &str) -> Option<Replay> {
            let Some(configuration) = crate::configuration::get_configuration::<Configuration>() else {
                log::warn!("cloud replay api provider has no configuration.");
                return None
            };
            let bytes = fetch(&configuration.replay_url.replace("{id}", id), &configuration.auth).await?;
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
        let mut response = request.call().map_err(|error| log::warn!("cannot fetch cloud replay {url}: {error}")).ok()?;
        response.body_mut().with_config().limit(64 * 1024 * 1024).read_to_vec().map_err(|error| log::warn!("cannot read cloud replay {url}: {error}")).ok()
    }
}
