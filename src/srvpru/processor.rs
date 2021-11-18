use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::future::Future;

use parking_lot::RwLock;
use tokio::io::AsyncWriteExt;
use anyhow::Result;

use crate::ygopro::message::stoc;
use crate::ygopro::message::Struct;
use crate::ygopro::message::MappedStruct;
use crate::ygopro::message::Direction;
use crate::ygopro::message::MessageType;
use crate::ygopro::message::try_get_message_type;
use crate::ygopro::message::deserialize_struct_by_type;

pub struct Handler {
    name: String,
    priority: u8,
    owner: Option<String>,
    condition: HandlerCondition,
    occasion: HandlerOccasion,
    execution: Box<dyn for <'a, 'b> Fn(&'b mut Context<'a>) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>> + Send + Sync>
}

pub enum HandlerOccasion {
    Before,
    After,
    #[allow(dead_code)]
    Never
}

pub enum HandlerCondition {
    /// Always trigger this handler.
    Always,
    /// Trigger when detect specific message type
    MessageType(MessageType),
    /// Run a function to concern if need to run
    Dynamic(Box<dyn Fn(&mut Context) -> bool + Send + Sync>)
}

impl Handler {
    pub fn new<F>(priority: u8, name: &str, occasion: HandlerOccasion, condition: HandlerCondition, execution: F) -> Handler
    where F: for<'a, 'b> Fn(&'b mut Context<'a>) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>> + Copy,
          F: Send + Sync + 'static {
        Handler {
            name: name.to_string(),
            priority,
            owner: None,
            occasion,
            condition,
            execution: Box::new(execution)
        }
    }

    /// Create a handler, execute code before it send to server.
    pub fn before_message<S, F>(priority: u8, name: &str, execution: F) -> Handler 
    where S: Struct + MappedStruct,
          F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b S) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>> + Copy,
          F: Send + Sync + 'static {
        Handler {
            name: name.to_string(),
            priority,
            owner: None,
            occasion: HandlerOccasion::Before,
            condition: HandlerCondition::MessageType(S::message()),
            execution: Box::new(move |context| Box::pin(Handler::typed_execute::<S, F>(context, execution)))
        }
    }

    /// Create a handler, execute code after message send to server.  
    /// Response in context will be discarded.  
    /// Error/Abort still will take effect.  
    /// If an Error/Abort happened on before_message handler, all follow-message handlers will be skipped.
    pub fn follow_message<S, F>(priority: u8, name: &str, execution: F) -> Handler 
    where S: Struct + MappedStruct,
          F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b S) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>> + Copy,
          F: Send + Sync + 'static {
        Handler {
            name: name.to_string(),
            priority,
            owner: None,
            occasion: HandlerOccasion::After,
            condition: HandlerCondition::MessageType(S::message()),
            execution: Box::new(move |context| Box::pin(Handler::typed_execute::<S, F>(context, execution)))
        }
    }

    #[doc(hidden)]
    async fn typed_execute<'c, 'd, S, F>(context: &'d mut Context<'c>, execution: F) -> Result<bool>
    where S: Struct,
        F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b S) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>>,
        F: Send + Sync + 'static {
            let mut request = context.request.take().ok_or(anyhow!("request already taken."))?;
            let data = request.downcast_mut::<S>().ok_or(anyhow!("request is in wrong type."))?;
            let interrupt = execution(context, data).await?;
            context.request.replace(request);
            Ok(interrupt)
    }
}

impl HandlerCondition {
    #[inline]
    fn meet<'a>(&self, context: &mut Context<'a>) -> bool {
        match self {
            HandlerCondition::Always => true,
            HandlerCondition::MessageType(_type) => context.message_type == Some(*_type),
            HandlerCondition::Dynamic(func) => (*func)(context),
        }
    }
}

impl std::fmt::Display for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:>3}][{:>15}] {}", self.priority, self.owner.as_ref().unwrap_or(&"-".to_string()), self.name)
    }
}

unsafe impl Send for Handler {}
unsafe impl Sync for Handler {}

lazy_static! {
    pub static ref HANDLER_LIBRARY: RwLock<HashMap<String, Handler>> = RwLock::new(HashMap::new());
    pub static ref HANDLER_LIBRARY_BY_PLUGIN: RwLock<HashMap<String, HashMap<Direction, Vec<String>>>> = RwLock::new(HashMap::new());
    pub static ref HANDLER_DEPENDENCIES: RwLock<HashMap<String, Vec<&'static str>>> = RwLock::new(HashMap::new());
}

impl Handler {
    pub fn register(self) {
        HANDLER_LIBRARY.write().insert(self.name.clone(), self);
    }

    pub fn register_as(mut self, name: &str) {
        self.name = name.to_string();
        self.register();
    }

    pub fn register_handler(name: &str, handler: Handler) {
        HANDLER_LIBRARY.write().insert(name.to_string(), handler);
    }

    pub fn register_handlers(plugin_name: &str, direction: Direction, handlers: Vec<&str>) {
        let mut handlers: Vec<String> = handlers.into_iter().map(|_ref| _ref.to_string()).collect();
        let mut handler_library = HANDLER_LIBRARY.write();
        for handler_name in handlers.iter() {
            if let Some(mut handler) = handler_library.get_mut(handler_name) { handler.owner = Some(plugin_name.to_string()); }
            else { warn!("Plugin {} is try to register a unexist handler {}.", plugin_name, handler_name) }
        }

        let mut handler_library = HANDLER_LIBRARY_BY_PLUGIN.write();
        if ! handler_library.contains_key(plugin_name) { handler_library.insert(plugin_name.to_string(), HashMap::new()); }
        let plugin_library = handler_library.get_mut(plugin_name).unwrap();
        if let Some(origin_handlers) = plugin_library.get_mut(&direction) { origin_handlers.append(&mut handlers); }
        else { plugin_library.insert(direction, handlers); }
    }

    pub fn register_dependencies(plugin_name: &str, dependencies: Vec<&'static str>) {
        HANDLER_DEPENDENCIES.write().insert(plugin_name.to_string(), dependencies);
    }
}

pub struct Context<'a> {
    pub socket: &'a mut Option<tokio::net::tcp::OwnedWriteHalf>,
    pub addr: &'a SocketAddr,
    pub request_buffer: &'a [u8],

    pub direction: Direction,
    pub occasion: HandlerOccasion,

    pub message_type: Option<MessageType>,
    pub request: &'a mut Option<Box<dyn Struct>>,
    pub response: Option<Box<dyn Struct>>,
    pub parameters: HashMap<String, String>,
}

pub struct Processor {
    direction: Direction,
    before_handlers: Vec<Handler>,
    after_handlers: Vec<Handler>
}

pub enum ResponseData<'a> {
    Reference(&'a [u8]),
    Value(Vec<u8>),
    Skip
}

#[derive(Error, Debug)]
pub enum ProcessorError {
    #[error("Input data oversize.")]
    Oversize,
    #[error("Input data don't contains buffer length.")]
    BufferLength,
    #[error("Input data don't contains proto length.")]
    ProtoLength,
    #[error("Input data contains wrong message length.")]
    MessageLength,
    #[error("Write to target socket failed.")]
    FailedToWrite(anyhow::Error),
    #[error("Some error happened in processing.")]
    FailedToProcess(anyhow::Error),
    #[error("Failed to serialize a response")]
    FailedToSerialize(Box<bincode::ErrorKind>),
    #[error("Some handler decide to kick the client off.")]
    Abort
}

#[derive(Error, Debug)]
pub enum ListenError {
    #[error("Input data oversize.")]
    Oversize,
    #[error("Waiting for data timeout.")]
    Timeout,
    #[error("Socket listener report an error.")]
    Drop(anyhow::Error),
}

impl core::convert::From<anyhow::Error> for ProcessorError {
    fn from(err: anyhow::Error) -> Self {
        if err.is::<ProcessorError>() { return err.downcast::<ProcessorError>().unwrap(); }
        else { return ProcessorError::FailedToProcess(err); }
    }
}

impl Processor {
    pub fn new(direction: Direction) -> Processor {
        Processor { direction, before_handlers: Vec::new(), after_handlers: Vec::new() }
    }

    pub async fn process_multiple_messages<'a>(&self, socket: &mut Option<tokio::net::tcp::OwnedWriteHalf>, addr: &'a SocketAddr, data: &'a [u8]) -> core::result::Result<(), ProcessorError> {
        let mut rest_data = data;
        let mut count = 0;
        let mut requests = Vec::new();
        let mut responses = Vec::new();
        let mut no_change = true;
        while rest_data.len() > 0 {
            if rest_data.len() < 2 { Err(ProcessorError::BufferLength)?; } 
            else if rest_data.len() < 3 { Err(ProcessorError::ProtoLength)?; }
            let length: u16 = rest_data[0] as u16 + rest_data[1] as u16 * 256;
            if rest_data.len() < 2 + length as usize { Err(ProcessorError::MessageLength)?; }
            let request_buffer = &rest_data[0..(2 + length) as usize];
            let raw_data = &request_buffer[3..];
            let message_type = try_get_message_type(self.direction, request_buffer[2]);
            let mut request = message_type.map(|_type| deserialize_struct_by_type(_type, &raw_data)).flatten();
            let response_data = self.process_request(socket, addr, message_type, HandlerOccasion::Before, request_buffer, &mut request).await?;
            no_change = no_change && matches!(response_data, ResponseData::Reference(_));
            requests.push((message_type, request_buffer, request));
            responses.push(response_data);
            count += 1;
            if count > 1000 { Err(ProcessorError::Oversize)?; }
            rest_data = &rest_data[(2 + length) as usize..]
        }
        if let Some(socket) = socket { 
            // All data no change, just send the origin pack
            if no_change {
                if let Err(error) = socket.write_all(data).await {
                    Err(ProcessorError::FailedToWrite(error.into()))?;
                }
            }
            // Some data changed, send data one-by-one.
            else {
                for response in responses { 
                    if let Err(error) = match response {
                        ResponseData::Reference(actual_data) => socket.write_all(actual_data).await,
                        ResponseData::Value(actual_data) => socket.write_all(&actual_data).await,
                        ResponseData::Skip => Ok(()),
                    } { Err(ProcessorError::FailedToWrite(error.into()))?; } 
                }
            }
        }
        else { trace!("    Socket taken.") }
        for (message_type, request_buffer, mut request) in requests.into_iter() {
            self.process_request(socket, addr, message_type, HandlerOccasion::After, request_buffer, &mut request).await?;
        }
        Ok(())
    }

    pub async fn process_request<'a>(&self, socket: &mut Option<tokio::net::tcp::OwnedWriteHalf>, addr: &'a SocketAddr, message_type: Option<MessageType>, occasion: HandlerOccasion, request_buffer: &'a [u8], request: &mut Option<Box<dyn Struct>>) -> core::result::Result<ResponseData<'a>, ProcessorError> {
        // create context
        let response = Option::None;
        let parameters = HashMap::new();
        let mut context = Context {
            socket,
            addr,
            direction: self.direction,
            request_buffer,
            message_type,
            request,
            occasion,
            response,
            parameters
        };
        // process with handlers
        let mut interrupted = self.process_context(&mut context).await?;

        // Game message add on
        if !interrupted && context.message_type == Some(MessageType::STOC(stoc::MessageType::GameMessage)) {
            if let Some(game_message_general) = context.request.take() {
                let option_game_message = game_message_general.downcast::<stoc::GameMessage>();
                if let Ok(mut game_message) = option_game_message {
                    let child_struct = game_message.message;
                    context.message_type = Some(MessageType::GM(game_message.kind));
                    context.request.replace(child_struct);
                    // Once more, that's ugly
                    interrupted = self.process_context(&mut context).await?;
                    if let Some(message) = context.request.take() {
                        game_message.message = message;
                        context.request.replace(game_message);
                    }
                }
            }
        }
        
        // return response
        match context.response {
            Some(data) => {
                let response = bincode::serialize(&*data).map_err(|err| anyhow!(ProcessorError::FailedToSerialize(err)))?;
                if let Some(message_type) = context.message_type {
                    Ok(ResponseData::Value(crate::ygopro::message::generate::wrap_data(message_type, &response)))
                }
                else { Ok(ResponseData::Skip) }
            },
            None => if interrupted {
                debug!("    Message is blocked.");
                Ok(ResponseData::Skip)
            } else { Ok(ResponseData::Reference(request_buffer)) },
        } 
    }

    async fn process_context<'a>(&self, context: &mut Context<'a>) -> core::result::Result<bool, ProcessorError> {
        let handlers = match context.occasion {
            HandlerOccasion::Before => &self.before_handlers,
            HandlerOccasion::After => &self.after_handlers,
            HandlerOccasion::Never => return Ok(false),
        };
        for _handler in handlers {
            if _handler.condition.meet(context) {
                trace!("    processing {:}", _handler.name);
                if (*_handler.execution)(context).await.map_err::<ProcessorError, _>(|err| err.into())? {
                    return Ok(true)
                }
            }
        }
        Ok(false)
    }

    pub fn add_handler(&mut self, handler: Handler) {
        match handler.occasion {
            HandlerOccasion::Before => self.before_handlers.push(handler),
            HandlerOccasion::After => self.after_handlers.push(handler),
            HandlerOccasion::Never => {},
        }
    }

    pub fn prepare(&mut self) {
        self.before_handlers.sort_by_key(|handler| handler.priority);
        self.after_handlers.sort_by_key(|handler| handler.priority);
    }
}

impl std::fmt::Display for Processor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for processor in self.before_handlers.iter() {
            writeln!(f, "    {:}", processor)?;
        }
        for processor in self.after_handlers.iter() {
            writeln!(f, "    {:}", processor)?;
        }
        Ok(())
    }
}