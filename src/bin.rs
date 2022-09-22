use std::collections::HashMap;

use http::Request;
use log::info;
use middleware::LogMiddleware;
use response::Response;

use crate::http::{ProtocolVersion, ResponseStatus};

mod http;
mod middleware;
mod response;
mod server;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    server::Server::new("127.0.0.1", 8080)
        .middleware(LogMiddleware {})
        .get("/", test)
        .run()?;
    Ok(())
}

fn test(req: Request) -> anyhow::Result<Response> {
    info!("test - request that we've got: {:?}", req);
    info!("responding");
    Ok(Response {
        status: ResponseStatus::Ok,
        headers: HashMap::new(),
        protocol: ProtocolVersion::HTTP11,
        body: Some(String::from("response")),
    })
}

fn test2(req: Request) -> &'static str {
    "hello"
}
