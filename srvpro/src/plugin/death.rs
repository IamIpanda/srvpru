use tokio::task::JoinHandle;
use ygopro_data::complex::Complex;
use ygopro_data::constants::Color;
use ygopro_data::constants::DuelStage;
use ygopro_data::constants::Netplayer;
use ygopro_data::message::ctos;
use ygopro_data::message::gm;
use ygopro_data::message::stoc;
use ygopro_derive::Attachment;
use ygopro_derive::Configuration;
use ygopro_derive::after;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::mode::*;
use crate::room::*;
use crate::plugin::base::first_attack::FirstAttack;
use crate::plugin::base::lp::Lp;
use crate::plugin::base::score::Score;
use crate::plugin::base::stage::Stage;
use crate::plugin::register_dependencies;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::first_attack::NAME,
    crate::plugin::base::lp::NAME,
    crate::plugin::base::score::NAME,
    crate::plugin::base::stage::NAME,
    crate::plugin::base::position::NAME
);

#[derive(Clone)]
enum QuickDeathRule {
    Death(u8),
}

impl Default for QuickDeathRule {
    fn default() -> Self {
        Self::Death(3)
    }
}

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(not_from_env)]
    pub quick_death_rule: QuickDeathRule,
}

#[derive(Attachment)]
#[attachment(no_default)]
pub struct Death {
    pub minutes: u8,
    pub count_down: Option<JoinHandle<()>>,
    /// <0: Not started.
    /// 0: Sudden death.
    /// 1: This is final turn.
    /// 4: This is current turn.
    pub remain_turns: i32
}

#[handler(Normal)]
#[register_to(MODES as ModeHandler)]
fn death_mode(config: &mut RoomConfiguration, part: &str) -> bool {
    let mut minutes = None;
    if part == "DEATH" || part == "DH" {
        minutes = Some(40);
    } else if let Some(min) = slice_with_prefix(part, "DEATH").or(slice_with_prefix(part, "DH")) {
        minutes = Some(min);
    };
    if let Some(minutes) = minutes {
        config.states.insert(Death { 
            minutes,
            count_down: None,
            remain_turns: -1
        });
    }
    minutes.is_some()
}

#[handler(stoc::DuelStart)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_duel_start(player: &mut Player, death: &mut Death, sender: Sender) {
    if player.states.get::<bool>() != Some(&true) { return }
    if death.count_down.is_some() { return }
    let minutes = death.minutes as u64;
    death.count_down = Some(tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_mins(minutes)).await;
        sender.0.unbounded_send(Request::Command("start_death", Box::new(()))).ok();
    }));
}

#[command]
#[register_to(SRVPRO_COMMANDS as CommandHandler with &'static str)]
fn start_death(room: &mut Room, death: &mut Death, stage: &mut Stage, score: &mut Score, configuration: Configuration) {
    if death.remain_turns > 0 { return }
    if stage.stage == DuelStage::Siding {
        let score_a = score.winners.iter().filter(|winner| **winner == Some(Netplayer::Player(0))).count();
        let score_b = score.winners.iter().filter(|winner| **winner == Some(Netplayer::Player(1))).count();
        if score_a == score_b {
            death.remain_turns = match configuration.quick_death_rule {
            QuickDeathRule::Death(turn) => (turn + 2).into(),
        };
            room.broadcast_message(Color::Red, "比分相同，进行 4 回合额外决斗。");
        } else {
            let loser = if score_a > score_b { Netplayer::Player(1) } else { Netplayer::Player(0) };
            kick_from_provider(room, loser);
            room.broadcast_message(Color::Red, "比分不同，决斗结束。");
            death.remain_turns = -1;
        }
    } else {
        death.remain_turns = match configuration.quick_death_rule {
            QuickDeathRule::Death(turn) => (turn + 1).into(),
        };
        room.broadcast_message(Color::Red, "本轮限制时间已结束。开始死亡回合。");
    }
}

fn kick_from_provider(room: &mut Room, target: Netplayer) {
    let Some(position) = room.players.iter().find_map(|(index, player)| {
        (player.states.get::<Netplayer>() == Some(&target)).then_some(index)
    }) else { return };
    if let Some(player) = room.players.get_mut(position) {
        player.client_to_server_sink_towards_provider.unbounded_send(Complex::from_message(ctos::Message::LeaveGame(ctos::LeaveGame))).ok();
    }
}

fn sudden_death_judge(room: &mut Room, death: &mut Death, lp: &Lp, first_attack: &FirstAttack, tag: bool) {
    let Some(first_attacker) = first_attack.first_attack.last() else { return };
    if lp.lp[0] == lp.lp[1] { return }
    let loser = if lp.lp[0] > lp.lp[1] {
        opponent(tag, *first_attacker)
    } else {
        *first_attacker
    };
    kick_from_provider(room, loser);
    room.broadcast_message(Color::Red, "加时赛结束，LP 高者胜。");
    death.remain_turns = -1;
}

#[handler(gm::NewTurn)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_new_turn(room: &mut Room, index: usize, death: &mut Death, lp: &mut Lp, first_attack: &mut FirstAttack, tag: TagFlag) {
    let Some(player) = room.players.get(index) else { return };
    if player.states.get::<bool>() != Some(&true) { return };
    if death.remain_turns < 0 { return };
    if death.remain_turns > 0 { death.remain_turns -= 1; }
    if death.remain_turns == 0 {
        sudden_death_judge(room, death, lp, first_attack, tag.0);
    }
}

#[after(gm::LPUpdate)]
#[register_to(GM_HANDLERS as GameMessageHandler)]
fn on_lp_update(room: &mut Room, index: usize, death: &mut Death, lp: &mut Lp, first_attack: &mut FirstAttack, tag: TagFlag) {
    let Some(player) = room.players.get(index) else { return };
    if player.states.get::<bool>() != Some(&true) { return };
    if death.remain_turns != 0 { return }
    sudden_death_judge(room, death, lp, first_attack, tag.0);
}
