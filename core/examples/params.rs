use core::request::Request;
use core::server::Server;
use log::info;
use std::fmt::Write as _;

#[derive(macros::FromStored)]
struct OwnParam(String);

fn main() -> anyhow::Result<()> {
    env_logger::init();

    fn handler(req: Request) -> anyhow::Result<String> {
        let param_value = req.query::<String>("param1")?;
        Ok(param_value)
    }

    fn handler2(req: Request) -> anyhow::Result<String> {
        let mut param_value = req.query::<String>("param2")?;
        let param_value_i32: i32 = req.query::<i32>("param3")?;

        let _ = write!(param_value, "{}", param_value_i32);
        Ok(param_value)
    }

    fn handler3(req: Request) -> anyhow::Result<String> {
        let own_param = req.query::<OwnParam>("param")?;
        Ok(own_param.0)
    }

    fn headers_handler(req: Request) {
        info!("{:?}", req.headers())
    }

    Server::new("127.0.0.1", 8080)
        .get("/test/<param1>", handler)
        .get("/test/<param2>/<param3>", handler2)
        .get("/dupa/<param>", handler3)
        .get("/", headers_handler)
        .run()
}
