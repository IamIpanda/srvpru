use std::{collections::HashMap, net::SocketAddr, sync::Weak, sync::Arc, convert::Infallible};
use super::{FromRequest, SimplyContinue, Room};
use once_cell::sync::Lazy;
use parking_lot::{RwLock, Mutex};

pub trait Attachment<S, Key> {
    fn get_static_hash() -> &'static Lazy<RwLock<HashMap<Key, Arc<Mutex<Self>>>>>;
}

trait GetKey<S, Key> {
    fn get_socket(request: &mut super::Bundle<S>) -> Option<Key>;
}

trait PlayerAttachment<S>: Attachment<S, SocketAddr> {}
trait RoomAttachment<S>: Attachment<S, String> {}

impl<S, P> GetKey<S, SocketAddr> for P where P: PlayerAttachment<S> {
    fn get_socket(request: &mut super::Bundle<S>) -> Option<SocketAddr> {
        Some(request.0.socket_addr)
    }
}

impl<S, R> GetKey<S, String> for R where R: RoomAttachment<S> {
    fn get_socket(request: &mut super::Bundle<S>) -> Option<String> {
        Room::get_by_client_addr(&request.0.socket_addr).map(|room| room.lock().origin_name.clone())
    }
}


impl<T, S, Key> FromRequest<S> for Weak<Mutex<T>> where T: Attachment<S, Key> + GetKey<S, Key> + 'static {
    type Rejection = SimplyContinue;
    fn from_request(request: &mut super::Bundle<S>) -> Result<Self, Self::Rejection> {
        match T::get_static_hash().read().get(&T::get_socket(request)) {
            Some(ptr) => Ok(Arc::downgrade(ptr)),
            None => Err(SimplyContinue {})
        }
    }
}
/*
impl<T, S> FromRequest<S> for Arc<Mutex<T>> where T: Attachment + GetSocket<S> + Default + 'static {
    type Rejection = Infallible;
    fn from_request(request: &mut super::Bundle<S>) -> Result<Self, Self::Rejection> {
        Ok(T::get_static_hash().write().entry(T::get_socket(request)).or_insert_with(|| Arc::new(Mutex::new(T::default()))).clone())
    }
}
*/
