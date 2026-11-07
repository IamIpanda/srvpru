//! Sample each player's TCP round-trip time.
//! 
//! How to do that:
//! use glibc getsockopt() to get TCP_INFO (Linux) or TCP_CONNECTION_INFO (macOS) from the socket.
//! that info contains TTL message.
//! 
//! This plugin runs a sliding window to keep the latest latency samples.

use std::collections::VecDeque;
use std::os::fd::RawFd;
use std::time::Duration;
use std::time::Instant;

use ygopro_data::constants::Color;
use ygopro_derive::Configuration;
use ygopro_derive::command;
use ygopro_derive::handler;
use ygopro_derive::register_to;

use crate::plugin::chat_command::ChatCommandHandler;
use crate::plugin::chat_command::CHAT_COMMANDS;
use crate::room::Anymap;
use crate::room::ClientToServerHandler;
use crate::room::CTOS_HANDLERS;
use crate::room::Room;

/// Name for activitating this module in the plugin system.
#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    /// Samples kept per player.
    #[config(default = "8")]
    pub window: usize,
    /// Milliseconds between two messages of the same player.
    #[config(default = "5")]
    pub batch: u64,
}

#[derive(Default)]
pub struct Latency {
    pub samples: VecDeque<u32>,
    sampled_at: Option<Instant>,
}

impl Latency {
    fn sample(&mut self, socket: RawFd, configuration: &Configuration) {
        if self.sampled_at.is_some_and(|sampled_at| sampled_at.elapsed() < Duration::from_millis(configuration.batch)) { return }
        self.sampled_at = Some(Instant::now());
        if let Some(microseconds) = round_trip(socket) && microseconds > 0 { self.record(microseconds, configuration.window) }
    }

    fn record(&mut self, microseconds: u32, window: usize) {
        if self.samples.len() == window { self.samples.pop_front(); }
        self.samples.push_back(microseconds);
    }
}

#[handler(ygopro_handler::All)]
#[register_to(CTOS_HANDLERS as ClientToServerHandler)]
fn refresh_latency(room: &mut Room, index: usize, configuration: Configuration) {
    if let Some(player) = room.players.get_mut(index) && let Some(socket) = player.states.get::<RawFd>().copied() {
        player.states.entry::<Latency>().or_insert(Latency::default()).sample(socket, &configuration);
    }
}

#[command]
#[register_to(CHAT_COMMANDS as ChatCommandHandler with &'static str)]
fn ping(room: &mut Room, _states: &mut Anymap, index: usize, _args: &str) {
    let Some(player) = room.players.get(index) else { return };
    let Some(latency) = player.states.get::<Latency>() else {
        player.send_message("无法获取延迟", Color::Red);
        return
    };
    if latency.samples.is_empty() {
        player.send_message("无法获取延迟", Color::Red);
        return
    }
    let maximum = latency.samples.iter().copied().max().unwrap_or_default();
    let minimum = latency.samples.iter().copied().min().unwrap_or_default();
    let average = latency.samples.iter().sum::<u32>() as f64 / latency.samples.len() as f64;
    player.send_message(&format!("延迟: 最大 {:.1} ms，最小 {:.1} ms，平均 {:.1} ms", maximum as f64 / 1000.0, minimum as f64 / 1000.0, average / 1000.0), Color::Babyblue);
}

#[cfg(target_os = "linux")]
fn round_trip(fd: RawFd) -> Option<u32> {
    let mut info: libc::tcp_info = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of::<libc::tcp_info>() as libc::socklen_t;
    let code = unsafe { libc::getsockopt(fd, libc::IPPROTO_TCP, libc::TCP_INFO, &mut info as *mut libc::tcp_info as *mut libc::c_void, &mut length) };
    if code != 0 { return None }
    Some(info.tcpi_rtt)
}

#[cfg(target_os = "macos")]
fn round_trip(fd: RawFd) -> Option<u32> {
    let mut info: libc::tcp_connection_info = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of::<libc::tcp_connection_info>() as libc::socklen_t;
    let code = unsafe { libc::getsockopt(fd, libc::IPPROTO_TCP, libc::TCP_CONNECTION_INFO, &mut info as *mut libc::tcp_connection_info as *mut libc::c_void, &mut length) };
    if code != 0 { return None }
    Some(info.tcpi_srtt * 1000)
}
