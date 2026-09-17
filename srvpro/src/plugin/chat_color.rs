use std::str::FromStr;

use ygopro_data::constants::Color;
use ygopro_data::constants::Netplayer;
use ygopro_data::message::stoc;
use ygopro_derive::command;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::plugin::chat_command::ChatCommandHandler;
use crate::plugin::chat_command::CHAT_COMMANDS;
use crate::plugin::register_dependencies;
use crate::room::Room;
use crate::room::STOC_HANDLERS;
use crate::room::ServerToClientHandler;

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

register_dependencies!(
    crate::plugin::base::name::NAME,
    crate::plugin::base::position::NAME
);

#[derive(Clone, Copy, PartialEq, Eq)]
struct ChatColor(Color);

#[command]
#[register_to(CHAT_COMMANDS as ChatCommandHandler with &'static str)]
fn color(room: &mut Room, index: usize, args: &str) {
    let Some(player) = room.players.get_mut(index) else { return };
    let args = args.trim();
    if args.is_empty() {
        match player.states.get::<ChatColor>() {
            Some(color) => player.send_message(&format!("当前颜色: {}", color.0), Color::Babyblue),
            None => player.send_message("当前颜色: 默认", Color::Babyblue),
        }
    } else if args.eq_ignore_ascii_case("help") {
        player.send_message("可用颜色:", Color::Babyblue);
        for color in [Color::Red, Color::Green, Color::Blue, Color::Babyblue, Color::Pink, Color::Yellow, Color::White, Color::Gray, Color::Darkgray] {
            player.send_message(&color.to_string(), color);
        }
    } else if args.eq_ignore_ascii_case("default") {
        player.states.remove::<ChatColor>();
        player.send_message("已重置为默认颜色", Color::Babyblue);
    } else if let Ok(color) = Color::from_str(args) {
        player.states.insert(ChatColor(color));
        player.send_message(&format!("已设置颜色: {}", color), Color::Babyblue);
    } else {
        player.send_message(&format!("颜色不存在: {}", args), Color::Red);
    }
}

#[handler(stoc::Chat)]
#[register_to(STOC_HANDLERS as ServerToClientHandler)]
fn on_chat(room: &mut Room, chat: &stoc::Chat) -> Option<stoc::Chat> {
    let stoc::ChatSource::Player(Netplayer::Player(pid)) = chat.player else { return None };
    if pid >= 4 { return None }
    let Some(sender) = room.players.iter().find(|(_, player)| player.states.get::<Netplayer>() == Some(&Netplayer::Player(pid))) else { return None };
    let Some(color) = sender.1.states.get::<ChatColor>() else { return None };
    let Some(name) = sender.1.states.get::<String>() else { return None };
    Some(stoc::Chat {
        player: color.0.into(),
        msg: format!("{}: {}", name, chat.msg).into(),
    })
}
