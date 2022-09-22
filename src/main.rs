use std::collections::HashMap;

use http::{Request, Response};
use log::info;

use crate::http::{ProtocolVersion, ResponseStatus};

mod http;
mod outcome;
mod server;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    server::Server::new("127.0.0.1", 8080).get("/", test).run()
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
