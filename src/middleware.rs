use log::debug;

use crate::{http::Request, response::Response};

pub trait Middleware: Send + Sync {
    /// Functionality that is being run on every request that goes into the server.
    fn on_request(&self, _req: &mut Request) -> anyhow::Result<()> {
        Ok(())
    }

    /// Functionality that is being run every response that goes out of a server.
    fn on_response(&self, _res: &mut Response) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct LogMiddleware {}

impl Middleware for LogMiddleware {
    fn on_request(&self, req: &mut Request) -> anyhow::Result<()> {
        debug!("LogMiddleware::on_request - request: {:?}", req);
        Ok(())
    }

    fn on_response(&self, res: &mut Response) -> anyhow::Result<()> {
        debug!("LogMiddleware::on_response - response: {:?}", res);
        Ok(())
    }
}
