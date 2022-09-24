use crate::{
    http::Method,
    middleware::Middleware,
    request::Request,
    response::{Responder, Response},
};
use anyhow::{bail, Context};
use log::error;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
};

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
            Err(_e) => Response::default(),
        }
    }
}

impl HandlerTrait for () {
    fn handle(&self, _request: Request) -> Response {
        Response::default()
    }
}

#[derive(Debug, Default, Clone)]
struct RouteMetadata {
    /// Original, registered path.
    origin: String,

    /// Holds params' segments index counted as place after '/' character.
    ///
    /// `/test/<param1>/<param2>` - {"param1": 1, "param2": 2}.
    param_segments: HashMap<String, u8>,
}

impl TryFrom<String> for RouteMetadata {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self {
            origin: value.clone(),
            param_segments: parse_param_segments(value)?,
        })
    }
}

fn parse_param_segments(value: String) -> anyhow::Result<HashMap<String, u8>> {
    let mut param_segments: HashMap<String, u8> = HashMap::new();
    let mut segment = String::new();
    let mut beginning_found = false;
    let mut slash_counter = 0;

    for c in value.chars() {
        match c {
            '/' => slash_counter += 1,
            '<' => {
                beginning_found = true;
                continue;
            }
            '>' => {
                beginning_found = false;
                // slash_counter - 1 because we don't want to consider starting '/'
                param_segments.insert(segment.clone(), slash_counter - 1);
                segment.clear();
                continue;
            }
            _ => {
                if beginning_found {
                    segment.push(c)
                }
            }
        }
    }

    if beginning_found {
        bail!("Invalid url - param segment not closed")
    }

    Ok(param_segments)
}

#[derive(Clone)]
struct Route {
    pub handler: Arc<Box<dyn HandlerTrait>>,

    /// Contains metadata about registered route.
    meta: RouteMetadata,
}

impl Route {
    /// Creates new Route, tries to parse path into RouteMetadata.
    fn new<S: Into<String>>(path: S, handler: Box<dyn HandlerTrait>) -> anyhow::Result<Self> {
        let path: String = path.into();
        Ok(Self {
            handler: Arc::new(handler),
            meta: RouteMetadata::try_from(path)?,
        })
    }

    /// Indicates if request's path match with router's path.
    ///
    /// '/test/john/doe'  & '/test/<name>/<surn>' => true,
    /// '/test/test/      & '/test/test'          => true,
    /// '/test/test/test' & '/test/test'          => false,
    pub fn should_fire_on_path(&self, path: &str) -> bool {
        let mut split_path = path.split('/');
        let mut split_route = self.meta.origin.split('/');

        for p in split_path.by_ref() {
            let r = match split_route.next() {
                Some(value) => value,
                None => {
                    return false;
                }
            };
            if p != r && !(r.starts_with('<') && r.ends_with('>')) {
                return false;
            }
        }
        // paths does not match if split_route still has some items.
        if split_route.next().is_some() {
            return false;
        }

        true
    }
}

pub struct Server {
    host: String,
    port: u32,

    /// Registered routes.
    routes: HashMap<Method, Vec<Route>>,

    /// Registered middlewares that will be run during request handling.
    middlewares: Vec<Box<dyn Middleware>>,
}

impl Server {
    pub fn new(host: impl Into<String>, port: u32) -> Self {
        Self {
            host: host.into(),
            port,
            routes: HashMap::new(),
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
            .find(|route| route.should_fire_on_path(&request.url))
            .context("no matching route")?
            .clone();

        for m in &self.middlewares {
            m.on_request(&mut request)?;
        }

        request.inject_params_seqments(route.meta.param_segments);
        let mut response = route.handler.handle(request);

        for m in &self.middlewares {
            m.on_response(&mut response)?;
        }

        let response_bytes: Vec<u8> = response.into();
        stream.write_all(&response_bytes)?;

        Ok(())
    }

    /// Registers GET route.
    pub fn get<S, R, H>(mut self, path: S, handler: H) -> Self
    where
        S: Into<String>,
        R: Responder + 'static,
        H: Fn(Request) -> R + Send + Sync + 'static,
    {
        self.routes.entry(Method::Get).or_default().push(
            Route::new(path, Box::new(handler)).expect("tried to register invalid GET route"),
        );
        self
    }

    /// Registers POST route.
    pub fn post<S, R, H>(mut self, path: S, handler: H) -> Self
    where
        S: Into<String>,
        R: Responder + 'static,
        H: Fn(Request) -> R + Send + Sync + 'static,
    {
        self.routes.entry(Method::Post).or_default().push(
            Route::new(path, Box::new(handler)).expect("tried to register invalid POST route"),
        );
        self
    }

    /// Registers PUT route.
    pub fn put<S, R, H>(mut self, path: S, handler: H) -> Self
    where
        S: Into<String>,
        R: Responder + 'static,
        H: Fn(Request) -> R + Send + Sync + 'static,
    {
        self.routes.entry(Method::Put).or_default().push(
            Route::new(path, Box::new(handler)).expect("tried to register invalid PUT route"),
        );
        self
    }

    /// Registers DELETE route.
    pub fn delete<S, R, H>(mut self, path: S, handler: H) -> Self
    where
        S: Into<String>,
        R: Responder + 'static,
        H: Fn(Request) -> R + Send + Sync + 'static,
    {
        self.routes.entry(Method::Delete).or_default().push(
            Route::new(path, Box::new(handler)).expect("tried to register invalid DELETE route"),
        );
        self
    }

    /// Registers new middleware.
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
    use crate::{middleware::LogMiddleware, request::Request, response::Response};

    use super::{Route, Server};

    #[test]
    fn test_handlers() {
        fn handler(_req: Request) {}
        fn handler2(_req: Request) -> &'static str {
            "hello"
        }
        fn handler3(_req: Request) -> Response {
            Response::default()
        }

        Server::new("127.0.0.1", 8080)
            .get("/", handler)
            .get("/", handler2)
            .get("/", handler3);
    }

    #[test]
    fn test_middlewares() {
        Server::new("127.0.0.1", 8080).middleware(LogMiddleware {});
    }

    #[test]
    fn test_params() -> anyhow::Result<()> {
        fn handler(_req: Request) -> &'static str {
            "hello"
        }

        // /test/<param1>
        fn handler2(req: Request) -> anyhow::Result<&'static str> {
            let _param_value = req.query::<String>("param1")?;
            Ok("hello")
        }

        Server::new("127.0.0.1", 8080)
            .get("/", handler)
            .get("/test/<param1>", handler2);
        Ok(())
    }

    #[test]
    fn test_should_fire_on_path() {
        let r = Route::new("/test", Box::new(())).expect("valid route");

        assert!(r.should_fire_on_path("/test"));
        assert!(!r.should_fire_on_path("/test/test"));
        assert!(!r.should_fire_on_path("/"));

        let r = Route::new("/test/<param1>", Box::new(())).expect("valid route");

        assert!(!r.should_fire_on_path("/test"));
        assert!(r.should_fire_on_path("/test/test"));
        assert!(!r.should_fire_on_path("/"));

        let r = Route::new("/test/<param1>/<param2>", Box::new(())).expect("valid route");

        assert!(r.should_fire_on_path("/test/1/2"));
        assert!(!r.should_fire_on_path("/test/test"));
        assert!(!r.should_fire_on_path("/"));
    }
}
