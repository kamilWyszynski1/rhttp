use std::fmt::Debug;

use hyper::{Body, Request};
use log::debug;

use crate::response::Response;

/// Splitting MiddlewareClone into its own trait allows us to provide a blanket
/// implementation for all compatible types, without having to implement the
/// rest of Middleware. In this case, we implement it for all types that have
/// 'static lifetime (*i.e.* they don't contain non-'static pointers), and
/// implement both Middleware and Clone.
///
/// This is hack because `Middleware: Clone` would make it not object safe.
/// More info here: https://doc.rust-lang.org/reference/items/traits.html#object-safety.
pub trait MiddlewareClone {
    fn clone_box(&self) -> Box<dyn Middleware>;
}

impl<T> MiddlewareClone for T
where
    T: 'static + Middleware + Clone,
{
    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Middleware> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait Middleware<B = Body>: MiddlewareClone + Send + Sync {
    /// Functionality that is being run on every request that goes into the server.
    fn on_request(&self, _req: &mut Request<B>) -> anyhow::Result<()> {
        Ok(())
    }

    /// Functionality that is being run every response that goes out of a server.
    fn on_response(&self, _res: &mut Response) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LogMiddleware {}

impl<B> Middleware<B> for LogMiddleware
where
    B: Debug,
{
    fn on_request(&self, req: &mut Request<B>) -> anyhow::Result<()> {
        debug!("LogMiddleware::on_request - request: {:?}", req);
        Ok(())
    }

    fn on_response(&self, res: &mut Response) -> anyhow::Result<()> {
        debug!("LogMiddleware::on_response - response: {:?}", res);
        Ok(())
    }
}
