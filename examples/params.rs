use rhttp::request::Request;
use rhttp::server::Server;
use std::fmt::Write as _; // import without risk of name clashing

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

    Server::new("127.0.0.1", 8080)
        .get("/test/<param1>", handler)
        .get("/test/<param2>/<param3>", handler2)
        .run()
}
