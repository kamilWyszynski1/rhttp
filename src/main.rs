mod echo;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    echo::EchoServer::new("127.0.0.1", 8080).run()
}
