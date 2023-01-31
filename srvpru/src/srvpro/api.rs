use axum::Router;
use http::Request;
use once_cell::sync::Lazy;
use tower::Service;

pub static mut ROUTER: Lazy<Router> = Lazy::new(|| Router::new());

pub fn handle(request: &Request<()>) {
}
