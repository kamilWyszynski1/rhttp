use anyhow::{bail, Context};
use log::{debug, error, info};
use std::{
    any,
    collections::HashMap,
    fmt::{Debug, Display},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
};

use crate::http::{Method, ProtocolVersion, Request, Response, ResponseStatus};

type InnerHandler = Box<dyn Fn(Request) -> anyhow::Result<Response> + Send + Sync>;
type Handler = Arc<InnerHandler>;

#[derive(Clone)]
struct Route {
    path: String,
    pub handler: Handler,
}

pub struct Server {
    host: String,
    port: u32,

    routes: HashMap<Method, Vec<Route>>,
}

impl Server {
    pub fn new(host: impl Into<String>, port: u32) -> Self {
        Self {
            host: host.into(),
            port,
            routes: HashMap::new(),
        }
    }

    /// Starts server,
    pub fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port))?;

        for stream in listener.incoming() {
            if let Err(e) = self.handle(stream?) {
                error!("got error during handling connection: {}", e);
            }
        }
        Ok(())
    }

    // Calls route's handler and pass response to function that writes to opened stream.
    fn handle(&self, mut stream: TcpStream) -> anyhow::Result<()> {
        let request = parse_request_from_tcp(&mut stream)?;
        let route = self
            .routes
            .get(&request.method)
            .with_context(|| format!("not registered routes for {:?} method", request.method))?
            .iter()
            .find(|route| route.path == request.url)
            .context("no matching route")?
            .clone();

        thread::spawn(move || handle_connection_http(stream, (route.handler)(request)));

        Ok(())
    }

    /// Registers GET route.
    pub fn get<S, H>(&mut self, path: S, handler: H) -> &mut Self
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

    /// Registers POST route.
    pub fn post<S, H>(&mut self, path: S, handler: H) -> &mut Self
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
    pub fn put<S, H>(&mut self, path: S, handler: H) -> &mut Self
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
    pub fn delete<S, H>(&mut self, path: S, handler: H) -> &mut Self
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

fn handle_connection_http(
    mut stream: TcpStream,
    response: anyhow::Result<Response>,
) -> anyhow::Result<()> {
    info!("responding with: {:?}", response);

    let response = match response {
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

    let response_bytes: Vec<u8> = response.into();
    stream.write_all(&response_bytes)?;

    Ok(())
}
