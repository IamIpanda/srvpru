use srvpro::managers;

use std::sync::LazyLock;

use arc_swap::ArcSwap;
use axum::Router;
use axum::extract::Request;
use axum::response::Response;
use tower::ServiceExt;
use tower_http::services::ServeDir;
use tower_http::services::ServeFile;
use ygopro_derive::Configuration;

pub mod auth;
pub mod bad_words;
pub mod bootstrap;
pub mod config;
pub mod database;
pub mod death;
pub mod dialogues;
pub mod plugin;
pub mod roomlist;
pub mod shout;
pub mod stop;
pub mod user;
pub mod welcome;

#[macro_use]
extern crate ygopro_derive;
#[macro_use]
extern crate linkme;

#[distributed_slice]
pub static SRVPRO_API_ROUTERS: [(&'static str, fn() -> Router)];

#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync, prefix = "api", register_to = "srvpro::plugin::CONFIGURATIONS")]
pub struct Configuration {
    #[config(default = "7922")]
    pub port: u16,
    #[config(default = "\"srvpro.db\".to_string()")]
    pub database: String,
    #[config(default = "\"portal\".to_string()")]
    pub static_dir: String,
    #[config(default = "\"Srvpru Server\".to_string()")]
    pub name: String,
}

static ROUTER: LazyLock<ArcSwap<Router>> = LazyLock::new(|| ArcSwap::from_pointee(build_router_from_configuration()));

#[after(srvpro::message::Init)]
#[register_to(srvpro::SRVPRO_GLOBAL_HANDLERS as srvpro::GlobalHandler)]
fn on_init() {
    tokio::spawn(serve());
}

#[handler(srvpro::message::ConfigurationChanged)]
#[register_to(srvpro::SRVPRO_GLOBAL_HANDLERS as srvpro::GlobalHandler)]
fn on_configuration_changed() {
    ROUTER.store(std::sync::Arc::new(build_router_from_configuration()));
}

pub async fn serve() {
    let Some(configuration) = srvpro::configuration::get_configuration::<Configuration>() else { return };
    let port = configuration.port;
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.expect("cannot bind port");
    log::info!("api listening on port {}", port);
    axum::serve(listener, Router::new().fallback(dispatch)).await.expect("api server failed");
}

fn build_router_from_configuration() -> Router {
    let configuration = srvpro::configuration::get();
    let static_dir = srvpro::configuration::get_configuration::<Configuration>().map(|configuration| configuration.static_dir).unwrap_or_default();
    build_router(&configuration.enable_plugins, &static_dir)
}

fn build_router(enabled_plugins: &hashbrown::HashSet<String>, static_dir: &str) -> Router {
    let router = SRVPRO_API_ROUTERS.iter().fold(Router::new(), |router, (plugin, build)| {
        if enabled_plugins.contains(*plugin) {
            router.merge(build())
        } else {
            router
        }
    });
    if static_dir.is_empty() { return router }
    router.fallback_service(ServeDir::new(static_dir).not_found_service(ServeFile::new(format!("{static_dir}/index.html"))))
}

async fn dispatch(request: Request) -> Response {
    let router = ROUTER.load_full();
    Router::clone(&router).oneshot(request).await.expect("router service is infallible")
}

#[macro_export]
macro_rules! register_router {
    ($name:ident, $build:path) => {
        #[::linkme::distributed_slice($crate::SRVPRO_API_ROUTERS)]
        static $name: (&'static str, fn() -> ::axum::Router) = (module_path!(), $build);
    };
}
