use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;
use once_cell::sync::OnceCell;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::task::JoinHandle;

use crate::ygopro::message;
use crate::ygopro::message::srvpru;
use crate::ygopro::message::Struct;
use crate::ygopro::message::MappedStruct;

use crate::srvpru::processor::*;
use crate::srvpru::player::*;

use super::message::InternalProcessError;

pub static SOCKET_SERVER: OnceCell<Server> = OnceCell::new();

pub struct Server {
    pub stoc_processor: Processor,
    pub ctos_processor: Processor,
    internal_processor: Processor
}

impl Server {
    pub fn new() -> Server {
        Server {
            stoc_processor: Processor::new(message::Direction::STOC),
            ctos_processor: Processor::new(message::Direction::CTOS),
            internal_processor: Processor::new(message::Direction::SRVPRU)
        }
    }

    pub fn register_handlers(&mut self, plugins: &[&str], ctos_handlers: &[&str], stoc_handlers: &[&str], internal_handlers: &[&str]) {        
        Server::register_directional_handlers("ctos",     ctos_handlers,     &mut self.ctos_processor);
        Server::register_directional_handlers("stoc",     stoc_handlers,     &mut self.stoc_processor);
        Server::register_directional_handlers("internal", internal_handlers, &mut self.internal_processor);
        Server::check_plugin_dependency(plugins, ctos_handlers, stoc_handlers, internal_handlers);
        self.register_plugin_handlers(plugins);
        self.ctos_processor.prepare();
        self.stoc_processor.prepare();
        self.internal_processor.prepare();
    }

    fn register_directional_handlers(direction_name: &'static str, handler_names: &[&str], target_processor: &mut Processor) {
        let mut handler_library = HANDLER_LIBRARY.write();
        for handler_name in handler_names.iter() {
            if let Some(handler) = handler_library.remove(*handler_name) {
                target_processor.add_handler(handler);
            }
            else { warn!("No {} processor named {}", direction_name, handler_name); }
        }
    }

    fn register_plugin_handlers(&mut self, plugin_names: &[&str]) {
        let mut handlers_library = HANDLER_LIBRARY_BY_PLUGIN.write();
        for plugin_name in plugin_names.iter() {
            if let Some(mut library) = handlers_library.remove(*plugin_name) {
                if let Some(handlers) = library.remove(&message::Direction::CTOS)   { Server::register_directional_handlers("ctos",     &handlers.iter().map(|s| s as &str).collect::<Vec<&str>>(), &mut self.ctos_processor); }
                if let Some(handlers) = library.remove(&message::Direction::STOC)   { Server::register_directional_handlers("stoc",     &handlers.iter().map(|s| s as &str).collect::<Vec<&str>>(), &mut self.stoc_processor); }
                if let Some(handlers) = library.remove(&message::Direction::SRVPRU) { Server::register_directional_handlers("internal", &handlers.iter().map(|s| s as &str).collect::<Vec<&str>>(), &mut self.internal_processor); }
            }
            else { warn!("No plugin named {}", plugin_name); }
        }
    }

    fn check_plugin_dependency(plugins: &[&str], ctos_handlers: &[&str], stoc_handlers: &[&str], internal_handlers: &[&str]) {
        let mut handlers = plugins.to_vec();
        handlers.extend(ctos_handlers.iter());
        handlers.extend(stoc_handlers.iter());
        handlers.extend(internal_handlers.iter());
        let handler_depnedencies = HANDLER_DEPENDENCIES.read();
        for handler in handlers.iter() {
            if ! handler_depnedencies.contains_key(*handler) { continue; }
            let dependencies = handler_depnedencies.get(*handler).unwrap();
            for dependency in dependencies.iter() {
                if !handlers.contains(dependency) {
                    warn!("Plugin {} need plugin {} as dependency, but it's not registered.", handler, dependency);
                }
            }
        }
    }

    pub async fn start(&'static self) -> anyhow::Result<()> {
        let configuration = crate::srvpru::get_configuration();
        let listener = TcpListener::bind(format!("0.0.0.0:{}", configuration.port)).await?;
        let timeout = tokio::time::Duration::from_secs(configuration.timeout);
        info!("Socket server started.");
        loop {
            let (socket, addr) = listener.accept().await?;
            let (mut reader, writer) = socket.into_split();
            let mut writer = Some(writer);
            let server = SOCKET_SERVER.get().expect("socket server not propered initialized");
            tokio::spawn(async move {
                let mut buf = [0; 10240];
                loop {
                    let data = match tokio::time::timeout(timeout, reader.read(&mut buf)).await {
                        Ok(data) => data,
                        Err(_) => {
                            if Player::get_player(addr).map(|player| player.lock().timeout_exempt) == Some(true) { continue; }
                            self.trigger_internal(&addr, srvpru::CtosListenError { error: ListenError::Timeout }).await;
                            break;
                        }
                    };
                    let n = match data {
                        Ok(n) if n == 0 => break,
                        Ok(n) => n,
                        Err(e) => {
                            self.trigger_internal(&addr, srvpru::CtosListenError { error: ListenError::Drop(anyhow::Error::new(e)) }).await;
                            break
                        }
                    };
                    if n > 10240 { 
                        self.trigger_internal(&addr, srvpru::CtosProcessError { error: ProcessorError::Oversize }).await;
                        continue; 
                    }
                    let result = if let Some(player) = Player::get_player(addr) {
                        // Steal the socket, so that player won't be locked.
                        let mut socket = player.lock().server_stream_writer.take();
                        let res = server.ctos_processor.process_multiple_messages(&mut socket, &addr, &buf[0..n]).await;
                        // return the socket, player or socket both may disappear.
                        if let (Some(player), Some(_socket)) = (Player::get_player(addr), socket) {
                            player.lock().server_stream_writer.replace(_socket);
                        }
                        res
                    }
                    else {
                        server.ctos_processor.process_multiple_messages(&mut writer, &addr, &buf[0..n]).await
                    };
                    // Some process happen an error
                    if let Err(error) = result {
                        let break_user = match error { ProcessorError::Abort => true, _ => false };
                        self.trigger_internal(&addr, srvpru::CtosProcessError { error }).await;
                        if break_user { Player::get_player(addr).map(|player| player.lock().expel()); break; }
                    }
                };
                // Out of loop, Drop that player
                {
                    let player = { PLAYERS.write().remove(&addr) };
                    if let Some(player) = player {
                        let player_for_termination = player.clone();
                        self.trigger_internal(&addr, srvpru::PlayerDestroy { player: player_for_termination }).await;
                        if Arc::strong_count(&player) > 4 {
                            let player = player.lock();
                            warn!("Player {} seems still exist reference when drop. This may lead to memory leak.", player);
                        }
                    }
                }
            });
        }
    }


    async fn trigger_internal<S: Struct + MappedStruct>(&'static self, addr: &SocketAddr, obj: S) -> Option<ProcessorError> {
        let buf = [0; 0];
        let mut writer: Option<OwnedWriteHalf> = None;
        let addr = addr.clone();
        let mut obj = Some(Box::new(obj) as Box<dyn Struct>);
        let res = self.internal_processor.process_request(&mut writer, &addr, Some(S::message()), HandlerOccasion::Before, &buf, &mut obj).await.err();
        if res.is_some() { return res; }
        self.internal_processor.process_request(&mut writer, &addr, Some(S::message()), HandlerOccasion::After,  &buf, &mut obj).await.err()
        
        //if let Err(error) = self.internal_processor.process_request(&mut writer, &addr, Some(S::message()), HandlerOccasion::After, &buf, obj).await {
        //    self.trigger_internal(&addr, srvpru::InternalProcessError { error }).await;
        //}
    }
}

pub async fn trigger_internal<S: Struct + MappedStruct>(addr: SocketAddr, obj: S) {
    let server = get_server();
    if let Some(error) = server.trigger_internal(&addr, obj).await {
        server.trigger_internal(&addr, InternalProcessError { error }).await;
    }
}

pub fn trigger_internal_async<S: Struct + MappedStruct>(addr: SocketAddr, obj: S) -> JoinHandle<()> {
    tokio::spawn(async move { trigger_internal(addr, obj).await; })
}

impl std::fmt::Display for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "")?;
        writeln!(f, "srvpru socket server")?;
        writeln!(f, "  CTOS processors:")?;
        write!(f, "{:}", self.ctos_processor)?;
        writeln!(f, "  STOC processors:")?;
        write!(f, "{:}", self.stoc_processor)?;
        writeln!(f, "  INTERNAL processors:")?;
        write!(f, "{:}", self.internal_processor)?;
        Ok(())
    }
}

pub fn get_server() -> &'static Server {
    SOCKET_SERVER.get().expect("Socket server not set")
}

