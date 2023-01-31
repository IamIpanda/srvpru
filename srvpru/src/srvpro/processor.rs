use std::any::Any;
use std::collections::HashMap;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::net::SocketAddr;

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use parking_lot::RwLock;
use once_cell::sync::Lazy;
use tower::Service;

use ygopro::message::MessageType;
use ygopro::message::Message;
use ygopro::message::UndeserializedBytes;
use ygopro::message::client_to_server;
use ygopro::message::server_to_client;
use ygopro::serde::LengthDescribed;
use ygopro::serde::LengthWrapper;
use ygopro::serde::de::deserialize;
use ygopro::serde::ser::serialize;

use super::*;

static mut PROCESSORS:      Lazy<HashMap<(HandlerOccasion, MessageType), Box<dyn Interceptor + Send + Sync>>>  = Lazy::new(|| HashMap::new());
static     PRE_PROCESSORS:  Lazy<RwLock<HashMap<MessageType, Box<dyn Interceptor + Send + Sync>>>> = Lazy::new(|| RwLock::new(HashMap::new()));
static     POST_PROCESSORS: Lazy<RwLock<HashMap<MessageType, Box<dyn Interceptor + Send + Sync>>>> = Lazy::new(|| RwLock::new(HashMap::new()));

static MESSAGE_TYPE_FOR_ANY: MessageType = MessageType::Other("srvpru", 255);

#[async_trait]
pub trait Interceptor {
    async fn process(self: Box<Self>, socket_addr: SocketAddr, message_buffer: &[u8]) -> Result<Response>;
  
    fn sort(&mut self) -> ();

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clone_box(&self) -> Box<dyn Interceptor + Send + Sync>;
}

impl Clone for Box<dyn Interceptor + Send + Sync> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub struct HandlerGroup<S> {
    handlers: Vec<RegisteredHandler<Bundle<S>, Bundle<S>, Infallible>>,
}

unsafe impl<S: Message + 'static> Sync for HandlerGroup<S> {}

#[async_trait]
impl<S: Message + Send + Sync + 'static> Interceptor for HandlerGroup<S> {
    async fn process(self: Box<Self>, socket_addr: SocketAddr, message_buffer: &[u8]) -> Result<Response> {
        let req = Request::new(socket_addr, message_buffer);
        let state = State::<S>::new();
        let res = Response::new();
        self.process_inner((req, state, res)).await
    }

    fn sort(&mut self) {
        self.handlers.sort_by_key(|handler| handler.priority)
    }

    fn as_any(&self) -> &dyn Any { self } 
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    fn clone_box(&self) -> Box<(dyn Interceptor + Send + Sync)> { Box::new(self.clone()) }
}

impl<S: Message + Send + Sync + 'static> HandlerGroup<S> {
    fn new() -> Self {
        Self {
            handlers: Vec::new()
        }
    }

    fn new_box() -> Box<dyn Interceptor + Send + Sync> {
        Box::new(Self::new())
    }

    async fn process_with_type(self, socket_addr: SocketAddr, message: S) -> Result<Response> {
        let message_buffer = [];
        let req = Request::new(socket_addr, &message_buffer);
        let mut state = State::<S>::new();
        state.set_message(message);
        let res = Response::new();
        self.process_inner((req, state, res)).await
    }

    async fn process_inner(self, mut bundle: Bundle<S>) -> Result<Response> {
        for mut handler in self.handlers {
            trace!("{:?} {}", S::message_type(), handler.name);
            bundle = handler.call(bundle).await.with_context(|| format!("Process failed inner {}", S::message_type()))?;
            if ! bundle.2._continue { break; }
        }
        Ok(bundle.2)
    }

    pub fn get_precursor(occasion: HandlerOccasion) -> &'static mut HandlerGroup<S> {
        unsafe { PROCESSORS.entry((occasion, S::message_type())) }
            .or_insert(Self::new_box())
            .as_any_mut()
            .downcast_mut::<HandlerGroup<S>>()
            .unwrap()
    }

    pub fn register_handler<T: 'static>(&mut self, priority: u8, name: &'static str, occasion: HandlerOccasion, handler: impl Handler<T, S>) {
        self.handlers.push(RegisteredHandler::new(priority, name, occasion, handler));
    }
}

impl<S> Clone for HandlerGroup<S> {
    fn clone(&self) -> Self {
        Self { handlers: self.handlers.clone() }
    }
}

pub fn set_processors() {
    let source_processors = unsafe { PROCESSORS.drain() };
    let mut pre_processors  = PRE_PROCESSORS .write();
    let mut post_precessors = POST_PROCESSORS.write();
    pre_processors.clear();
    post_precessors.clear();
    let mut any_message_pre_processor = None;
    let mut any_message_post_processor = None;
    for ((handler_occasion, message_type), processor) in source_processors {
        if message_type == MESSAGE_TYPE_FOR_ANY {
            match handler_occasion {
                HandlerOccasion::Before => any_message_pre_processor = Some(processor.clone()),
                HandlerOccasion::After => any_message_post_processor = Some(processor.clone()),
                HandlerOccasion::Never => (),
            }
        }
        match handler_occasion {
            HandlerOccasion::Before => pre_processors.insert(message_type, processor),
            HandlerOccasion::After => post_precessors.insert(message_type, processor),
            HandlerOccasion::Never => None,
        };
    }
    if let Some(any_processor) = any_message_pre_processor {
        for (message_type, processor) in pre_processors.iter_mut() {
            if *message_type != MESSAGE_TYPE_FOR_ANY {

            }
        }
    }
    if let Some(any_processor) = any_message_post_processor {
        for (message_type, processor) in pre_processors.iter_mut() {
            if *message_type != MESSAGE_TYPE_FOR_ANY {

            }
        }
    }
}

pub fn get_processor(occasion: HandlerOccasion, message_type: &MessageType) -> Option<Box<dyn Interceptor + Sync + Send>> {
    let hashmap = match occasion {
        HandlerOccasion::Before => &PRE_PROCESSORS,
        HandlerOccasion::After => &POST_PROCESSORS,
        HandlerOccasion::Never => return None,
    };
    let hashmap = hashmap.read();
    hashmap.get(message_type).cloned()
}

pub fn get_processor_with_type<T: Message>(occasion: HandlerOccasion) -> Option<HandlerGroup<T>> {
    let hashmap = match occasion {
        HandlerOccasion::Before => &PRE_PROCESSORS,
        HandlerOccasion::After => &POST_PROCESSORS,
        HandlerOccasion::Never => return None,
    };
    let hashmap = hashmap.read();
    hashmap.get(&T::message_type()).map(|interceptor| interceptor.as_any().downcast_ref::<HandlerGroup<T>>()).flatten().cloned()
}

fn extend_bytes<'a, T>(start: &'a [T], size: usize) -> &'a [T] {
    unsafe {
        std::slice::from_raw_parts(start.as_ptr(), start.len() + size)
    }
}

pub async fn process<'handler, M>(client_addr: SocketAddr, data: &'handler mut [u8]) 
    where M: Into<MessageType> + serde::Deserialize<'handler> + TryFrom<u8> + std::fmt::Debug + 'static,
          SendMarker<M>: DualDirectionSender
{
    let (mut message_cache, mut data) = data.split_at(0);
    let mut socket = get_player_enum(client_addr);
    let messages: Vec<LengthWrapper<UndeserializedBytes<M>>> = match deserialize(data) {
        Ok(m) => m,
        Err(_) => {
            warn!("Cannot deserialize body [{}]: {:?}", client_addr, data);
            socket.write_to_server(data).await.ok(); 
            return 
        }
    };
    for message in messages.into_iter() {
        let size = message.sizeof();
        let message = message.0;
        let message_type = message.message_type.into();
        let mut extend = true;
        if let Some(processor) = get_processor_with_type::<message::AnyMessage>(HandlerOccasion::Before) {
            processor.process_with_type(client_addr, message::AnyMessage { message_type }).await.ok();
        }
        if let Some(processor) = get_processor(HandlerOccasion::Before, &message_type) {
            match processor.process(client_addr, &message.bytes).await {
                Ok(response) => if let Some(body) = response.body {
                    SendMarker::<M>::write(&mut socket, &message_cache).await.ok();
                    SendMarker::<M>::write(&mut socket, &serialize(body.as_ref()).unwrap()).await.ok();
                    (message_cache, data) = data.split_at(message_cache.len() + size);
                    extend = false;
                }, 
                Err(err) => {
                    warn!("{}", err);
                },
            };
        }
        if extend { 
            message_cache = extend_bytes(message_cache, size) 
        }
        if message_type == MessageType::CTOS(ygopro::message::client_to_server::MessageType::JoinGame) {
            socket = get_player_enum(client_addr); 
            debug!("Socket refreshed in {:?}", message_type);
        }
        tokio::spawn(async move {
            if let Some(processor) = get_processor(HandlerOccasion::After, &message_type) {
                processor.process(client_addr, &[]).await.ok();
            }
        });
    }
    if message_cache.len() > 0 {
        SendMarker::<M>::write(&mut socket, message_cache).await.ok();
    }
}

pub async fn process_with_instance<'handler, M: Message + Send + Sync + 'static>(client_addr: SocketAddr, data: M) {
    if let Some(processor) = get_processor_with_type(HandlerOccasion::Before) {
        processor.process_with_type(client_addr, message::AnyMessage { message_type: M::message_type() }).await.ok();
    }
    if let Some(processor) = get_processor_with_type::<M>(HandlerOccasion::Before) {
        processor.process_with_type(client_addr, data).await.ok();
    }
}


#[async_trait]
pub trait DualDirectionSender {
    async fn write(client: &mut (impl PlayerLike + Send), data: &[u8]) -> Result<()>;
}
pub struct SendMarker<T> {
    phantom: PhantomData<T>
}
#[async_trait]
impl DualDirectionSender for SendMarker<client_to_server::MessageType> {
    async fn write(client: &mut (impl PlayerLike + Send), data: &[u8]) -> Result<()> {
        trace!("Write to server {} bytes.", data.len());
        client.write_to_server(data).await 
    }
}
#[async_trait]
impl DualDirectionSender for SendMarker<server_to_client::MessageType> {
    async fn write(client: &mut (impl PlayerLike + Send), data: &[u8]) -> Result<()> {
        trace!("Write to client {} bytes", data.len());
        client.write_to_client(data).await 
    }
}
