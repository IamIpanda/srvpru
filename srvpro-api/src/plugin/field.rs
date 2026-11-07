//! Duel field of one room, as tracked by `base::field`.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use serde::Serialize;
use slotmap::Key;
use tokio::time::timeout;
use ygopro_data::field as duel;

use srvpro::plugin::base::field::Field;
use srvpro::room::ROOMS;

use crate::auth;

const PERMISSION: &str = "get_field";
const QUERY_TIMEOUT: Duration = Duration::from_secs(1);

#[api(srvpro::plugin::base::field::NAME)]
fn build_field_router() -> Router {
    auth::required(PERMISSION, Router::new().route("/api/field/{room}", get(get_field)))
}

async fn get_field(Path(room): Path<String>) -> Response {
    let receiver = {
        let rooms = ROOMS.read();
        let Some(host) = rooms.get(&room) else { return StatusCode::NOT_FOUND.into_response() };
        host.query(|_, states| states.get::<Field>().map(|tracked| FieldSnapshot::from(&tracked.field)))
    };
    match timeout(QUERY_TIMEOUT, receiver).await {
        Ok(Ok(Some(snapshot))) => Json(snapshot).into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::GATEWAY_TIMEOUT.into_response(),
    }
}

type CardId = u64;

#[derive(Serialize)]
struct FieldSnapshot {
    cards: BTreeMap<CardId, CardSnapshot>,
    deck: [Vec<CardId>; 2],
    hand: [Vec<CardId>; 2],
    mzone: [[Option<CardId>; 7]; 2],
    szone: [[Option<CardId>; 8]; 2],
    grave: [Vec<CardId>; 2],
    remove: [Vec<CardId>; 2],
    extra: [Vec<CardId>; 2],
    extra_pendulum_count: [i32; 2],
    duel_info: DuelInfoSnapshot,
    chains: Vec<ChainSnapshot>,
    current_chain: ChainSnapshot,
    attacker: Option<CardId>,
    attack_target: Option<CardId>,
    disabled_field: u32,
    deck_reversed: bool,
    player_description_hints: [BTreeMap<i32, i32>; 2],
}

#[derive(Serialize)]
struct DuelInfoSnapshot {
    lp: [i32; 2],
    turn: i32,
    duel_rule: u8,
}

#[derive(Serialize)]
struct CardSnapshot {
    code: u32,
    alias: u32,
    card_type: u32,
    level: u32,
    rank: u32,
    link: u32,
    attribute: u32,
    race: u32,
    attack: i32,
    defense: i32,
    base_attack: i32,
    base_defense: i32,
    left_scale: u32,
    right_scale: u32,
    link_marker: u32,
    reason: u32,
    owner: u8,
    controller: u8,
    location: u8,
    sequence: u8,
    position: u8,
    status: u32,
    card_hint_type: Option<u8>,
    card_hint_value: i32,
    overlay_target: Option<CardId>,
    overlayed: Vec<CardId>,
    equip_target: Option<CardId>,
    equipped: Vec<CardId>,
    card_target: Vec<CardId>,
    owner_target: Vec<CardId>,
    counters: BTreeMap<u16, u16>,
    description_hints: BTreeMap<i32, i32>,
}

#[derive(Serialize)]
struct ChainSnapshot {
    chain_card: Option<CardId>,
    code: u32,
    description: i32,
    controller: u8,
    location: u8,
    sequence: u8,
    solved: bool,
    negated: bool,
    disabled: bool,
    target: Vec<CardId>,
}

impl From<&duel::Field> for FieldSnapshot {
    fn from(field: &duel::Field) -> Self {
        Self {
            cards: field.cards.iter().map(|(key, card)| (id(key), card.into())).collect(),
            deck: field.deck.each_ref().map(|zone| ids(zone)),
            hand: field.hand.each_ref().map(|zone| ids(zone)),
            mzone: field.mzone.map(|zone| zone.map(|slot| slot.map(id))),
            szone: field.szone.map(|zone| zone.map(|slot| slot.map(id))),
            grave: field.grave.each_ref().map(|zone| ids(zone)),
            remove: field.remove.each_ref().map(|zone| ids(zone)),
            extra: field.extra.each_ref().map(|zone| ids(zone)),
            extra_pendulum_count: field.extra_pendulum_count,
            duel_info: (&field.duel_info).into(),
            chains: field.chains.iter().map(ChainSnapshot::from).collect(),
            current_chain: (&field.current_chain).into(),
            attacker: field.attacker.map(id),
            attack_target: field.attack_target.map(id),
            disabled_field: field.disabled_field,
            deck_reversed: field.deck_reversed,
            player_description_hints: field.player_description_hints.each_ref().map(|hints| hints.iter().map(|(key, value)| (*key, *value)).collect()),
        }
    }
}

impl From<&duel::DuelInfo> for DuelInfoSnapshot {
    fn from(info: &duel::DuelInfo) -> Self {
        Self { lp: info.lp, turn: info.turn, duel_rule: info.duel_rule.into() }
    }
}

impl From<&duel::Card> for CardSnapshot {
    fn from(card: &duel::Card) -> Self {
        Self {
            code: card.code,
            alias: card.alias,
            card_type: card.card_type.bits(),
            level: card.level,
            rank: card.rank,
            link: card.link,
            attribute: card.attribute.bits(),
            race: card.race.bits(),
            attack: card.attack,
            defense: card.defense,
            base_attack: card.base_attack,
            base_defense: card.base_defense,
            left_scale: card.left_scale,
            right_scale: card.right_scale,
            link_marker: card.link_marker.bits(),
            reason: card.reason.bits(),
            owner: card.owner.into(),
            controller: card.controller.into(),
            location: card.location.bits(),
            sequence: card.sequence,
            position: card.position.bits(),
            status: card.status.bits(),
            card_hint_type: card.card_hint_type.map(u8::from),
            card_hint_value: card.card_hint_value,
            overlay_target: card.overlay_target.map(id),
            overlayed: ids(&card.overlayed),
            equip_target: card.equip_target.map(id),
            equipped: sorted(&card.equipped),
            card_target: sorted(&card.card_target),
            owner_target: sorted(&card.owner_target),
            counters: card.counters.iter().map(|(key, value)| (*key, *value)).collect(),
            description_hints: card.description_hints.iter().map(|(key, value)| (*key, *value)).collect(),
        }
    }
}

impl From<&duel::ChainInfo> for ChainSnapshot {
    fn from(chain: &duel::ChainInfo) -> Self {
        Self {
            chain_card: chain.chain_card.map(id),
            code: chain.code,
            description: chain.description,
            controller: chain.controller.into(),
            location: chain.location.bits(),
            sequence: chain.sequence,
            solved: chain.solved,
            negated: chain.negated,
            disabled: chain.disabled,
            target: sorted(&chain.target),
        }
    }
}

fn id(key: duel::CardKey) -> CardId {
    key.data().as_ffi()
}

fn ids(keys: &[duel::CardKey]) -> Vec<CardId> {
    keys.iter().copied().map(id).collect()
}

fn sorted(keys: &HashSet<duel::CardKey>) -> Vec<CardId> {
    let mut ids: Vec<CardId> = keys.iter().copied().map(id).collect();
    ids.sort_unstable();
    ids
}
