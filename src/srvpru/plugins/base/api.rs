use axum::Router;
use parking_lot::Mutex;
use once_cell::sync::OnceCell;

use crate::srvpru::Handler;
use crate::srvpru::message::ServerStart;

set_configuration! {
    #[serde(default = "default_port")]
    port: u16
}
fn default_port() -> u16 { 7933 }

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    load_configuration()?;
    Ok(())
}

fn register_handlers() {
    Handler::before_message::<ServerStart, _>(100, "api", |_, _| Box::pin(async move {
        start_server();
        Ok(false)
    })).register_for_plugin("api");
}

static ROUTER: OnceCell<Mutex<Option<Router>>> = OnceCell::new();
pub fn register_api<F: FnOnce(Router) -> Router>(register: F) {
    let mut router = ROUTER.get_or_init(|| Mutex::new(Some(Router::new()))).lock();
    let _router = router.take().expect("Router has been taken. It seems server already start.");
    let _router = register(_router);
    router.replace(_router);
}

fn start_server() {
    let configuration = get_configuration();
    let router_wrapper = match ROUTER.get() {
        Some(router_wrapper) => router_wrapper,
        None => {
            warn!("No api registered to server. Server on port {} won't start.", configuration.port);
            return;
        }
    };
    let app = router_wrapper.lock().take().expect("Router has been taken. It seems server already start.");
    tokio::spawn(async move {
        axum::Server::bind(&format!("0.0.0.0:{}", configuration.port).parse().unwrap())
        .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr, _>())
        .await
        .unwrap();
    });
}