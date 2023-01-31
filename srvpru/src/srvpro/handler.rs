// RefService is axum::RefService<T, State>

use std::pin::Pin; 
use std::future::Future;
use std::task::Poll;
use std::marker::PhantomData; 
use std::convert::Infallible;
use std::net::SocketAddr;

use futures::FutureExt;
use tower::{Service, util::BoxCloneService};

use ygopro::message::Message;

use super::{Bundle, Response, Request, State};

pub trait Handler<T, S>: Clone + Send + Sized + 'static {
    type Future: Future<Output = Bundle<S>> + Send;
    fn call(self, req: Bundle<S>) -> Self::Future;

    fn into_service(self) -> HandlerService<Self, T, S> {
        HandlerService::new(self)
    }
}

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

pub struct MultipleParameetrNonSenseGenericParameter;

pub trait FromRequest<S, M = MultipleParameetrNonSenseGenericParameter>: Sized {
    type Rejection: IntoResponse + Sized;
    fn from_request(request: &mut Bundle<S>) -> Result<Self, Self::Rejection>;
}

impl<F, Fut, Res, S> Handler<((),), S> for F
where
    F: FnOnce() -> Fut + Clone + Send + 'static,
    Fut: Future<Output = Res> + Send,
    Res: IntoResponse,
    S: 'static
{
    type Future = Pin<Box<dyn Future<Output = Bundle<S>> + Send>>;

    fn call(self, req: Bundle<S>) -> Self::Future {
        Box::pin(async move {
            let response_priority = self().await;
            (req.0, req.1, response_priority.into_response())
        })
    }
}

impl<F, Res, S> Handler<Option<()>, S> for F
where
    F: FnOnce() -> Res + Clone + Send + 'static,
    Res: IntoResponse,
    S: 'static
{
    type Future = Pin<Box<dyn Future<Output = Bundle<S>> + Send>>;

    fn call(self, req: Bundle<S>) -> Self::Future {
        Box::pin(async move {
            let response_priority = self();
            (req.0, req.1, response_priority.into_response())
        })
    }
}

macro_rules! impl_handler {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[allow(non_snake_case, unused_mut)]
        impl<F, Fut, S, Res, M, $($ty,)* $last> Handler<(M, $($ty,)* $last,), S> for F
        where
            F: FnOnce($($ty,)* $last,) -> Fut + Clone + Send + 'static,
            Fut: Future<Output = Res> + Send,
            S: Send + Sync + 'static,
            Res: IntoResponse,
            $( $ty: FromRequest<S, M> + Send, )*
            $last: FromRequest<S, M> + Send,
        {
            type Future = Pin<Box<dyn Future<Output = Bundle<S>> + Send>>;

            fn call(self, mut req: Bundle<S>) -> Self::Future {
                Box::pin(async move {
                    $(
                        let $ty = match $ty::from_request(&mut req) {
                            Ok(value) => value,
                            Err(rejection) => return (req.0, req.1, rejection.into_response()),
                        };
                    )*

                    let $last = match $last::from_request(&mut req) {
                        Ok(value) => value,
                        Err(rejection) => return (req.0, req.1, rejection.into_response()),
                    };

                    let res = self($($ty,)* $last,).await;

                    (req.0, req.1, res.into_response()) 
                })
            }
        }

        #[allow(non_snake_case, unused_mut)]
        impl<F, S, Res, M, $($ty,)* $last> Handler<Option<(M, $($ty,)* $last,)>, S> for F
        where
            F: FnOnce($($ty,)* $last,) -> Res + Clone + Send + 'static,
            S: Send + Sync + 'static,
            Res: IntoResponse,
            $( $ty: FromRequest<S, M> + Send, )*
            $last: FromRequest<S, M> + Send,
        {
            type Future = Pin<Box<dyn Future<Output = Bundle<S>> + Send>>;

            fn call(self, mut req: Bundle<S>) -> Self::Future {
                Box::pin(async move {
                    $(
                        let $ty = match $ty::from_request(&mut req) {
                            Ok(value) => value,
                            Err(rejection) => return (req.0, req.1, rejection.into_response()),
                        };
                    )*

                    let $last = match $last::from_request(&mut req) {
                        Ok(value) => value,
                        Err(rejection) => return (req.0, req.1, rejection.into_response()),
                    };

                    let res = self($($ty,)* $last,);

                    (req.0, req.1, res.into_response()) 
                })
            }
        }
    };
}


impl_handler!([], T1);
impl_handler!([T1], T2);
impl_handler!([T1, T2], T3);
impl_handler!([T1, T2, T3], T4);
impl_handler!([T1, T2, T3, T4], T5);
impl_handler!([T1, T2, T3, T4, T5], T6);
impl_handler!([T1, T2, T3, T4, T5, T6], T7);
impl_handler!([T1, T2, T3, T4, T5, T6, T7], T8);
impl_handler!([T1, T2, T3, T4, T5, T6, T7, T8], T9);
impl_handler!([T1, T2, T3, T4, T5, T6, T7, T8, T9], T10);
impl_handler!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10], T11);
impl_handler!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11], T12);
impl_handler!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12], T13);
impl_handler!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13], T14);
impl_handler!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14], T15);
impl_handler!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15], T16);

impl IntoResponse for std::convert::Infallible {
    fn into_response(self) -> Response {
        Response::new()
    }
}

pub struct SimplyContinue;

impl IntoResponse for SimplyContinue {
    fn into_response(self) -> Response {
        Response::new()
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response::new()
    } 
}

impl IntoResponse for bool {
    fn into_response(self) -> Response {
        Response { body: None, _continue: self, error: None }
    }
}

/*
impl<E: std::error::Error + Sync + Send + 'static> IntoResponse for Result<bool, E> {
    fn into_response(self) -> Response {
        match self {
            Ok(_continue) => Response { body: None, _continue, error: None },
            Err(error) => Response { body: None, _continue: false, error: Some(Box::new(error) as Box<dyn std::error::Error + Send + Sync>)},
        }
    }
}

impl<E: std::error::Error + Sync + Send + 'static> IntoResponse for Result<(), E> {
    fn into_response(self) -> Response {
        match self {
            Ok(_) => Response { body: None, _continue: true, error: None },
            Err(error) => Response { body: None, _continue: false, error: Some(Box::new(error) as Box<dyn std::error::Error + Send + Sync>) },
        }
    }
}
*/

impl IntoResponse for anyhow::Result<()> {
    fn into_response(self) -> Response {
        match self {
            Ok(_) => Response { body: None, _continue: true, error: None },
            Err(error) => Response { body: None, _continue: true, error: Some(error.into()) }
        }
    }
}

impl IntoResponse for anyhow::Result<bool> {
    fn into_response(self) -> Response {
        match self {
            Ok(_continue) => Response { body: None, _continue, error: None },
            Err(error) => Response { body: None, _continue: true, error: Some(error.into()) }
        }
    }
}
/* 
impl<S: Message> IntoResponse for anyhow::Result<S> {
    fn into_response(self) -> Response {
        match self {
            Ok(_) => Response { body: None, _continue: true, error: None },
            Err(error) => Response { body: None, _continue: true, error: Some(error.into()) }
        }
    }
}

impl<S: Message + Send + Sync + 'static> IntoResponse for S {
    fn into_response(self) -> Response {
        Response { body: Some(Box::new(self) as Box<dyn Message + Send + Sync>), _continue: true, error: None }
    }
}
*/

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl<'inner, S> FromRequest<S> for &'inner S where S: Message + serde::Deserialize::<'inner> {
    type Rejection = SimplyContinue;
    fn from_request(request: &mut Bundle<S>) -> Result<Self, Self::Rejection> {
        request.1.set_message_deserialize(&request.0);
        request.1.get_message().ok_or(SimplyContinue {})
    }
}

impl<'inner, S> FromRequest<S> for &'inner mut S where S: Message + serde::Deserialize::<'inner> {
    type Rejection = SimplyContinue;
    fn from_request(request: &mut Bundle<S>) -> Result<Self, Self::Rejection> {
        request.1.set_message_deserialize(&request.0);
        request.1.get_message_mut().ok_or(SimplyContinue {})
    }
}

impl<S> FromRequest<S> for SocketAddr {
    type Rejection = Infallible;

    fn from_request(request: &mut Bundle<S>) -> Result<Self, Self::Rejection> {
        Ok(request.0.socket_addr)
    }
}

impl<S> FromRequest<S> for Vec<u8> {
    type Rejection = SimplyContinue;

    fn from_request(request: &mut Bundle<S>) -> Result<Self, Self::Rejection> {
        let buffer = request.0.get_message_buffer().ok_or(SimplyContinue {})?;
        Ok(buffer.to_owned())
    }
}

impl<'inner, S> FromRequest<S> for &'inner [u8] {
    type Rejection = SimplyContinue;

    fn from_request(request: &mut Bundle<S>) -> Result<Self, Self::Rejection> {
        Ok(request.0.get_message_buffer().ok_or(SimplyContinue {})?)
    }
}

impl<'inner, S> FromRequest<S> for &'inner mut Response {
    type Rejection = Infallible;

    fn from_request(request: &mut Bundle<S>) -> Result<Self, Self::Rejection> {
        Ok(unsafe {(&mut request.2 as *mut Response).as_mut().unwrap()})
    }
}

impl<'inner, S> FromRequest<S> for &'inner mut State<S> {
    type Rejection = Infallible;

    fn from_request(request: &mut Bundle<S>) -> Result<Self, Self::Rejection> {
        Ok(unsafe {(&mut request.1 as *mut State<S>).as_mut().unwrap()})
    }
}

impl<'inner, S> FromRequest<S> for &'inner mut Request {
    type Rejection = Infallible;

    fn from_request(request: &mut Bundle<S>) -> Result<Self, Self::Rejection> {
        Ok(unsafe {(&mut request.0 as *mut Request).as_mut().unwrap()})
    }
}


pub struct HandlerService<H, T, S> {
    handler: H,
    _marker: PhantomData<fn() -> (T, S)>,
}

impl<H, T, S> HandlerService<H, T, S> {
    fn new(handler: H) -> Self {
        Self {
            handler,
            _marker: PhantomData
        }
    }

    pub fn into_box_clone_service(self) -> BoxCloneService<Bundle<S>, Bundle<S>, Infallible> 
    where
        H: Clone + Send + 'static, 
        H: Handler<T, S>,
        S: 'static,
        Self: Service<Bundle<S>, Response = Bundle<S>, Error = Infallible, Future = HandlerServiceFuture<futures::future::Map<H::Future, fn(Bundle<S>) -> Result<Bundle<S>, Infallible>>>> + 'static
    {
        BoxCloneService::new(self)
    }
}

impl<H: Clone, T, S> Clone for HandlerService<H, T, S> {
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            _marker: PhantomData,
        }
    }
}

impl<H, T, S> Service<Bundle<S>> for HandlerService<H, T, S>
where
    H: Handler<T, S> + Clone + Send + 'static,
    S: Send + Sync + 'static,
{
    type Response = Bundle<S>;
    type Error = Infallible;
    type Future = HandlerServiceFuture<futures::future::Map<H::Future, fn(Bundle<S>) -> Result<Bundle<S>, Infallible>>>;

    fn call(&mut self, req: Bundle<S>) -> Self::Future {
        let handler = self.handler.clone();
        let future = handler.call(req).map(Ok as _);
        HandlerServiceFuture { future }
    }

    fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

pin_project_lite::pin_project! {
    pub struct HandlerServiceFuture<F> {
        #[pin]
        future: F
    }
}

impl<F> Future for HandlerServiceFuture<F> where F: Future {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        self.project().future.poll(cx)
    }
}
