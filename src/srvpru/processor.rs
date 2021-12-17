// ============================================================
// processor
// ------------------------------------------------------------
//! `processor` provide main process interface for srvpru.
//! 
//! Please refer:
//! - [Handler](crate::srvpru::Handler)
//! - [Context](crate::srvpru::Context)
//! - [Processor](crate::srvpru::Processor)
// ============================================================

use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;

use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use parking_lot::Mutex;
use tokio::io::AsyncWriteExt;
use anyhow::Result;
use tokio::net::tcp::OwnedWriteHalf;

use crate::ygopro::message::stoc;
use crate::ygopro::message::Struct;
use crate::ygopro::message::MappedStruct;
use crate::ygopro::message::Direction;
use crate::ygopro::message::MessageType;
use crate::ygopro::message::try_get_message_type;
use crate::ygopro::message::deserialize_struct_by_type;

use crate::srvpru::CommonError;
use crate::srvpru::message::SRVPRUProcessError;

// ============================================================
//  Handler
// ------------------------------------------------------------
/// Store a process procedure when receive a message.
// ============================================================
pub struct Handler {
    /// **Only for display.** \
    /// Handler name. 
    name: String,
    /// Decide which handler process order. \
    /// Handler with smaller priority process first.
    priority: u8,
    /// **Only for display.** \
    /// Point out which plugin own this handler.
    owner: Option<String>,
    /// Decide how this handler trigger.
    condition: HandlerCondition,
    /// Handler trigger before or after the moment request sent to target. 
    occasion: HandlerOccasion,
    /// What handler actually execute when condition meets. \
    /// Go to [Processor] to get what return value means.
    execution: Box<dyn for <'a, 'b> Fn(&'b mut Context<'a>) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>> + Send + Sync>
}

/// Decide when to check or run a [Handler].
pub enum HandlerOccasion {
    /// Run this handler before message sent to server/client.
    Before,
    /// Run this handler after message sent to server/client.
    After,
    /// Don't run this handler.
    #[allow(dead_code)]
    Never
}

/// Decide if [`Handler`] need to execute.
pub enum HandlerCondition {
    /// Always trigger this handler.
    Always,
    /// Trigger when detect specific message type
    MessageType(MessageType),
    /// Run a function to concern if need to run
    Dynamic(Box<dyn Fn(&mut Context) -> bool + Send + Sync>)
}

impl Handler {
    // ----------------------------------------------------------------------------------------------------
    //  new
    // ----------------------------------------------------------------------------------------------------
    /// Manual create a Handler, point out each property.
    // ----------------------------------------------------------------------------------------------------
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

    // ----------------------------------------------------------------------------------------------------
    //  before_message
    // ----------------------------------------------------------------------------------------------------
    /// Create a handler, execute code before it send to server. \
    /// 
    /// Before `execution` call, wrapper will auto call `deserialize_message`,
    /// and cast message to `S`, steal it from `context` as message.
    /// so you will always see a `None` in `context.message` when executing.
    /// 
    /// If this procedure fail, **nothing will happen**, execution won't call, handler process finished.
    /// 
    /// #### Example
    /// ```
    /// use crate::ygopro::message::ctos::JoinGame;
    /// use crate::srvpru::Handler;
    /// 
    /// Handler::before_message::<JoinGame, _>(100, "foo", |context, message| Box::pin(async move { Ok(false) }));
    /// // also works, but not recommended
    /// Handler::before_message(100, "bar", |context, message: &mut JoinGame| Box::pin(async move { Ok(false) }));
    /// ```
    // ----------------------------------------------------------------------------------------------------
    pub fn before_message<S, F>(priority: u8, name: &str, execution: F) -> Handler 
    where S: Struct + MappedStruct,
          F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b mut S) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>> + Copy,
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

    // ----------------------------------------------------------------------------------------------------
    // follow_message
    // ----------------------------------------------------------------------------------------------------
    /// Create a handler, execute code after message send to server. \
    /// `block_message` in context will be ignored. \
    /// `Error`/`Abort` still take effect. \
    /// If an `Error`/`Abort` happened on before_message handler, all follow-message handlers will be skipped.
    /// 
    /// The only difference between [`before_message`](#method.before_message)
    /// and [`follow_message`](#method.follow_message) is [`HandlerOccasion`]. \
    /// Goto [`before_message`](#method.before_message) for other comment.
    // ----------------------------------------------------------------------------------------------------
    pub fn follow_message<S, F>(priority: u8, name: &str, execution: F) -> Handler 
    where S: Struct + MappedStruct,
          F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b mut S) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>> + Copy,
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

    pub fn before_message_simple<'c, S, F, R>(priority: u8, name: &str, execution: F) -> Handler
    where S: Struct + MappedStruct,
          F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b mut S) -> R + Copy,
          F: Send + Sync + 'static,
          R: Future<Output = ()> + Send {    
        Handler::before_message(priority, name, move |context: &mut Context, _: &mut S| Box::pin(Handler::typed_execute(context, move |context, message| Box::pin(async move{
            execution(context, message);
            Ok(false)
        }))))
    }

    #[doc(hidden)]
    async fn typed_execute<'c, 'd, S, F>(context: &'d mut Context<'c>, execution: F) -> Result<bool>
    where S: Struct,
          F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b mut S) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>>,
          F: Send + Sync + 'static {
            if !context.deserialized { context.deserialize_message(); }
            let mut result = Ok(false);
            if let Some(mut message) = context.message.take() {
                if let Ok(data) = message.downcast_mut::<S>().ok_or(CommonError::IllegalType) {
                    result = Ok(execution(context, data).await?);
                }
                context.message.replace(message);
            }
            result
    }
}

impl HandlerCondition {
    /// Check if current condition meets.
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

lazy_static! {
    /// Handlers simply sorted by name. \
    /// Keep the real [Handler] instances.
    pub static ref HANDLER_LIBRARY: RwLock<HashMap<String, Handler>> = RwLock::new(HashMap::new());
    /// Handlers sorted by plugins. \
    /// Only save handler name.
    pub static ref HANDLER_LIBRARY_BY_PLUGIN: RwLock<HashMap<String, HashMap<Direction, Vec<String>>>> = RwLock::new(HashMap::new());
    /// Handlers' dependency tree. \
    /// Only save handler name.
    pub static ref HANDLER_DEPENDENCIES: RwLock<HashMap<String, Vec<&'static str>>> = RwLock::new(HashMap::new());
}

impl Handler {
    // ----------------------------------------------------------------------------------------------------
    //  register
    // ----------------------------------------------------------------------------------------------------
    /// Register handler to handler library. \
    /// Handler registered by this way can **only** be referenced by set srvpru 
    /// configuration in `ctos_handlers`, `stoc_handlers`, or `srvpru_handlers`
    // ----------------------------------------------------------------------------------------------------
    pub fn register(self) {
        HANDLER_LIBRARY.write().insert(self.name.clone(), self);
    }

    // ----------------------------------------------------------------------------------------------------
    //  register_as
    // ----------------------------------------------------------------------------------------------------
    /// Register handler to handler libray, but set its name first.
    /// 
    /// #### Arguments
    /// * `name`: handler's name.
    // ----------------------------------------------------------------------------------------------------
    pub fn register_as(mut self, name: &str) {
        self.name = name.to_string();
        self.register();
    }

    // ----------------------------------------------------------------------------------------------------
    //  register_for_plugin
    // ----------------------------------------------------------------------------------------------------
    /// Register handler to handler library, and mark it to a plugin. \
    /// This function will evolve Direction by MessageType in [HandlerCondition]. \
    /// **Don't use this method** if Handler use other [HandlerCondition]. 
    /// 
    /// #### Arguments 
    /// * `plugin_name`: plugin's name.
    // ----------------------------------------------------------------------------------------------------
    pub fn register_for_plugin(self, plugin_name: &str) {
        let name = self.name.clone();
        let direction = match self.condition {
            HandlerCondition::MessageType(message) => match message {
                MessageType::STOC(_) => Direction::STOC,
                MessageType::GM(_) => Direction::STOC,
                MessageType::CTOS(_) => Direction::CTOS,
                MessageType::SRVPRU(_) => Direction::SRVPRU,
            },
            _ => { warn!("Can't decide direction for {}/{}, this handler won't be registeded.", plugin_name, name); return; }
        };
        self.register();
        Handler::register_handlers(plugin_name, direction, vec![&name]);
    }

    #[allow(dead_code)]
    pub fn register_handler(name: &str, handler: Handler) {
        HANDLER_LIBRARY.write().insert(name.to_string(), handler);
    }

    
    // ----------------------------------------------------------------------------------------------------
    //  register_handlers
    // ----------------------------------------------------------------------------------------------------
    /// Register several handlers **ALREADY IN LIBRARY** to target plugin.
    /// 
    /// #### Arguments
    /// * plugin_name: plugin's name
    /// * direction: on which direction these handlers should be registered.
    /// * handlers: name of handler's plugins.
    /// 
    /// #### Example
    /// ```
    /// Handler::register_handlers("plugin_name", Direction::CTOS, vec!["my_handler1_name", "my_handler2_name"]);
    /// ```
    // ----------------------------------------------------------------------------------------------------
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

// ====================================================================================================
//  Context
// ----------------------------------------------------------------------------------------------------
/// Holds data during [`Handler`] process.
/// 
/// Fields inner `Context` can be kept across [`HandlerOccasion::Before`] amd [`HandlerOccasion::After`].
// ====================================================================================================
pub struct Context<'a> {
    /// Socket which will point to origin **target**.
    pub socket: &'a mut Option<tokio::net::tcp::OwnedWriteHalf>,
    /// For `CTOS` and `STOC` message, always [`Player`](crate::srvpru::Player)'s address. \
    /// For `SRVPRU` message, it depends on specific message type.
    pub addr: SocketAddr,

    /// Current message direction.
    pub direction: Direction,
    /// If current message has already sent to server/client.
    pub occasion: HandlerOccasion,

    /// Raw message. \
    /// **Contains message type and length.**
    pub message_buffer: &'a [u8],
    /// Message type recorded in header. \
    /// Can be `none` if srvpru can't recognize this message type.
    pub message_type: Option<MessageType>,
    /// Message which already deserialized. \
    /// **Always `None`** before call [`deserialize_message`](Context#method.deserialize_message).
    pub message: Option<Box<dyn Struct>>,

    /// Parameters which passed from handlers before.
    pub parameters: HashMap<&'static str, Box<dyn std::any::Any + Send + Sync>>,
    /// Will be set to `true` if already call [`deserialize_message`](Context#method.deserialize_message).
    pub deserialized: bool,
    /// Set to `true` if want to change [`message`](Context#structfield.message), 
    /// and don't want to manual serialize it to [`message_buffer`](Context#structfield.message_buffer)
    pub reserialize: bool,
    /// Block this message, don't send it to server/client.
    /// For internal message, it can stop error throw.
    /// This is a normal property, won't stop handler process.
    pub block_message: bool,

    #[doc(hidden)]
    pub(super) player: OnceCell<Arc<Mutex<crate::srvpru::Player>>>,
    #[doc(hidden)]
    pub(super) room: OnceCell<Arc<Mutex<crate::srvpru::Room>>>
}

impl<'a> Context<'a> {
    /// Try deserialize [`messsage_buffer`](Context#structfield.message_buffer) to a general [`message`](Context#structfield.message).
    pub fn deserialize_message(&mut self) {
        if self.deserialized { return }
        self.message = self.message_type.map(|_type| deserialize_struct_by_type(_type, &self.message_buffer[3..])).flatten();
        self.deserialized = true
    }    
}


// ====================================================
//  Processor
// ----------------------------------------------------
/// `Processor` is a group of handlers.
// ==================================================== 
pub struct Processor {
    direction: Direction,
    /// Handlers run after message sent to server/client.
    before_handlers: Vec<Handler>,
    /// Handlers run before message sent to server/client.
    after_handlers: Vec<Handler>
}

#[doc(hidden)]
enum ResponseData<'a> {
    NoChange(&'a [u8]),
    Value(Vec<u8>),
    Skip
}

/// Errors happen on processing message.
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
    /// Throw an [`ProcessorError::Abort`] will lead to Server/Player stop listening to socket immediately.
    #[error("Some handler decide to kick the client off.")]
    Abort,
    #[error("After error process, handler decide to kick the client off.")]
    ErrorAbort
}

/// Errors happen on listening to sockets.
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
    #[doc(hidden)]
    pub(super) fn new(direction: Direction) -> Processor {
        Processor { direction, before_handlers: Vec::new(), after_handlers: Vec::new() }
    }

    // ----------------------------------------------------------------------------------------------------
    //  process_multiple_message
    // ----------------------------------------------------------------------------------------------------
    /// Process `CTOS` or `STOC` messages combined in a buffer.
    /// 
    /// #### Arguments
    /// * `socket`: where message should send to.
    /// * `addr`: [Player](crate::srvpru::Player)'s address. 
    /// * `data`: the message which will be send.
    /// 
    /// #### How [Handler] return values affects
    /// * `Ok(false)`: Continue.
    /// * `Ok(true)`: Skip all other handlers after this.
    /// * `Err(_)`: Skip all other handlers after this. tart another run for target direction `ProcessError`. 
    ///    * `block_message` is `true`: This message remain unchanged ignoring any change, send to server/client.
    ///    * `block_message` is `false`: Throw this error out. This mean all message rest inner the 
    /// buffer will be remain unprocessed.
    ///    * [`ProcessorError::Abort`] can also be blocked this way.
    /// 
    /// `Err` means some unexpected scene happen. Be cautious about throwing errors. 
    /// 
    /// #### [Game Message](crate::ygopro::message::gm)
    /// For convenience of processing [`GameMessage`](crate::ygopro::message::stoc::GameMessage), 
    /// an additional round will be added after message processed, with [MessageType::GM].
    /// 
    /// If [`GameMessage`](crate::ygopro::message::stoc::GameMessage) process get an `Error`/`Abort`,
    /// the additional round will be skipped. if an `Err` happen on addtional round, the final result of 
    /// message is `Err`.
    // ----------------------------------------------------------------------------------------------------
    pub async fn process_multiple_messages<'a>(&self, socket: &mut Option<tokio::net::tcp::OwnedWriteHalf>, addr: SocketAddr, data: &'a [u8]) -> core::result::Result<(), ProcessorError> {
        let mut rest_data = data;
        let mut count = 0;
        let mut requests = Vec::new();
        let mut responses = Vec::new();
        let socket_placeholders = typed_arena::Arena::new();
        let mut no_change = true;
        while rest_data.len() > 0 {
            if rest_data.len() < 2 { Err(ProcessorError::BufferLength)?; } 
            else if rest_data.len() < 3 { Err(ProcessorError::ProtoLength)?; }
            let length: u16 = rest_data[0] as u16 + rest_data[1] as u16 * 256;
            if rest_data.len() < 2 + length as usize { Err(ProcessorError::MessageLength)?; }
            let message_buffer = &rest_data[0..(2 + length) as usize];
            let message_type = try_get_message_type(self.direction, message_buffer[2]);
            let socket_placeholder = socket_placeholders.alloc(None);
            let mut context = self.generate_context(socket_placeholder, addr, message_type, HandlerOccasion::Before, message_buffer, None);
            let response_data = match self.process_context(socket, &mut context).await {
                Ok(response) => response,
                Err(error) => if self.process_runtime_error(addr, error).await { ResponseData::NoChange(message_buffer) } else { Err(ProcessorError::ErrorAbort)? },
            };
            no_change = no_change && matches!(response_data, ResponseData::NoChange(_));
            requests.push(context);
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
                        ResponseData::NoChange(actual_data) => socket.write_all(actual_data).await,
                        ResponseData::Value(actual_data) => socket.write_all(&actual_data).await,
                        ResponseData::Skip => Ok(()),
                    } { Err(ProcessorError::FailedToWrite(error.into()))?; } 
                }
            }
        }
        else { trace!("    Socket taken, message don't send to server.") }
        for mut context in requests.into_iter() {
            context.occasion = HandlerOccasion::After;
            self.process_context(socket, &mut context).await?;
        }
        Ok(())
    }

    // ----------------------------------------------------------------------------------------------------
    //  process_internal_message
    // ----------------------------------------------------------------------------------------------------
    /// Process messages which srvpru itself triggers.
    /// 
    /// Only [Handler] with [HandlerOccasion::Before] are processed before this method return.
    /// 
    /// #### Arguments
    /// * `addr`: address decided by message itself.
    /// * `obj`: the message struct need to process 
    /// 
    /// #### Special fields in [Context]
    /// * [`socket`](Context#structfiled.socket) is [None].
    /// * `message_buffer` is always `[0; 0]`.
    /// * `deserialized` is always `true`.
    /// * `reserialize` will be ignored.
    /// * `deserialize_message` never call.
    /// 
    /// #### Error in processing
    /// * Happen in [HandlerOccasion::Before]: Nothing different to normal messages.
    /// * Happen in [HandlerOccasion::After]: Immediately start a [SRVPRUProcessError] with no recursive,
    /// also ignore the `block_message`
    // ----------------------------------------------------------------------------------------------------
    pub async fn process_internal_message<S: Struct + MappedStruct>(&'static self, addr: SocketAddr, obj: S) -> core::result::Result<bool, ProcessorError> {
        let mut socket = None;
        let message_buffer: [u8; 0] = [0; 0];
        let mut context = self.generate_context(&mut socket, addr, Some(S::message()), HandlerOccasion::Before, &message_buffer, Some(Box::new(obj) as Box<dyn Struct>));
        context.deserialized = true;
        let interrupted = self.process_handlers(&mut context).await?;
        let message = context.message;
        let block_message = context.block_message;
        if interrupted { return Ok(block_message); }
        tokio::spawn(async move {
            let mut context = self.generate_context(&mut socket, addr, Some(S::message()), HandlerOccasion::After, &message_buffer, message);
            context.deserialized = true;
            if let Err(error) = self.process_handlers(&mut context).await {
                let message = Some(Box::new(SRVPRUProcessError { error }) as Box<dyn Struct>);
                let mut inner_context = self.generate_context(&mut context.socket, addr, Some(SRVPRUProcessError::message()), HandlerOccasion::Before, &context.message_buffer, message);
                inner_context.deserialized = true;
                if self.process_handlers(&mut inner_context).await.is_ok() {
                    inner_context.occasion = HandlerOccasion::After;
                    self.process_handlers(&mut inner_context).await.ok();
                }
            }
        });
        Ok(block_message)
    }

    fn generate_context<'a>(&self, socket: &'a mut Option<OwnedWriteHalf>, addr: SocketAddr, message_type: Option<MessageType>, occasion: HandlerOccasion, message_buffer: &'a [u8], message: Option<Box<dyn Struct>>) -> Context<'a> {
        Context {
            socket,
            addr,
            direction: self.direction,
            occasion,

            message_buffer,
            message_type,
            message,

            parameters: HashMap::new(),

            deserialized: false,
            reserialize: false,
            block_message: false,

            player: OnceCell::new(),
            room: OnceCell::new()
        }
    }

    async fn process_runtime_error(&self, addr: SocketAddr, error: ProcessorError) -> bool {
        match self.direction {
            Direction::STOC =>   crate::srvpru::server::trigger_internal(addr, crate::srvpru::message::STOCProcessError   { error }).await.map_or(true, |block_message| block_message),
            Direction::CTOS =>   crate::srvpru::server::trigger_internal(addr, crate::srvpru::message::CTOSProcessError   { error }).await.map_or(true, |block_message| block_message),
            Direction::SRVPRU => crate::srvpru::server::trigger_internal(addr, crate::srvpru::message::SRVPRUProcessError { error }).await.map_or(true, |block_message| block_message),
        }
    }

    async fn process_context<'a>(&self, socket: &mut Option<OwnedWriteHalf>, context: &mut Context<'a>) -> core::result::Result<ResponseData<'a>, ProcessorError> {
        // take actual socket into it
        if let Some(actual_socket) = socket.take() { context.socket.replace(actual_socket); }

        // process with handlers
        let interrupted = self.process_handlers(context).await?;

        // Game message add on
        if !interrupted && context.message_type == Some(MessageType::STOC(stoc::MessageType::GameMessage)) {
            // Game message always run a deserialize, or we can't get the type.
            context.deserialize_message();
            if let Some(game_message_general) = context.message.take() {
                let option_game_message = game_message_general.downcast::<stoc::GameMessage>();
                if let Ok(mut game_message) = option_game_message {
                    let child_struct = game_message.message;
                    context.message_type = Some(MessageType::GM(game_message.kind));
                    context.message.replace(child_struct);
                    // Once more
                    self.process_handlers(context).await?;
                    if let Some(message) = context.message.take() {
                        game_message.message = message;
                        context.message.replace(game_message);
                    }
                }
            }
        }

        // return the rent socket.
        if let Some(residual_socket) = context.socket.take() { socket.replace(residual_socket); }
        
        // reserialize the message
        if context.reserialize {
            if let Some(data) = context.message.as_ref() {
                if let Some(message_type) = context.message_type {
                    match bincode::serialize(&**data) {
                        Ok(data) => return Ok(ResponseData::Value(crate::ygopro::message::generate::wrap_data(message_type, &data))),
                        Err(e) => return Err(ProcessorError::FailedToSerialize(e)) 
                    }
                }
                else { warn!("Leave a none message type when try to reserialize") }
            }
            else { return Ok(ResponseData::Skip); }
        }
        
        return if context.block_message { Ok(ResponseData::Skip) } 
               else { Ok(ResponseData::NoChange(context.message_buffer)) }
    }

    async fn process_handlers<'a>(&self, context: &mut Context<'a>) -> core::result::Result<bool, ProcessorError> {
        let handlers = match context.occasion {
            HandlerOccasion::Before => &self.before_handlers,
            HandlerOccasion::After => &self.after_handlers,
            HandlerOccasion::Never => return Ok(false),
        };
        for handler in handlers {
            if handler.condition.meet(context) {
                if (*handler.execution)(context).await.map_err::<ProcessorError, _>(|err| err.into())? {
                    trace!("    {:} decide to break process.", handler.name);
                    return Ok(true)
                }
                trace!("    processed {:}", handler.name);
            }
        }
        Ok(false)
    }

    /// Add a handler to this processor.
    pub fn add_handler(&mut self, handler: Handler) {
        match handler.occasion {
            HandlerOccasion::Before => self.before_handlers.push(handler),
            HandlerOccasion::After => self.after_handlers.push(handler),
            HandlerOccasion::Never => {},
        }
    }

    /// Sort handlers by priority.
    /// Run this method before process.
    pub fn prepare(&mut self) {
        self.before_handlers.sort_by_key(|handler| handler.priority);
        self.after_handlers.sort_by_key(|handler| handler.priority);
    }
}

impl std::fmt::Display for Processor {
    /// Show processor's handlers sorted by Handler condition.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indent = match f.width() {
            Some(length) => std::iter::repeat(" ").take(length).collect::<String>(),
            None => "".to_string()
        };
        let mut conditional_handlers = Vec::new();
        let mut always_handlers = Vec::new();
        let mut typed_handlers = HashMap::new();
        for handler in self.before_handlers.iter() {
            match handler.condition {
                HandlerCondition::Always => &mut always_handlers,
                HandlerCondition::MessageType(message_type) => typed_handlers.entry(message_type).or_insert_with(|| Vec::new()),
                HandlerCondition::Dynamic(_) => &mut conditional_handlers,
            }.push(format!("{}{:}", indent, handler));
        } 
        let mut message_sent: HashMap<MessageType, bool> = HashMap::new();
        for handler in self.after_handlers.iter() {
            match handler.condition {
                HandlerCondition::Always => &mut always_handlers,
                HandlerCondition::MessageType(message_type) => {
                    let reference = typed_handlers.entry(message_type).or_insert_with(|| Vec::new());
                    if reference.len() > 0 && ! message_sent.contains_key(&message_type) { 
                        reference.push(format!("{}--------- MESSAGE SENT ---------", indent));
                        message_sent.insert(message_type, true);
                    }
                    reference
                },
                HandlerCondition::Dynamic(_) => &mut conditional_handlers,
            }.push(format!("{}{:}", indent, handler));
        }
        writeln!(f, "{}Processor [{:?}]", indent, self.direction)?;
        if always_handlers.len() > 0 {
            writeln!(f, "{}  Always trigger", indent)?;
            for handler_description in always_handlers.iter() {
                writeln!(f, "{}  {:}", indent, handler_description)?;
            }
            writeln!(f, "")?;
        }
        if conditional_handlers.len() > 0 {
            writeln!(f, "{}  Custom trigger", indent)?;
            for handler_description in conditional_handlers.iter() {
                writeln!(f, "{}  {:}", indent, handler_description)?;
            }
            writeln!(f, "")?;
        }
        let mut iter = Vec::from_iter(typed_handlers.into_iter());
        iter.sort();
        for (message_type, handler_descriptions) in iter.into_iter() {
            writeln!(f, "{}  [{:}]", indent, message_type)?;
            for handler_description in handler_descriptions {
                writeln!(f, "{}  {:}", indent, handler_description)?
            }
            writeln!(f, "")?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for Processor {
    /// Simply show this processor, without resort.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indent = match f.width() {
            Some(length) => std::iter::repeat(" ").take(length).collect::<String>(),
            None => "".to_string()
        };
        for processor in self.before_handlers.iter() {
            writeln!(f, "{}{:}", indent, processor)?;
        }
        for processor in self.after_handlers.iter() {
            writeln!(f, "{}{:}", indent, processor)?;
        }
        Ok(())
    }
}