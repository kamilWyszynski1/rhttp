use core::handler::HandlerTraitWithoutState;
use core::request::ContentType;
use core::request::Json;
use core::server::Server;
use hyper::Body;
use hyper::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct OwnBody {
    val: String,
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

    fn handler5(Json(own_body): Json<OwnBody>) {}

    fn handler_header(ContentType(content_type): ContentType) -> anyhow::Result<String> {
        Ok(content_type)
    }

    Server::new("127.0.0.1", 8080)
        .get("/test/<param1>", handler.into_service())
        .get("/dupa/<param>", handler3.into_service())
        .get("/", handler4.into_service())
        .get("/json", handler5.into_service())
        .get("/header", handler_header.into_service())
        .post("/body", handler5.into_service())
        .run()
}
