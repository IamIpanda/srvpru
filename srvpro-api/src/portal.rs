//! The portal bundle and the plugin that serves it.

use axum::Json;
use axum::Router;
use axum::routing::get;
use serde::Serialize;
use tower_http::services::ServeDir;
#[cfg(not(feature = "embedded-portal"))]
use tower_http::services::ServeFile;
use ygopro_derive::Configuration;

/// Name for activitating this module in the plugin system.
#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync, register_to = "srvpro::plugin::CONFIGURATIONS")]
pub struct Configuration {
    /// Directory holding the built portal; empty disables static file serving.
    #[config(default = "\"portal\".to_string()")]
    pub static_dir: String,
}

/// Serves the portal bundle, its manifest and answers unknown paths with its `index.html`, so the client side router owns them.
#[api]
fn build_portal_router() -> Router {
    let router = Router::new().route("/api/manifest", get(manifest));
    let static_dir = srvpro::configuration::get_configuration::<Configuration>().map(|configuration| configuration.static_dir).unwrap_or_default();
    if static_dir.is_empty() { return router }
    #[cfg(feature = "embedded-portal")]
    return router.fallback_service(ServeDir::new(&static_dir).fallback(get(embedded::fallback)));
    #[cfg(not(feature = "embedded-portal"))]
    router.fallback_service(ServeDir::new(&static_dir).not_found_service(ServeFile::new(format!("{static_dir}/index.html"))))
}

#[cfg(feature = "embedded-portal")]
mod embedded {
    use std::sync::LazyLock;

    use axum::body::Body;
    use axum::http::HeaderMap;
    use axum::http::Method;
    use axum::http::StatusCode;
    use axum::http::Uri;
    use axum::http::header;
    use axum::response::IntoResponse;
    use axum::response::Response;
    use http_body_util::BodyExt;
    use web_static_pack::common::pack::PackArchived;
    use web_static_pack::loader::load;
    use web_static_pack::responder::Responder;

    use super::Configuration;

    static PACK: &[u8] = include_bytes_aligned::include_bytes_aligned!(16, concat!(env!("CARGO_MANIFEST_DIR"), "/../srvpro-portal.pack"));

    static RESPONDER: LazyLock<Responder<'static, PackArchived>> = LazyLock::new(|| Responder::new(unsafe { load(PACK) }.unwrap()));

    pub async fn fallback(method: Method, uri: Uri, headers: HeaderMap) -> Response {
        if let Some(response) = serve(&method, uri.path(), &headers).await { return response }
        let static_dir = srvpro::configuration::get_configuration::<Configuration>().map(|configuration| configuration.static_dir).unwrap_or_default();
        if let Ok(index) = tokio::fs::read(format!("{static_dir}/index.html")).await { return ([(header::CONTENT_TYPE, "text/html")], index).into_response() }
        serve(&Method::GET, "/index.html", &headers).await.unwrap_or_else(|| StatusCode::NOT_FOUND.into_response())
    }

    async fn serve(method: &Method, path: &str, headers: &HeaderMap) -> Option<Response> {
        let response = RESPONDER.respond_flatten(method, path, headers);
        if response.status() == StatusCode::NOT_FOUND { return None }
        let (parts, body) = response.into_parts();
        Some(Response::from_parts(parts, Body::from(body.collect().await.ok()?.to_bytes())))
    }
}

#[derive(Serialize)]
pub struct Page {
    pub plugin: &'static str,
    pub route: &'static str,
    pub export: &'static str,
    pub topbar: bool,
}

#[derive(Serialize)]
pub struct Bootstrap {
    pub plugin: &'static str,
}

#[derive(Serialize)]
pub struct Icon {
    pub plugin: &'static str,
    pub order: i32,
    pub export: &'static str,
}

#[derive(Serialize)]
pub struct ConfigTab {
    pub plugin: &'static str,
    pub title: &'static str,
    pub order: i32,
    pub toggle: bool,
    pub content: ConfigTabContent,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigTabContent {
    Fields(&'static [ConfigField]),
    Component { export: &'static str },
}

#[derive(Serialize)]
pub struct ConfigField {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: ConfigFieldKind,
    pub default: &'static str,
    pub options: &'static [(&'static str, &'static str)],
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFieldKind {
    Text,
    Multiline,
    Boolean,
    Select,
    Tags,
    Json,
}

#[derive(Serialize)]
struct Manifest {
    bootstraps: Vec<&'static Bootstrap>,
    pages: Vec<&'static Page>,
    icons: Vec<&'static Icon>,
    tabs: Vec<&'static ConfigTab>,
}

#[distributed_slice]
pub static PORTAL_CONFIG_TABS: [ConfigTab];
#[distributed_slice]
pub static PORTAL_PAGES: [Page];
#[distributed_slice]
pub static PORTAL_ICONS: [Icon];
#[distributed_slice]
pub static PORTAL_BOOTSTRAPS: [Bootstrap];
#[distributed_slice]
pub static PORTAL_GLOBAL_PAGES: [Page];
#[distributed_slice]
pub static PORTAL_GLOBAL_ICONS: [Icon];

async fn manifest() -> Json<Manifest> {
    let configuration = srvpro::configuration::get();
    let enabled = &configuration.enable_plugins;
    let bootstraps = PORTAL_BOOTSTRAPS.iter().filter(|bootstrap| enabled.contains(bootstrap.plugin)).collect();
    let pages = PORTAL_GLOBAL_PAGES.iter().chain(PORTAL_PAGES.iter().filter(|page| enabled.contains(page.plugin))).collect();
    let mut icons = PORTAL_GLOBAL_ICONS.iter().chain(PORTAL_ICONS.iter().filter(|icon| enabled.contains(icon.plugin))).collect::<Vec<_>>();
    icons.sort_by_key(|icon| icon.order);
    let mut tabs = PORTAL_CONFIG_TABS.iter().collect::<Vec<_>>();
    tabs.sort_by_key(|tab| tab.order);
    Json(Manifest { bootstraps, pages, icons, tabs })
}
