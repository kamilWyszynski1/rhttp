use crate::http::{ProtocolVersion, ResponseStatus};
use http::Request;
use log::info;
use middleware::LogMiddleware;
use response::{Responder, Response};
use rhttp::server::HandlerTrait;
use std::collections::HashMap;

mod http;
mod middleware;
mod response;
mod server;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    server::Server::new("127.0.0.1", 8080)
        .middleware(LogMiddleware {})
        // .get("/", test)
        // .get2("/", test2)
        // .get2("/", test3)
        // .get2("/", test4)
        .run()?;
    Ok(())
}

// fn test(req: Request) -> anyhow::Result<Response> {
//     info!("test - request that we've got: {:?}", req);
//     info!("responding");
//     Ok(Response {
//         status: ResponseStatus::Ok,
//         headers: HashMap::new(),
//         protocol: ProtocolVersion::HTTP11,
//         body: Some(String::from("response")),
//     })
// }

fn test2(req: Request) -> &'static str {
    "hello"
}

fn test() -> &'static str {
    "elo"
}

fn test4(req: Request) -> String {
    "hello".into()
}

// Fn(Request) -> Box<dyn Responder>

fn foo<H: HandlerTrait>(h: H) {}

fn boo() {
    foo(test4)
}
