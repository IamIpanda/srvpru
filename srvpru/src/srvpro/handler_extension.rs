use std::{convert::Infallible, task::Poll};

use tower::{Service, util::BoxCloneService};

use crate::srvpro::{Bundle, Handler};

/// Decide when to check or run a [Handler].
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum HandlerOccasion {
    /// Run this handler before message sent to server/client.
    Before,
    /// Run this handler after message sent to server/client.
    After,
    /// Don't run this handler.
    #[allow(dead_code)]
    Never
}

pub struct RegisteredHandler<T, U, E> {
    pub service: BoxCloneService<T, U, E>,
    pub priority: u8,
    pub name: &'static str,
    pub occasion: HandlerOccasion,
}

impl<S: Sync + Send + 'static> RegisteredHandler<Bundle<S>, Bundle<S>, Infallible> {
    pub fn new<T: 'static>(priority: u8, name: &'static str, occasion: HandlerOccasion, handler: impl Handler<T, S>) -> Self {
        Self {
            priority,
            name,
            occasion,
            service: handler.into_service().into_box_clone_service()
        }
    }
}

impl<T, U, E> Service<T> for RegisteredHandler<T, U, E> {
    type Response = U;
    type Error = E;
    type Future = futures::future::BoxFuture<'static, Result<U, E>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: T) -> Self::Future {
        self.service.call(req)
    }
}

impl<T, U, E> Clone for RegisteredHandler<T, U, E> {
    fn clone(&self) -> Self {
        Self { 
            service: self.service.clone(), 
            priority: self.priority, 
            name: self.name,
            occasion: self.occasion
        }
    }
}
