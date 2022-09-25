use std::fmt::Debug;

use hyper::{Body, Request};
use log::debug;

use crate::response::Response;

pub trait Middleware<B = Body>: Send + Sync {
    /// Functionality that is being run on every request that goes into the server.
    fn on_request(&self, _req: &mut Request<B>) -> anyhow::Result<()> {
        Ok(())
    }

    /// Functionality that is being run every response that goes out of a server.
    fn on_response(&self, _res: &mut Response) -> anyhow::Result<()> {
        Ok(())
    }
}

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
