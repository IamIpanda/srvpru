use std::sync::Arc;
use std::net::SocketAddr;

use parking_lot::Mutex;

use ygopro::message::PureMessage;

use super::{Player, Room};

 pub struct Request {
    pub socket_addr: SocketAddr,
    pub message_buffer: *mut [u8],
}

impl Request {
    pub fn new<'handler>(socket_addr: SocketAddr, message_buffer: &'handler [u8]) -> Self {
        Request {
            socket_addr,
            message_buffer: message_buffer as *const [u8] as *mut [u8]
        }
    }

    pub fn get_message_buffer<'inner>(&self) -> Option<&'inner [u8]> {
        unsafe {
            self.message_buffer.as_ref()
        }
    }
}

unsafe impl Send for Request {}
unsafe impl Sync for Request {}

pub struct Response {
    pub body: Option<Box<dyn PureMessage + Send + Sync>>,
    pub _continue: bool,
    pub error: Option<Box<dyn std::error::Error + Send + Sync>>
}

impl Response {
    pub fn new() -> Self {
        Response {
            body: None,
            _continue: true,
            error: None
        }
    }

    pub fn normal(&self) -> bool {
        matches!(self.body, None) && matches!(self.error, None) && self._continue
    }
}

pub struct State<S> {
    message: Option<S>,
    pub player: Option<Arc<Mutex<Player>>>,
    pub room: Option<Arc<Mutex<Room>>>
}

impl<S> State<S> {
    pub fn new() -> Self {
        State {
            message: None,
            player: None,
            room: None
        }
    }

    pub fn set_message_deserialize<'de>(&mut self, request: &Request) where S: serde::de::Deserialize<'de> {
        if matches!(self.message, None) {
            if let Some(message_buffer) = unsafe { request.message_buffer.as_ref() } {
                self.message = ygopro::serde::de::deserialize(message_buffer).ok();
            } 
        }
    }

    pub fn set_message(&mut self, message: S) {
        if matches!(self.message, None) {
            self.message = Some(message)
        }
    }

    pub fn get_message<'inner>(&self) -> Option<&'inner S> {
        self.message.as_ref().map(|m| unsafe { (m as *const S).as_ref() }).flatten()
    }

    pub fn get_message_mut<'inner>(&mut self) -> Option<&'inner mut S> {
        self.message.as_mut().map(|m| unsafe { (m as *mut S).as_mut() }).flatten()
    }
}

unsafe impl<S> Send for State<S> {}
unsafe impl<S> Sync for State<S> {}

pub type Bundle<S> = (Request, State<S>, Response);

