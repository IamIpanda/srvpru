use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::future::Future;

use parking_lot::RwLock;
use tokio::io::AsyncWriteExt;
use anyhow::Result;

use crate::ygopro::message::MappedStruct;
use crate::ygopro::message::STOCMessageType;
use crate::ygopro::message::deserialize_struct_by_type;
use crate::ygopro::message::get_message_type;
use crate::ygopro::message::Direction;
use crate::ygopro::message::MessageType;
use crate::ygopro::message::Struct;

pub struct Handler {
    pub name: String,
    pub priority: u8,
    pub owner: Option<String>,
    pub condition: HandlerCondition,
    pub execution: Box<dyn for <'a, 'b> Fn(&'b mut Context<'a>) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>> + Send + Sync>
}

pub enum HandlerCondition {
    Always,
    MessageType(MessageType),
    Dynamic(Box<dyn Fn(&mut Context) -> bool + Send + Sync>)
}

impl Handler {
    pub fn new<F1, F2>(priority: u8, name: &str, condition: F1, execution: F2) -> Handler
    where F1: for<'a, 'b> Fn(&'b mut Context<'a>) -> bool,
          F1: Send + Sync + 'static,
          F2: for<'a, 'b> Fn(&'b mut Context<'a>) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>> + Copy,
          F2: Send + Sync + 'static {
        Handler {
            name: name.to_string(),
            priority,
            owner: None,
            condition: HandlerCondition::Dynamic(Box::new(condition)),
            execution: Box::new(execution)
        }
    }

    pub fn follow_message<S, F>(priority: u8, name: &str, execution: F) -> Handler 
    where S: Struct + MappedStruct,
          F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b S) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>> + Copy,
          F: Send + Sync + 'static {
        Handler {
            name: name.to_string(),
            priority,
            owner: None,
            condition: HandlerCondition::MessageType(S::message()),
            execution: Box::new(move |context| Box::pin(Handler::typed_execute::<S, F>(context, execution)))
        }
    }


    async fn typed_execute<'c, 'd, S, F>(context: &'d mut Context<'c>, execution: F) -> Result<bool>
    where S: Struct,
        F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b S) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>>,
        F: Send + Sync + 'static {
            let mut request = context.request.take().unwrap();
            let interrupt = execution(context, request.downcast_mut::<S>().unwrap()).await;
            context.request.replace(request);
            interrupt
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
    pub static ref HANDLER_LIBRARY_BY_PLUGIN: RwLock<HashMap<String, HashMap<Direction, Vec<&'static str>>>> = RwLock::new(HashMap::new());
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

    pub fn register_handlers(plugin_name: &str, direction: Direction, mut handlers: Vec<&'static str>) {
        let mut handler_library = HANDLER_LIBRARY.write();
        for handler_name in handlers.iter() {
            if let Some(mut handler) = handler_library.get_mut(*handler_name) { handler.owner = Some(plugin_name.to_string()); }
            else { warn!("Plugin {} is try to register a unexist handler {}.", plugin_name, handler_name) }
        }

        let mut handler_library = HANDLER_LIBRARY_BY_PLUGIN.write();
        if ! handler_library.contains_key(plugin_name) { handler_library.insert(plugin_name.to_string(), HashMap::new()); }
        let plugin_library = handler_library.get_mut(plugin_name).unwrap();
        if let Some(origin_handlers) = plugin_library.get_mut(&direction) { origin_handlers.append(&mut handlers); }
        else { plugin_library.insert(direction, handlers); }
    }
}

pub struct Context<'a> {
    pub socket: &'a mut Option<tokio::net::tcp::OwnedWriteHalf>,
    pub addr: &'a SocketAddr,
    pub request_buffer: &'a [u8],

    pub direction: Direction,
    pub message_type: Option<MessageType>,
    pub request: Option<Box<dyn Struct>>,
    pub response: Option<Box<dyn Struct>>,
    pub parameters: HashMap<String, String>,
}

pub struct Processor {
    pub direction: Direction,
    pub handlers: Vec<Handler>,
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
    #[error("Waiting for data timeout.")]
    Timeout,
    #[error("Socket listen report an error.")]
    Drop(anyhow::Error),
    #[error("Some handler decide to kick the client off.")]
    Abort
}

impl core::convert::From<anyhow::Error> for ProcessorError {
    fn from(err: anyhow::Error) -> Self {
        if err.is::<ProcessorError>() { return err.downcast::<ProcessorError>().unwrap(); }
        else { return ProcessorError::FailedToProcess(err); }
    }
}

impl Processor {
    pub async fn process_multiple_messages<'a>(&self, socket: &mut Option<tokio::net::tcp::OwnedWriteHalf>, addr: &'a SocketAddr, data: &'a [u8]) -> core::result::Result<(), ProcessorError> {
        let mut rest_data = data;
        let mut count = 0;
        let mut datas: Vec<ResponseData<'a>> = Vec::new();
        let mut no_change = true;
        while rest_data.len() > 0 {
            if rest_data.len() < 2 { Err(ProcessorError::BufferLength)?; } 
            else if rest_data.len() < 3 { Err(ProcessorError::ProtoLength)?; }
            let length: u16 = rest_data[0] as u16 + rest_data[1] as u16 * 256;
            if rest_data.len() < 2 + length as usize { Err(ProcessorError::MessageLength)?; }
            let child_data = &rest_data[0..(2 + length) as usize];

            let current_data = self.process_message(socket, addr, child_data).await?;
            no_change = no_change && matches!(current_data, ResponseData::Reference(_));
            datas.push(current_data);
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
                for data in datas { 
                    if let Err(error) = match data {
                        ResponseData::Reference(actual_data) => socket.write_all(actual_data).await,
                        ResponseData::Value(actual_data) => socket.write_all(&actual_data).await,
                        ResponseData::Skip => Ok(()),
                    } { Err(ProcessorError::FailedToWrite(error.into()))?; } 
                }
            }
        }
        else { trace!("    Socket taken.") }
        Ok(())
    }

    async fn process_message<'a>(&self, socket: &mut Option<tokio::net::tcp::OwnedWriteHalf>, addr: &'a SocketAddr, request_buffer: &'a [u8]) -> core::result::Result<ResponseData<'a>, ProcessorError> {
        // read header
        let message_type = get_message_type(self.direction, request_buffer[2]);
        let data = &request_buffer[3..];
        // deserialize item
        let request = match message_type {
            Some(actual_message_type) => deserialize_struct_by_type(actual_message_type, &data),
            Option::None => Option::None
        };
        self.process_request(socket, addr, message_type, request_buffer, request).await
    }

    pub async fn process_request<'a>(&self, socket: &mut Option<tokio::net::tcp::OwnedWriteHalf>, addr: &'a SocketAddr, message_type: Option<MessageType>, request_buffer: &'a [u8], request: Option<Box<dyn Struct>>) -> core::result::Result<ResponseData<'a>, ProcessorError> {
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
            response,
            parameters
        };
        // process with handlers
        let mut interrupted = self.process_context(&mut context).await?;

        // Game message add on
        if !interrupted && context.message_type == Some(MessageType::STOC(STOCMessageType::GameMessage)) {
            if let Some(game_message_general) = context.request {
                let option_game_message = game_message_general.downcast::<crate::ygopro::message::STOCGameMessage>();
                if let Ok(game_message) = option_game_message {
                    context.message_type = Some(MessageType::GM(game_message.kind));
                    context.request = Some(game_message.message);
                    // Once more, that's ugly
                    interrupted = self.process_context(&mut context).await?;
                }
            }
        }
        
        // return response
        match context.response {
            Some(data) => Ok(ResponseData::Value(crate::ygopro::message::wrap_data(context.message_type.as_ref().unwrap(), &bincode::serialize(&*data).unwrap()))),
            None => if interrupted {
                trace!("    Message is blocked.");
                Ok(ResponseData::Skip)
            } else { Ok(ResponseData::Reference(request_buffer)) },
        } 
    }

    async fn process_context<'a>(&self, context: &mut Context<'a>) -> core::result::Result<bool, ProcessorError> {
        for _handler in &self.handlers {
            if _handler.condition.meet(context) {
                trace!("    processing {:}", _handler.name);
                if (*_handler.execution)(context).await.map_err::<ProcessorError, _>(|err| err.into())? {
                    return Ok(true)
                }
            }
        }
        Ok(false)
    }

    pub fn prepare(&mut self) {
        self.handlers.sort_by_key(|handler| handler.priority)
    }
}
