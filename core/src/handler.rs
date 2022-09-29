use crate::{
    request::{FromRequest, FromRequestParts},
    response::{Responder, Response},
};
use hyper::{Body, Request};
use std::{marker::PhantomData, sync::Arc};

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
pub struct IntoService<H, S, Q, B> {
    handler: H,
    state: Arc<S>,
    _marker: PhantomData<fn() -> (Q, B)>,
}

impl<H, S, Q, B> Service<Request<B>> for IntoService<H, S, Q, B>
where
    H: HandlerTrait<Q, S, B>,
{
    type Response = Response;

    fn call(&self, req: Request<B>) -> Self::Response {
        self.handler.handle(req, &self.state.clone())
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
pub trait HandlerTrait<Q, S = (), B = Body>: Sized + Send + Sync + 'static {
    /// User defined logic.
    fn handle(&self, request: Request<B>, state: &S) -> Response;

    /// Turns Self into `IntoService`.
    fn into_service_with_state(self, state: S) -> IntoService<Self, S, Q, B> {
        IntoService {
            handler: self,
            state: Arc::new(state),
            _marker: PhantomData,
        }
    }
}

/// Helper trait for implementing handler that does not use state.
pub trait HandlerTraitWithoutState<Q, B>: HandlerTrait<Q, (), B> {
    fn into_service(self) -> IntoService<Self, (), Q, B>;
}

impl<Q, B, H> HandlerTraitWithoutState<Q, B> for H
where
    H: HandlerTrait<Q, (), B>,
{
    fn into_service(self) -> IntoService<Self, (), Q, B> {
        IntoService {
            handler: self,
            state: Arc::new(()),
            _marker: PhantomData,
        }
    }
}

macro_rules! implement_handler_trait {
    ([$($ty:ident),*], $last:ident) => {
        #[allow(non_snake_case, unused_mut)]
        impl<F, S, B, R, $($ty,)* $last, M> HandlerTrait<($($ty,)* $last, M), S, B> for F
        where
            R: Responder + 'static,
            $($ty:FromRequestParts<S>,)*
            $last: FromRequest<B, S, M>,
            F: Fn($($ty,)* $last) -> R + Send + Sync + 'static
        {
            fn handle(&self, request: Request<B>, state: &S) -> Response {
                let (mut parts, body) = request.into_parts();

                match self(
                    $(
                        $ty::from_request_parts(&mut parts, state).unwrap(),
                    )*
                    $last::from_request(Request::from_parts(parts, body), state).unwrap(),
                )
                .into_response()
                {
                    Ok(response) => response,
                    Err(_e) => Response::default(),
                }
            }
        }
    };
}

implement_handler_trait!([], T1);
implement_handler_trait!([T1], T2);
implement_handler_trait!([T1, T2], T3);
implement_handler_trait!([T1, T2, T3], T4);
implement_handler_trait!([T1, T2, T3, T4], T5);

impl<F, S, B, R> HandlerTrait<((),), S, B> for F
where
    R: Responder + 'static,
    F: Fn() -> R + Send + Sync + 'static,
{
    fn handle(&self, _request: Request<B>, _state: &S) -> Response {
        match self().into_response() {
            Ok(response) => response,
            Err(_e) => Response::default(),
        }
    }
}

impl<S, B> HandlerTrait<(), S, B> for () {
    fn handle(&self, _request: Request<B>, _state: &S) -> Response {
        Response::default()
    }
}

pub struct BoxCloneService<T, U>(pub Box<dyn Service<T, Response = U> + Send + Sync>);

impl<T, U> BoxCloneService<T, U> {
    pub fn new<V>(service: V) -> Self
    where
        V: Service<T, Response = U> + Send + Sync + 'static,
    {
        Self(Box::new(service))
    }
}

impl<H, S, Q, B> From<IntoService<H, S, Q, B>> for BoxCloneService<Request<B>, Response>
where
    S: Send + Sync + 'static,
    B: 'static,
    Q: 'static,
    H: HandlerTrait<Q, S, B>,
{
    fn from(val: IntoService<H, S, Q, B>) -> Self {
        BoxCloneService::new(val)
    }
}
