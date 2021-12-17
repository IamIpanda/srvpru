// ============================================================
// arena
// ------------------------------------------------------------
//! Offer arena tools.
// ============================================================
use crate::srvpru::generate_chat;
use crate::srvpru::message::ServerStart;
use crate::ygopro::Colors;
use crate::ygopro::message::ctos;
use crate::srvpru::Handler;
use crate::srvpru::CommonError;

set_configuration! {
    arena: String,
    init: Option<String>,
    permit: Option<String>,
    get_score: Option<String>,
    access_key: String
}

use_http_client!();

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())
}

fn register_handlers() {
    Handler::follow_message::<ServerStart, _>(100, "arena_init", |_, _| Box::pin(async move {
        let configuration = get_configuration();
        if let Some(init) = configuration.permit.as_ref() {
            if let Err(e) = get_http_client().post(init)
            .query(&[
                "ak", &configuration.access_key,
                "arena", &configuration.arena
            ]).send().await {
                warn!("Arena init post error: {:?}", e)
            }
        }
        Ok(false)
    })).register_for_plugin("arena");

    Handler::before_message::<ctos::JoinGame, _>(10, "arena", |context, message| Box::pin(async move {
        let configuration = get_configuration();
        if ! context.parameters.contains_key("flag_arena") { return Ok(false) }
        if let Some(permit) = configuration.permit.as_ref() {
            let player_name = context.get_player().ok_or(CommonError::PlayerNotExist)?.lock().name.clone();
            let room_name = context.get_string(&message.pass, "pass")?;
            if ! get_http_client().get(permit)
                .query(&[
                    ("username", &player_name),
                    ("password", room_name),
                    ("arena", &configuration.arena)
                ]).send().await?
                .json::<PermitResponse>().await?.permit {
                return context.refuse_join_game(Some("{invalid_password_unauthorized}")).await;
            }
        }
        if let Some(get_score) = configuration.get_score.as_ref() {
            let player_name = context.get_player().ok_or(CommonError::PlayerNotExist)?.lock().name.clone();
            let response = get_http_client().get(get_score).query(&["username", &player_name]).send().await?.json::<GetScoreResponse>().await?;
            let rank_text = if response.arena_rank > 0 { format!("{{rank_arena}}{}", response.arena_rank) } else { "rank_blank".to_string() };
            context.send_back(&generate_chat(&format!(
                "{}{{exp_value_part1}}{}{{exp_value_part2}}{{exp_value_part3}}{}{}{{exp_value_part4}}", 
                player_name, 
                response.exp,
                response.pt.round(),
                rank_text), Colors::Babyblue, context.get_region())).await.ok();
        }
        Ok(false)
    })).register_for_plugin("arena");
}

#[derive(serde::Deserialize)]
struct PermitResponse {
    permit: bool 
}

#[derive(serde::Deserialize)]
struct GetScoreResponse {
    pt: f64,
    exp: u64,
    arena_rank: u64
}