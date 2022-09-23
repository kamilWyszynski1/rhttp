use anyhow::Context;
use log::error;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
};

use crate::{
    http::{Method, ProtocolVersion, Request, ResponseStatus},
    middleware::Middleware,
    response::{self, Responder, Response},
};

type InnerHandler = Box<dyn Fn(Request) -> anyhow::Result<Response> + Send + Sync>;
type Handler = Arc<InnerHandler>;

#[derive(Clone)]
struct Route {
    path: String,
    pub handler: Handler,
}

fn wrap<F, R>(f: F) -> Box<dyn HandlerTrait>
where
    R: Responder,
    F: Fn(Request) -> R + Send + Sync,
{
    // Box::new(|req: Request| f(req.clone()).respond_to(req).expect("error in wrap"))
    Box::new(|req| Response::default())
}

pub trait HandlerTrait: Send + Sync + 'static {
    fn handle(&self, request: Request) -> Response;
}

impl<F, R> HandlerTrait for F
where
    R: Responder + 'static,
    F: Fn(Request) -> R + Send + Sync + 'static,
{
    fn handle(&self, request: Request) -> Response {
        match self(request.clone()).respond_to(request) {
            Ok(response) => response,
            Err(e) => Response::default(),
        }
    }
}

impl HandlerTrait for () {
    fn handle(&self, request: Request) -> Response {
        Response::default()
    }
}

struct Route2 {
    path: String,
    pub handler: Box<dyn HandlerTrait>,
}

pub struct Server {
    host: String,
    port: u32,

    /// Registered routes.
    routes: HashMap<Method, Vec<Route>>,

    routes2: HashMap<Method, Vec<Route2>>,

    /// Registered middlewares that will be run during request handling.
    middlewares: Vec<Box<dyn Middleware>>,
}

impl Server {
    pub fn new(host: impl Into<String>, port: u32) -> Self {
        Self {
            host: host.into(),
            port,
            routes: HashMap::new(),
            routes2: HashMap::new(),
            middlewares: vec![],
        }
    }

    /// Starts server,
    pub fn run(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port))?;

        let server = Arc::new(self);

        for stream in listener.incoming() {
            let stream = stream?;
            let s = server.clone();
            thread::spawn(move || {
                if let Err(e) = s.handle(stream) {
                    error!("got error during handling connection: {}", e);
                }
            });
        }
        Ok(())
    }

    // Calls route's handler and pass response to function that writes to opened stream.
    fn handle(&self, mut stream: TcpStream) -> anyhow::Result<()> {
        let mut request = parse_request_from_tcp(&mut stream)?;
        let route = self
            .routes
            .get(&request.method)
            .with_context(|| format!("not registered routes for {:?} method", request.method))?
            .iter()
            .find(|route| route.path == request.url)
            .context("no matching route")?
            .clone();

        for m in &self.middlewares {
            m.on_request(&mut request)?;
        }

        let mut response = match (route.handler)(request) {
            Ok(r) => r,
            Err(e) => {
                error!("handle_connection_http - error: {}", e);
                Response {
                    protocol: ProtocolVersion::HTTP10,
                    status: ResponseStatus::InternalServerError,
                    headers: HashMap::new(),
                    body: None,
                }
            }
        };

        for m in &self.middlewares {
            m.on_response(&mut response)?;
        }

        let response_bytes: Vec<u8> = response.into();
        stream.write_all(&response_bytes)?;

        Ok(())
    }

    /// Registers GET route.
    pub fn get<S, H>(mut self, path: S, handler: H) -> Self
    where
        S: Into<String>,
        H: Fn(Request) -> anyhow::Result<Response> + Send + Sync + 'static,
    {
        self.routes.entry(Method::Get).or_default().push(Route {
            path: path.into(),
            handler: Arc::new(Box::new(handler)),
        });
        self
    }

    /// Registers GET route.
    pub fn get2<S, R, H>(mut self, path: S, handler: H) -> Self
    where
        S: Into<String>,
        R: Responder + 'static,
        H: Fn(Request) -> R + Send + Sync + 'static,
    {
        self.routes2.entry(Method::Get).or_default().push(Route2 {
            path: path.into(),
            handler: Box::new(handler),
        });
        self
    }

    /// Registers POST route.
    pub fn post<S, H>(mut self, path: S, handler: H) -> Self
    where
        S: Into<String>,
        H: Fn(Request) -> anyhow::Result<Response> + Send + Sync + 'static,
    {
        self.routes.entry(Method::Post).or_default().push(Route {
            path: path.into(),
            handler: Arc::new(Box::new(handler)),
        });
        self
    }
    /// Registers PUT route.
    pub fn put<S, H>(mut self, path: S, handler: H) -> Self
    where
        S: Into<String>,
        H: Fn(Request) -> anyhow::Result<Response> + Send + Sync + 'static,
    {
        self.routes.entry(Method::Put).or_default().push(Route {
            path: path.into(),
            handler: Arc::new(Box::new(handler)),
        });
        self
    }
    /// Registers DELETE route.
    pub fn delete<S, H>(mut self, path: S, handler: H) -> Self
    where
        S: Into<String>,
        H: Fn(Request) -> anyhow::Result<Response> + Send + Sync + 'static,
    {
        self.routes.entry(Method::Delete).or_default().push(Route {
            path: path.into(),
            handler: Arc::new(Box::new(handler)),
        });
        self
    }

    pub fn middleware<M>(mut self, m: M) -> Self
    where
        M: Middleware + 'static,
    {
        self.middlewares.push(Box::new(m));
        self
    }
}

const MESSAGE_SIZE: usize = 1024;

/// Takes TcpStream, reads whole content and parses it to a http request.
fn parse_request_from_tcp(stream: &mut TcpStream) -> anyhow::Result<Request> {
    // Store all the bytes for our received String
    let mut received: Vec<u8> = vec![];

    // Array with a fixed size
    let mut rx_bytes = [0u8; MESSAGE_SIZE];
    loop {
        // Read from the current data in the TcpStream
        let bytes_read = stream.read(&mut rx_bytes)?;

        // However many bytes we read, extend the `received` string bytes
        received.extend_from_slice(&rx_bytes[..bytes_read]);

        // If we didn't fill the array
        // stop reading because there's no more data (we hope!)
        if bytes_read < MESSAGE_SIZE {
            break;
        }
    }

    Request::parse(String::from_utf8(received)?)
}

#[cfg(test)]
mod tests {
    use crate::{http::Request, middleware::LogMiddleware, response::Response};

    use super::Server;

    #[test]
    fn test_handlers() -> anyhow::Result<()> {
        fn handler(_req: Request) {}
        fn handler2(_req: Request) -> &'static str {
            "hello"
        }
        fn handler3(_req: Request) -> Response {
            Response::default()
        }

        Server::new("127.0.0.1", 8080)
            .get2("/", handler)
            .get2("/", handler2)
            .get2("/", handler3)
            .run()
    }

    #[test]
    fn test_middlewares() -> anyhow::Result<()> {
        Server::new("127.0.0.1", 8080)
            .middleware(LogMiddleware {})
            .run()
    }
}
