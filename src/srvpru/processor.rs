use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::future::Future;
use parking_lot::RwLock;
use tokio::io::AsyncWriteExt;

use crate::ygopro::message::deserialize_component;
use crate::ygopro::message::get_message_type;
use crate::ygopro::message::Direction;
use crate::ygopro::message::MessageType;
use crate::ygopro::message::Struct;

pub struct Handler {
    pub priority: u8,
    pub condition: Box<dyn Fn(&mut Context) -> bool + Send + Sync>,
    pub execution: Box<dyn for <'a, 'b> Fn(&'b mut Context<'a>) -> Pin<Box<dyn Future<Output = bool> + Send + 'b>> + Send + Sync>
}

impl Handler {
    pub fn new<T>(priority: u8, condition: fn(&mut Context) -> bool, execution: T) -> Handler
    where T: for<'a, 'b> Fn(&'b mut Context<'a>) -> Pin<Box<dyn Future<Output = bool> + Send + 'b>>,
          T: Send + Sync + 'static {
        Handler {
            priority,
            condition: Box::new(condition),
            execution: Box::new(execution)
        }
    }

    pub fn follow_message<T, F>(priority: u8, message_type: MessageType, execution: F) -> Handler 
    where T: Struct,
          F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b T) -> Pin<Box<dyn Future<Output = bool> + Send + 'b>> + Copy,
          F: Send + Sync + 'static {
        Handler {
            priority,
            condition: Box::new(move |context| Some(message_type) == context.message_type),
            execution: Box::new(move |context| Box::pin(Handler::typed_execute::<T, F>(context, execution)))
        }
    }

    async fn typed_execute<'c, 'd, T, F>(context: &'d mut Context<'c>, execution: F) -> bool
    where T: Struct,
          F: for<'a, 'b> Fn(&'b mut Context<'a>, &'b T) -> Pin<Box<dyn Future<Output = bool> + Send + 'b>>,
          F: Send + Sync + 'static {
            let mut request = context.request.take().unwrap();
            let interrupt = execution(context, request.downcast_mut::<T>().unwrap()).await;
            context.request.replace(request);
            interrupt
    }
}

unsafe impl Send for Handler {}
unsafe impl Sync for Handler {}

lazy_static! {
    pub static ref HANDLER_LIBRARY: RwLock<HashMap<String, Handler>> = RwLock::new(HashMap::new());
    pub static ref HANDLER_LIBRARY_BY_PLUGIN: RwLock<HashMap<String, Vec<Handler>>> = RwLock::new(HashMap::new());
}

impl Handler {
    pub fn register_handler(name: &str, handler: Handler) {
        HANDLER_LIBRARY.write().insert(name.to_string(), handler);
    }

    pub fn register_handlers(plugin_name: &str, handlers: Vec<Handler>) {
        HANDLER_LIBRARY_BY_PLUGIN.write().insert(plugin_name.to_string(), handlers);
    }
}

pub struct Context<'a> {
    pub socket: &'a mut Option<tokio::net::tcp::OwnedWriteHalf>,
    pub addr: &'a SocketAddr,
    pub request_buffer: &'a [u8],

    pub current_direction: Direction,
    pub message_type: Option<MessageType>,
    pub request: Option<Box<dyn Struct>>,
    pub response: Option<Box<dyn Struct>>,
    pub parameters: HashMap<String, String>,
}

impl<'a> Context<'a> {
    pub fn cast_request_to_type<F>(&mut self) -> Option<&mut F> where F: Struct {
        let _box = self.request.as_mut()?;
        _box.downcast_mut::<F>()
    }
}

pub struct Processor {
    pub direction: Direction,
    pub handlers: Vec<Handler>
}

pub enum ResponseData<'a> {
    Reference(&'a [u8]),
    Value(Vec<u8>),
    Skip
}

pub enum ProcesorError {
    Oversize,
    BufferLength,
    ProtoLength,
    MessageLength,
    FailedToWrite
}

impl Processor {
    pub async fn process_multiple<'a>(&self, socket: &mut Option<tokio::net::tcp::OwnedWriteHalf>, addr: &'a SocketAddr, data: &'a [u8]) -> Option<ProcesorError> {
        let mut rest_data = data;
        let mut count = 0;
        let mut datas: Vec<ResponseData<'a>> = Vec::new();
        let mut error: Option<ProcesorError> = None;
        let mut no_change = true;
        while rest_data.len() > 0 {
            if rest_data.len() < 2 {
                error = Some(ProcesorError::BufferLength);
                break
            } else if rest_data.len() < 3 {
                error = Some(ProcesorError::ProtoLength);
                break
            }
            let length: u16 = rest_data[0] as u16 + rest_data[1] as u16 * 256;
            if rest_data.len() < 2 + length as usize {
                error = Some(ProcesorError::MessageLength);
                break
            }
            let child_data = &rest_data[0..(2 + length) as usize];
            let current_data = self.process(socket, addr, child_data).await;
            no_change = no_change && matches!(current_data, ResponseData::Reference(_));
            datas.push(current_data);
            count += 1;
            if count > 1000 {
                error = Some(ProcesorError::Oversize);
                break
            }
            rest_data = &rest_data[(2 + length) as usize..]
            
        }
        if let Some(socket) = socket { 
            // All data no change, just send the origin pack
            if no_change {
                if let Err(_) = socket.write_all(data).await {
                    error = Some(ProcesorError::FailedToWrite);
                }
            }
            // Some data changed, send data one-by-one.
            else {
                for data in datas { 
                    if match data {
                        ResponseData::Reference(actual_data) => socket.write_all(actual_data).await,
                        ResponseData::Value(actual_data) => socket.write_all(&actual_data).await,
                        ResponseData::Skip => Ok(()),
                    }.is_err() {
                        error = Some(ProcesorError::FailedToWrite); 
                    }
                }
            }
        }
        return error;
    }

    async fn process<'a>(&self, socket: &mut Option<tokio::net::tcp::OwnedWriteHalf>, addr: &'a SocketAddr, request_buffer: &'a [u8]) -> ResponseData<'a> {
        // read header
        let message_type = get_message_type(self.direction, request_buffer[2]);
        let data = &request_buffer[3..];
        // deserialize item
        let request = match message_type {
            Some(actual_message_type) => deserialize_component(actual_message_type, &data),
            Option::None => Option::None
        };
        // create context
        let response = Option::None;
        let parameters = HashMap::new();
        let mut context = Context {
            socket,
            addr,
            current_direction: self.direction,
            request_buffer,
            message_type,
            request,
            response,
            parameters
        };
        // process with handlers
        let mut interrupted = false;
        for _handler in &self.handlers {
            if (*_handler.condition)(&mut context) && (*_handler.execution)(&mut context).await {
                interrupted = true;
                break
            }
        }
        // return response
        match context.response {
            Some(data) => ResponseData::Value(bincode::serialize(&*data).unwrap()),
            None => if interrupted { ResponseData::Skip } else { ResponseData::Reference(request_buffer) },
        } 
    }

    pub fn sort(&mut self) {
        self.handlers.sort_by_key(|handler| handler.priority)
    }
}

/*
0040         2c 00 01 06 01 04 04 00 00 00 04 00 00 00
0050   10 00 00 00 03 00 00 00 75 38 59 01 01 04 02 04
0060   04 00 00 00 04 00 00 00 04 00 00 00 04 00 00 00


0070   30 00 01 06 00 08 04 00 00 00 04 00 00 00 10 00
0080   00 00 03 00 00 00 2d 31 de 04 00 08 02 05 04 00
0090   00 00 04 00 00 00 04 00 00 00 04 00 00 00 04 00
00a0   00 00 

             24 00 01 06 01 08 04 00 00 00 04 00 00 00
00b0   04 00 00 00 04 00 00 00 04 00 00 00 04 00 00 00
00c0   04 00 00 00 04 00 00 00 

                               24 00 01 06 00 02 10 00
00d0   00 00 00 00 00 00 00 00 00 00 00 00 00 00 10 00
00e0   00 00 00 00 00 00 00 00 00 00 00 00 00 00 

                                                 54 00
00f0   01 06 01 02 10 00 00 00 03 00 00 00 75 38 59 01
0100   01 02 00 0a 10 00 00 00 03 00 00 00 85 06 e5 01
0110   01 02 01 0a 10 00 00 00 03 00 00 00 9f da e1 02
0120   01 02 02 0a 10 00 00 00 03 00 00 00 de 32 ea 05
0130   01 02 03 0a 10 00 00 00 03 00 00 00 ac b1 ea 05
0140   01 02 04 0a 

                   03 00 01 28 00 

                                  04 00 01 29 01 00 

                                                    2c
0150   00 01 06 00 04 10 00 00 00 03 00 00 00 dc fb 43
0160   04 00 04 00 01 04 00 00 00 04 00 00 00 04 00 00
0170   00 04 00 00 00 04 00 00 00 04 00 00 00 

                                              2c 00 01
0180   06 01 04 04 00 00 00 04 00 00 00 10 00 00 00 03
0190   00 00 00 75 38 59 01 01 04 02 04 04 00 00 00 04
01a0   00 00 00 04 00 00 00 04 00 00 00 

                                        30 00 01 06 00
01b0   08 04 00 00 00 04 00 00 00 10 00 00 00 03 00 00
01c0   00 2d 31 de 04 00 08 02 05 04 00 00 00 04 00 00
01d0   00 04 00 00 00 04 00 00 00 04 00 00 00 

                                              24 00 01
01e0   06 01 08 04 00 00 00 04 00 00 00 04 00 00 00 04
01f0   00 00 00 04 00 00 00 04 00 00 00 04 00 00 00 04
0200   00 00 00 

                24 00 01 06 00 02 10 00 00 00 00 00 00
0210   00 00 00 00 00 00 00 00 00 10 00 00 00 00 00 00
0220   00 00 00 00 00 00 00 00 00 

                                  54 00 01 06 01 02 10
0230   00 00 00 03 00 00 00 75 38 59 01 01 02 00 0a 10
0240   00 00 00 03 00 00 00 85 06 e5 01 01 02 01 0a 10
0250   00 00 00 03 00 00 00 9f da e1 02 01 02 02 0a 10
0260   00 00 00 03 00 00 00 de 32 ea 05 01 02 03 0a 10
0270   00 00 00 03 00 00 00 ac b1 ea 05 01 02 04 0a 

                                                    08
0280   00 01 5a 00 01 00 00 00 00 

                                  02 00 01 03 

                                              05 00 18
0290   00 7f e7 03
*/