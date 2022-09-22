use std::collections::HashMap;

use log::info;
use server::{Request, Response};

mod server;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    server::Server::new("127.0.0.1", 8080).get("/", test).run()
}

fn test(req: Request) -> anyhow::Result<Response> {
    info!("test - request that we've got: {:?}", req);
    info!("responding");
    Ok(Response {
        status: server::ResponseStatus::Ok,
        headers: HashMap::new(),
        protocol: server::ProtocolVersion::HTTP11,
        body: Some(String::from("response")),
    })
}
