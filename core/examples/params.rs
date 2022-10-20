use anyhow::Context;
use core::handler::HandlerTrait;
use core::handler::HandlerTraitWithoutState;
use core::request::ContentType;
use core::request::Json;
use core::request::State;
use core::response::Responder;
use core::route::Router;
use core::server::Server;
use hyper::Body;
use hyper::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct OwnBody {
    val: String,
}

impl Responder for OwnBody {
    fn into_response(self) -> anyhow::Result<core::response::Response> {
        serde_json::to_string(&self)
            .context("could not serialize to string")?
            .into_response()
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    fn handler() {}

    fn handler3(_req: Request<Body>) -> anyhow::Result<String> {
        Ok("own_param.0.".into())
    }

    fn handler4(body: String) -> anyhow::Result<String> {
        Ok(body)
    }

    fn handler5(Json(own_body): Json<OwnBody>) -> OwnBody {
        own_body
    }

    fn handler_header(ContentType(content_type): ContentType) -> anyhow::Result<String> {
        Ok(content_type)
    }

    fn handler_state(state: State<i32>) {}

    let app = Router::with_state(123)
        .get("/test/<param1>", handler)
        .get("/dupa/<param>", handler3)
        .get("/", handler4)
        .get("/json", handler5)
        .get("/header", handler_header)
        .post("/body", handler5)
        .get("/closure", handler)
        .get("/state", handler_state);

    Server::new("127.0.0.1", 8080).with_service(app).run()
}
