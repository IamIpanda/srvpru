//! srvpro's room string is `<modes>#<password>`. The modes are either the legacy compact
//! form or comma-separated flags like `M`, `T`, `BO5` or `G`.
//!
//! This module is the [`ModeHandler`] architecture: the chain runs when a room is created,
//! parses the room string and fills in everything the room needs to start.

mod basic;
pub(crate) mod common;
mod srvpru;

use std::future::Future;
use std::ops::Deref;
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::LazyLock;

use ygopro::host::DuelHost;
use ygopro_data::complex::Complex;
use ygopro_data::constants::Netplayer;
use ygopro_data::message::HostInfo;
use ygopro_data::message::{ctos, stoc};
use ygopro_external_server_bridge::YgoproBinaryFactory;
use ygopro_external_server_bridge::YgoproBinaryProvider;
use ygopro_handler::RoomProvider;
use ygopro_handler::extract::ContainsMapMut;
use ygopro_handler::sync_handler::SyncHandler;
use ygopro_handler::Bundle;
use ygopro_handler::FromRequest;
use ygopro_handler::Processor;

use crate::message as srvpro;
use crate::GlobalHandler;
use crate::room::State;

/// A handler that consumes a mode string, or one comma-separated part of it.
pub type ModeHandler = SyncHandler<String, RoomConfiguration, ModeResponse>;
/// Room name handlers, collected at link time.
#[distributed_slice]
pub static MODES: [fn() -> (u8, ModeHandler)];

/// Whether the segment was consumed by a handler.
#[derive(Clone, Copy, Debug, Default)]
pub struct ModeResponse(pub bool);

impl std::ops::Mul for ModeResponse {
    type Output = ModeResponse;

    fn mul(self, rhs: Self) -> Self::Output {
        ModeResponse(self.0 || rhs.0)
    }
}

impl ygopro_handler::IntoResponse<ModeResponse> for bool {
    fn into_response(self) -> ModeResponse {
        ModeResponse(self)
    }
}

impl ygopro_handler::IntoResponse<ModeResponse> for () {
    fn into_response(self) -> ModeResponse {
        ModeResponse(false)
    }
}

/// A marker mark this room is a tag duel room.
pub struct Tag;

/// Whether the room is a tag duel, available as a request extractor.
pub struct TagFlag(pub bool);

impl<Req, State, Res> FromRequest<Req, State, Res> for TagFlag
where Req: Send, State: ContainsMapMut + Send, Res: Send
{
    fn from_request(bundle: &mut Bundle<Req, State, Res>) -> Option<Self> {
        Some(TagFlag(ContainsMapMut::get_map(&mut bundle.state).get::<Tag>().is_some()))
    }
}

/// The netplayer sitting opposite `player` in a tag or single duel.
pub fn opponent(tag: bool, player: Netplayer) -> Netplayer {
    match player {
        Netplayer::Player(index) => Netplayer::Player(if tag { index ^ 2 } else { index ^ 1 }),
        other => other,
    }
}

#[derive(Debug)]
/// The mode handler of normal rooms.
pub struct Normal {
    _private: (),
}

impl ygopro_data::message::PureMessage for Normal {}

impl ygopro_data::message::Message for Normal {
    fn message_type() -> ygopro_data::message::all::MessageType {
        ygopro_data::message::all::MessageType::Other("mode", 1)
    }
}

type ModeProcessor = Processor<u8, String, RoomConfiguration, ModeResponse, ModeHandler>;
static MODE_PROCESSOR: LazyLock<arc_swap::ArcSwap<ModeProcessor>> = LazyLock::new(|| arc_swap::ArcSwap::from_pointee(build_mode_processor()));

fn build_mode_processor() -> ModeProcessor {
    let configuration = crate::configuration::get();
    let mut processor = Processor::new();
    for mode_generator in MODES {
        let (key, handler) = (mode_generator)();
        if !configuration.enable_plugins.contains(handler.module_name) { continue; }
        if key == 0 { processor.register_global(handler); } else { processor.register(key, handler); }
    }
    processor
}

#[handler(srvpro::ConfigurationChanged)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn on_configuration_changed() {
    MODE_PROCESSOR.store(std::sync::Arc::new(build_mode_processor()));
}

/// Which duel provider a room creates.
#[derive(Debug, Default, Clone, Copy)]
pub enum ProviderType {
    #[default]
    Embedded,
    Remote
}

/// The duel provider of a room, either embedded in this process or a remote ygopro binary.
pub enum Provider {
    Embedded(DuelHost),
    Remote(YgoproBinaryProvider)
}

/// Extractor matching rooms served by an embedded duel host.
pub struct EmbedProvider;
/// Extractor matching rooms served by a remote ygopro binary.
pub struct RemoteProvider;

impl<Req: Send, Res: Send> FromRequest<Req, State, Res> for EmbedProvider {
    fn from_request(bundle: &mut Bundle<Req, State, Res>) -> Option<Self> {
        matches!(bundle.state.room.provider, Some(Provider::Embedded(_))).then_some(EmbedProvider)
    }
}

impl<Req: Send, Res: Send> FromRequest<Req, State, Res> for RemoteProvider {
    fn from_request(bundle: &mut Bundle<Req, State, Res>) -> Option<Self> {
        matches!(bundle.state.room.provider, Some(Provider::Remote(_))).then_some(RemoteProvider)
    }
}

impl RoomProvider<Complex<ctos::Message>, Complex<stoc::Message>> for Provider {
    type ServerToClientStream = tokio_stream::wrappers::UnboundedReceiverStream<Complex<stoc::Message>>;
    type FinishFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

    fn add(&mut self, client_to_server_stream: impl futures::Stream<Item = Complex<ctos::Message>> + Unpin + Send + 'static) -> Self::ServerToClientStream {
        match self {
            Provider::Embedded(single_duel_host) => <DuelHost as RoomProvider<Complex<ctos::Message>, Complex<stoc::Message>>>::add(single_duel_host, client_to_server_stream),
            Provider::Remote(ygopro_binary_provider) => RoomProvider::<Complex<ctos::Message>, Complex<stoc::Message>>::add(ygopro_binary_provider, client_to_server_stream),
        }
    }

    fn get_finish_signal(&mut self) -> Self::FinishFuture {
        match self {
            Provider::Embedded(single_duel_host) => <DuelHost as RoomProvider<Complex<ctos::Message>, Complex<stoc::Message>>>::get_finish_signal(single_duel_host),
            Provider::Remote(ygopro_binary_provider) => RoomProvider::<Complex<ctos::Message>, Complex<stoc::Message>>::get_finish_signal(ygopro_binary_provider),
        }
    }
}

type Anymap = anymap3::Map<dyn std::any::Any + Send>;
/// Provider settings of a room.
#[derive(Default)]
pub struct RoomProviderConfiguration {
    pub provider: ProviderType,
    pub hostinfo: HostInfo,
    pub ygopru_configuration: ygopro::Configuration,
}

/// Everything a room needs: its name, its provider and the states of its handlers.
pub struct RoomConfiguration {
    pub name: String,
    pub origin_name: String,
    pub srvpro_configuration: crate::configuration::Configuration,
    pub provider_configuration: RoomProviderConfiguration,
    pub states: Anymap
}

impl Default for RoomConfiguration {
    fn default() -> Self {
        Self { name: Default::default(), origin_name: Default::default(), srvpro_configuration: (*crate::configuration::get()).clone(), provider_configuration: Default::default(), states: Default::default() }
    }
}

impl Deref for RoomConfiguration {
    type Target = RoomProviderConfiguration;

    fn deref(&self) -> &RoomProviderConfiguration {
        &self.provider_configuration
    }
}

impl DerefMut for RoomConfiguration {
    fn deref_mut(&mut self) -> &mut RoomProviderConfiguration {
        &mut self.provider_configuration
    }
}

impl<Res> FromRequest<String, RoomConfiguration, Res> for &mut RoomConfiguration
where Res: Send,
{
    fn from_request(bundle: &mut Bundle<String, RoomConfiguration, Res>) -> Option<Self> {
        Some(unsafe { &mut *(&mut bundle.state as *mut RoomConfiguration) })
    }
}

impl<Res> FromRequest<String, RoomConfiguration, Res> for &str
where Res: Send,
{
    fn from_request(bundle: &mut Bundle<String, RoomConfiguration, Res>) -> Option<Self> {
        Some(unsafe { &*(bundle.request.as_str() as *const str) })
    }
}

impl ContainsMapMut for RoomConfiguration {
    fn get_map(&mut self) -> &mut Anymap {
        &mut self.states
    }
}

impl RoomProviderConfiguration {
    /// Create the provider described by this configuration.
    pub async fn create_room_provider(&mut self, name: String) -> Result<Provider, std::io::Error> {
        let hostinfo = self.hostinfo.clone();
        match self.provider {
            ProviderType::Embedded => {
                let duel = DuelHost::new(hostinfo, std::mem::take(&mut self.ygopru_configuration));
                Ok(Provider::Embedded(duel))
            },
            ProviderType::Remote => {
                let factory = YgoproBinaryFactory::new("./ygopro".to_string(), "./ygopro".to_string());
                let duel = factory.start(name, hostinfo, None).await;
                match duel {
                    Ok(duel) => Ok(Provider::Remote(duel)),
                    Err(err) => {
                        log::warn!("Failed to create remote duel: {:?}", err);
                        Err(err)
                    },
                }
            },
        }
    }
}

impl RoomConfiguration {
    /// Build a configuration from a room name.
    pub async fn new(pass: &str) -> Self {
        Self::default().analyze_hash(pass).await
    }

    async fn analyze_hash(mut self, pass: &str) -> Self {
        let (modes, _) = pass.split_once('#').unwrap_or(("", pass));
        self.name = pass.to_string();
        self.origin_name = pass.to_string();
        let mut this = self;
        let response = {
            let bundle = Bundle::new(modes.to_string(), this, ModeResponse::default());
            let bundle = MODE_PROCESSOR.load().process_bundle(bundle, 0).await;
            this = bundle.state;
            bundle.response
        };
        if !response.0 {
            for part in modes.to_uppercase().split(['，', ',']) {
                let bundle = Bundle::new(part.to_string(), this, ModeResponse::default());
                let bundle = MODE_PROCESSOR.load().process_bundle(bundle, 1).await;
                this = bundle.state;
            }
        }
        this
    }
}

/// Parse a room name segment written as `prefix` followed by a value.
/// 
/// That's useful for parsing room config like `LP16000`.
pub fn slice_with_prefix<T: std::str::FromStr>(slice: &str, prefix: &str) -> Option<T> {
    if slice.starts_with(prefix) && let Ok(v) = slice[prefix.len()..].parse() {
        Some(v)
    } else { None }
}
