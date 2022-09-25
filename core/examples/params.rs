use core::server::FromRequest;
use core::server::Server;
use hyper::body::Bytes;
use hyper::Body;
use hyper::Request;
use serde::{Deserialize, Serialize};

#[derive(macros::FromStored)]
struct OwnParam(String);

#[derive(Deserialize, Serialize)]
struct OwnBody {
    val: String,
}

// impl FromRequest<Body> for OwnBody {
//     fn from_request(req: Request<Body>) -> anyhow::Result<Self> {
//         let bytes: Bytes = futures_executor::block_on(hyper::body::to_bytes(req.into_body()))?;
//         let decoded: OwnBody = bincode::deserialize(bytes.);
//     }
// }

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
        Ok("own_param.0.".into())
    }

    // fn headers_handler(req: Request<()>) {
    //     info!("{:?}", req.headers())
    // }

    // #[derive(Deserialize, Serialize)]
    // struct Body {
    //     val: String,
    //     val_int: i32,
    // }

    // impl FromStored for Body {
    //     fn from_stored(stored: String) -> anyhow::Result<Self> {
    //         Ok(serde_json::from_str(&stored)?)
    //     }
    // }

    // fn body_handler(req: Request<()>) -> anyhow::Result<String> {
    //     let body = req.body::<Body>()?;
    //     Ok(serde_json::to_string(&body)?)
    // }

    Server::new("127.0.0.1", 8080)
        .get("/test/<param1>", handler)
        .get("/", handler2)
        .get("/dupa/<param>", handler3)
        // .get("/", handler4)
        // .post("/body", body_handler)
        .run()
}
