use core::request::ContentType;
use core::request::Json;
use core::server::Server;
use hyper::body::Bytes;
use hyper::Body;
use hyper::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct OwnBody {
    val: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    fn handler() {
        dbg!("handler");
        // let param_value = req.query::<String>("param1")?;
        // Ok(param_value)
    }

    fn handler2() -> &'static str {
        dbg!("handler2");
        "ok"
    }

    fn handler3(req: Request<Body>) -> anyhow::Result<String> {
        dbg!("handler3");
        Ok("own_param.0.".into())
    }

    fn handler4(body: String) -> anyhow::Result<String> {
        dbg!("handler3");
        Ok(body)
    }

    fn handler5(Json(own_body): Json<OwnBody>) {
        dbg!(own_body);
    }

    fn handler_header(ContentType(content_type): ContentType) -> anyhow::Result<String> {
        Ok(content_type)
    }

    Server::new("127.0.0.1", 8080)
        .get("/test/<param1>", handler)
        // .get("/", handler2)
        .get("/dupa/<param>", handler3)
        .get("/", handler4)
        .get("/json", handler5)
        .get("/header", handler_header)
        .run()
}
