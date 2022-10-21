use crate::{
    request::{FromRequest, FromRequestParts},
    response::{Responder, Response},
};
use hyper::{Body, Request};
use std::{marker::PhantomData, sync::Arc};

/// Trait implemented by transition handler's state.
/// Introduced to have handlers that are generic only over R type.
pub trait Service<R> {
    /// Calls service's logic.
    fn call(&self, req: R) -> Response;
}

/// Transition state for handler, it helps 'hide' Q type that is specific
/// for various types of functions(with different amount of parameters).
///
/// IntoService implements Service trait and this way it's responsible for
/// calling handler effectively calling wanted handler's logic.
pub struct IntoService<H, S, Q> {
    handler: H,
    state: Arc<S>,
    _marker: PhantomData<fn() -> (Q, Body)>,
}

impl<H, S, Q> Service<Request<Body>> for IntoService<H, S, Q>
where
    H: HandlerTrait<Q, S>,
{
    fn call(&self, req: Request<Body>) -> Response {
        self.handler.handle(req, &self.state.clone())
    }
}

impl<B> Service<Request<B>> for () {
    fn call(&self, _req: Request<B>) -> Response {
        Response::default()
    }
}

/// Main 'entrypoint' for crate handlers. Various types of functions
/// can implement this trait to be passed to Server as handlers.
/// This trait itself does not represent 'final' state of handler,
/// `into_service` function has to be called to turn Self into
/// `IntoService` which is responsible for calling handler's logic.
pub trait HandlerTrait<Q, S = ()>: Sized + Send + Sync + 'static {
    /// User defined logic.
    fn handle(&self, request: Request<Body>, state: &S) -> Response;

    /// Turns Self into `IntoService`.
    fn into_service_with_state(self, state: S) -> IntoService<Self, S, Q> {
        IntoService {
            handler: self,
            state: Arc::new(state),
            _marker: PhantomData,
        }
    }

    fn into_service_with_state_arc(self, state: Arc<S>) -> IntoService<Self, S, Q> {
        IntoService {
            handler: self,
            state,
            _marker: PhantomData,
        }
    }
}

/// Helper trait for implementing handler that does not use state.
pub trait HandlerTraitWithoutState<Q>: HandlerTrait<Q, ()> {
    fn into_service(self) -> IntoService<Self, (), Q> {
        self.into_service_with_state(())
    }
}

impl<Q, H> HandlerTraitWithoutState<Q> for H where H: HandlerTrait<Q> {}

macro_rules! implement_handler_trait {
    ([$($ty:ident),*], $last:ident) => {
        #[allow(non_snake_case, unused_mut)]
        impl<F, S,  R, $($ty,)* $last, M> HandlerTrait<($($ty,)* $last, M), S> for F
        where
            R: Responder + 'static,
            $($ty:FromRequestParts<S>,)*
            $last: FromRequest<Body, S, M>,
            F: Fn($($ty,)* $last) -> R + Send + Sync + 'static
        {
            fn handle(&self, request: Request<Body>, state: &S) -> Response {
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

impl<F, S, R> HandlerTrait<((),), S> for F
where
    R: Responder + 'static,
    F: Fn() -> R + Send + Sync + 'static,
{
    fn handle(&self, _request: Request<Body>, _state: &S) -> Response {
        match self().into_response() {
            Ok(response) => response,
            Err(_e) => Response::default(),
        }
    }
}

impl<S> HandlerTrait<(), S> for () {
    fn handle(&self, _request: Request<Body>, _state: &S) -> Response {
        Response::default()
    }
}

pub struct BoxCloneService<T>(pub Box<dyn Service<T> + Send + Sync>);

impl<T> BoxCloneService<T> {
    pub fn new<V>(service: V) -> Self
    where
        V: Service<T> + Send + Sync + 'static,
    {
        Self(Box::new(service))
    }
}

impl<H, S, Q> From<IntoService<H, S, Q>> for BoxCloneService<Request<Body>>
where
    S: Send + Sync + 'static,
    Q: 'static,
    H: HandlerTrait<Q, S>,
{
    fn from(val: IntoService<H, S, Q>) -> Self {
        BoxCloneService::new(val)
    }
}
