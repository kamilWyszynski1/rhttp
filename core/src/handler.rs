use crate::{
    request::FromRequest,
    response::{Responder, Response},
};
use hyper::{Body, Request};
use std::marker::PhantomData;

/// Trait implemented by transition handler's state.
/// Introduced to have handlers that are generic only over R type.
pub trait Service<R> {
    type Response;

    /// Calls service's logic.
    fn call(&self, req: R) -> Self::Response;
}

/// Transition state for handler, it helps 'hide' Q type that is specific
/// for various types of functions(with different amount of parameters).
///
/// IntoService implements Service trait and this way it's responsible for
/// calling handler effectively calling wanted handler's logic.
pub struct IntoService<H, Q, B> {
    handler: H,
    _marker: PhantomData<fn() -> (Q, B)>,
}

impl<H, Q, B> Service<Request<B>> for IntoService<H, Q, B>
where
    H: HandlerTrait<Q, B>,
{
    type Response = Response;

    fn call(&self, req: Request<B>) -> Self::Response {
        self.handler.handle(req)
    }
}

impl<B> Service<Request<B>> for () {
    type Response = ();

    fn call(&self, _req: Request<B>) -> Self::Response {}
}

/// Main 'entrypoint' for crate handlers. Various types of functions
/// can implement this trait to be passed to Server as handlers.
/// This trait itself does not represent 'final' state of handler,
/// `into_service` function has to be called to turn Self into
/// `IntoService` which is responsible for calling handler's logic.
pub trait HandlerTrait<Q, B = Body>: Sized + Send + Sync + 'static {
    /// User defined logic.
    fn handle(&self, request: Request<B>) -> Response;

    /// Turns Self into `IntoService`.
    fn into_service(self) -> IntoService<Self, Q, B>;
}

impl<F, B, R, Q> HandlerTrait<(Q, B), B> for F
where
    R: Responder + 'static,
    Q: FromRequest<B>,
    F: Fn(Q) -> R + Send + Sync + 'static,
{
    fn handle(&self, request: Request<B>) -> Response {
        let q = Q::from_request(request).unwrap();
        match self(q).into_response() {
            Ok(response) => response,
            Err(_e) => Response::default(),
        }
    }

    fn into_service(self) -> IntoService<Self, (Q, B), B> {
        IntoService {
            handler: self,
            _marker: PhantomData,
        }
    }
}

impl<F, B, R> HandlerTrait<((),), B> for F
where
    R: Responder + 'static,
    F: Fn() -> R + Send + Sync + 'static,
{
    fn handle(&self, _request: Request<B>) -> Response {
        match self().into_response() {
            Ok(response) => response,
            Err(_e) => Response::default(),
        }
    }
    fn into_service(self) -> IntoService<Self, ((),), B> {
        IntoService {
            handler: self,
            _marker: PhantomData,
        }
    }
}

impl<B> HandlerTrait<(), B> for () {
    fn handle(&self, _request: Request<B>) -> Response {
        Response::default()
    }

    fn into_service(self) -> IntoService<Self, (), B> {
        IntoService {
            handler: (),
            _marker: PhantomData,
        }
    }
}

pub struct BoxCloneService<T, U>(pub Box<dyn Service<T, Response = U> + Send + Sync>);

impl<T, U> BoxCloneService<T, U> {
    pub fn new<S>(service: S) -> Self
    where
        S: Service<T, Response = U> + Send + Sync + 'static,
    {
        Self(Box::new(service))
    }
}

impl<H, Q, B> From<IntoService<H, Q, B>> for BoxCloneService<Request<B>, Response>
where
    B: 'static,
    Q: 'static,
    H: HandlerTrait<Q, B>,
{
    fn from(val: IntoService<H, Q, B>) -> Self {
        BoxCloneService::new(val)
    }
}
