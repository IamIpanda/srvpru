#![doc = include_str!("../README.md")]

use srvpro::managers;

use std::sync::LazyLock;

use arc_swap::ArcSwap;
use axum::Router;
use axum::extract::Request;
use axum::response::Response;
use tower::ServiceExt;
use ygopro_derive::Configuration;

use crate::portal::*;

pub mod auth;
pub mod bootstrap;
pub mod config;
pub mod database;
pub mod plugin;
pub mod portal;

#[macro_use]
extern crate ygopro_derive;
#[macro_use]
extern crate linkme;

/// Routers contributed by the API plugins, keyed by plugin name.
#[distributed_slice]
pub static SRVPRO_API_ROUTERS: [(&'static str, fn() -> Router)];

/// Routers mounted no matter which plugins are enabled.
#[distributed_slice]
pub static SRVPRO_API_GLOBAL_ROUTERS: [fn() -> Router];

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

/// HTTP API settings.
#[derive(Clone, Configuration)]
#[config(sync, prefix = "api", register_to = "srvpro::plugin::CONFIGURATIONS")]
pub struct Configuration {
    /// Port the HTTP server listens on.
    #[config(default = "7922")]
    pub port: u16,
    /// SQLite database file holding accounts and configuration.
    #[config(default = "\"srvpro.db\".to_string()")]
    pub database: String,
    /// Server name shown by the API.
    #[config(default = "\"Srvpru Server\".to_string()")]
    pub name: String,
}

#[distributed_slice(PORTAL_CONFIG_TABS)]
static GENERAL_TAB: ConfigTab = ConfigTab {
    plugin: "srvpro::core",
    title: "通用",
    order: 0,
    toggle: false,
    content: ConfigTabContent::Fields(&[
        ConfigField { key: "api_name", label: "服务器名称", kind: ConfigFieldKind::Text, default: "Srvpru Server", options: &[] },
        ConfigField { key: "auth_token_ttl_days", label: "登录有效期（天）", kind: ConfigFieldKind::Text, default: "180", options: &[] },
        ConfigField { key: "api_port", label: "HTTP 端口", kind: ConfigFieldKind::Text, default: "7922", options: &[] },
        ConfigField { key: "server_port", label: "ygopro 端口", kind: ConfigFieldKind::Text, default: "7911", options: &[] },
    ]
)};

static ROUTER: LazyLock<ArcSwap<Router>> = LazyLock::new(|| ArcSwap::from_pointee(build_router_from_configuration()));

#[after(srvpro::message::Init)]
#[register_to(srvpro::SRVPRO_GLOBAL_HANDLERS as srvpro::GlobalHandler)]
fn on_init() {
    tokio::spawn(serve());
}

#[after(srvpro::message::ConfigurationChanged)]
#[register_to(srvpro::SRVPRO_GLOBAL_HANDLERS as srvpro::GlobalHandler)]
fn on_configuration_changed() {
    ROUTER.store(std::sync::Arc::new(build_router_from_configuration()));
}

async fn serve() {
    let Some(configuration) = srvpro::configuration::get_configuration::<Configuration>() else { return };
    let port = configuration.port;
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.expect("cannot bind port");
    log::info!("api listening on port {}", port);
    axum::serve(listener, Router::new().fallback(dispatch).into_make_service_with_connect_info::<std::net::SocketAddr>()).await.expect("api server failed");
}

fn build_router_from_configuration() -> Router {
    let configuration = srvpro::configuration::get();
    build_router(&configuration.enable_plugins)
}

fn build_router(enabled_plugins: &hashbrown::HashSet<String>) -> Router {
    let enabled = SRVPRO_API_ROUTERS.iter().filter(|(plugin, _)| enabled_plugins.contains(*plugin)).map(|(_, build)| build);
    SRVPRO_API_GLOBAL_ROUTERS.iter().chain(enabled).fold(Router::new(), |router, build| router.merge(build()))
}

async fn dispatch(request: Request) -> Response {
    let router = ROUTER.load_full();
    Router::clone(&router).oneshot(request).await.expect("router service is infallible")
}
