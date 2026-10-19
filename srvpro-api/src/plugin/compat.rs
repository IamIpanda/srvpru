//! srvpro's `/api/message` compatibility endpoint for existing dashboards.

use axum::Router;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;

use crate::auth;
use crate::plugin::death;
use crate::plugin::shout;
use crate::plugin::stop;
use crate::plugin::welcome;
use crate::register_router;

/// Name for activitating this module in the plugin system.
#[distributed_slice(srvpro::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_router!(COMPAT_ROUTER, build_compat_router);

#[derive(Deserialize)]
struct CompatQuery {
    #[serde(default)]
    callback: String,
    shout: Option<String>,
    stop: Option<String>,
    welcome: Option<String>,
    getwelcome: Option<String>,
    death: Option<String>,
    deathcancel: Option<String>,
}

fn build_compat_router() -> Router {
    Router::new().route("/api/message", get(message))
}

async fn message(request: Request) -> Response {
    let Ok(query) = serde_urlencoded::from_str::<CompatQuery>(request.uri().query().unwrap_or_default()) else {
        return StatusCode::BAD_REQUEST.into_response()
    };
    let callback = query.callback.as_str();
    if let Some(value) = filled(query.shout) {
        if !auth::authorized(&request, "shout") { return reply(callback, json!(["密码错误", 0])) }
        shout::broadcast(&value);
        return reply(callback, json!(["shout ok", value]))
    }
    if let Some(value) = filled(query.stop) {
        if !auth::authorized(&request, "stop") { return reply(callback, json!(["密码错误", 0])) }
        stop::set(&value).await;
        let payload = if value == "false" { Value::Bool(false) } else { Value::String(value) };
        return reply(callback, json!(["stop ok", payload]))
    }
    if let Some(value) = filled(query.welcome) {
        if !auth::authorized(&request, "change_settings") { return reply(callback, json!(["密码错误", 0])) }
        return match welcome::change(&value).await {
            Ok(()) => reply(callback, json!(["welcome ok", value])),
            Err(error) => {
                log::warn!("cannot change welcome: {error}");
                reply(callback, json!(["welcome fail", value]))
            },
        }
    }
    if filled(query.getwelcome).is_some() {
        if !auth::authorized(&request, "change_settings") { return reply(callback, json!(["密码错误", 0])) }
        return reply(callback, json!(["get ok", welcome::current()]))
    }
    if let Some(value) = filled(query.death) {
        if !auth::authorized(&request, "start_death") { return reply(callback, json!(["密码错误", 0])) }
        if death::send_command(&value, "start_death") { return reply(callback, json!(["death ok", value])) }
        return reply(callback, json!(["room not found", value]))
    }
    if let Some(value) = filled(query.deathcancel) {
        if !auth::authorized(&request, "start_death") { return reply(callback, json!(["密码错误", 0])) }
        if death::send_command(&value, "cancel_death") { return reply(callback, json!(["death cancel ok", value])) }
        return reply(callback, json!(["room not found", value]))
    }
    StatusCode::BAD_REQUEST.into_response()
}

fn filled(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

fn reply(callback: &str, body: Value) -> Response {
    let body = body.to_string();
    if callback.is_empty() {
        ([(CONTENT_TYPE, "application/json")], body).into_response()
    } else {
        ([(CONTENT_TYPE, "text/javascript")], format!("{callback}( {body} );")).into_response()
    }
}
