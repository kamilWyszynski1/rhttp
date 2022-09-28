use crate::{
    handler::{BoxCloneService, HandlerTrait, Service},
    middleware::Middleware,
    response::{response_to_bytes, Response},
};
use anyhow::{bail, Context, Ok};
use hyper::{Body, Method, Request};
use log::error;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
};

#[derive(Debug, Default, Clone)]
pub struct RouteMetadata {
    /// Original, registered path.
    origin: String,

    /// Holds params' segments index counted as place after '/' character.
    ///
    /// `/test/<param1>/<param2>` -{ 0: 1, 1: 2 }.
    pub param_segments: HashMap<usize, usize>,
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

fn parse_param_segments(value: String) -> anyhow::Result<HashMap<usize, usize>> {
    let mut param_segments: HashMap<usize, usize> = HashMap::new();
    let mut segment = String::new();
    let mut beginning_found = false;
    let mut slash_counter = 0;
    let mut found = 0;

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
                param_segments.insert(found, slash_counter - 1);
                found += 1;
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
pub struct Route {
    pub service: Arc<BoxCloneService<Request<Body>, Response>>,

    /// Contains metadata about registered route.
    pub metadata: RouteMetadata,
}

impl Route {
    /// Creates new Route, tries to parse path into RouteMetadata.
    pub fn new<P>(
        path: P,
        handler: BoxCloneService<Request<Body>, Response>,
    ) -> anyhow::Result<Self>
    where
        P: Into<String>,
    {
        let path: String = path.into();
        Ok(Self {
            service: Arc::new(handler),
            metadata: RouteMetadata::try_from(path)?,
        })
    }

    /// Indicates if request's path match with router's path.
    ///
    /// '/test/john/doe'  & '/test/<name>/<surn>' => true,
    /// '/test/test/      & '/test/test'          => true,
    /// '/test/test/test' & '/test/test'          => false,
    pub fn should_fire_on_path<P: ToString>(&self, path: P) -> bool {
        let path = path.to_string();
        let mut split_path = path.split('/');
        let mut split_route = self.metadata.origin.split('/');

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

#[derive(Default)]
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
            ..Default::default()
        }
    }

    /// Creates new server with given routes. Should be used only internally for testing.
    pub fn new_with_routes(routes: HashMap<Method, Vec<Route>>) -> Self {
        Self {
            routes,
            ..Default::default()
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

    /// Calls route's handler and pass response to function that writes to opened stream.
    fn handle(&self, mut stream: TcpStream) -> anyhow::Result<()> {
        let response = self.fire::<TcpStream>(parse_request_from_tcp(&mut stream)?)?;

        let response_bytes: Vec<u8> = response_to_bytes(response)?;
        stream.write_all(&response_bytes)?;

        Ok(())
    }

    /// Method that runs whole server's logic. Takes Write trait
    /// implementation in order to mock it during testing.
    pub fn fire<W>(&self, mut request: Request<Body>) -> anyhow::Result<Response>
    where
        W: std::io::Write,
    {
        let route = self
            .routes
            .get(request.method())
            .with_context(|| format!("not registered routes for {:?} method", request.method()))?
            .iter()
            .find(|route| route.should_fire_on_path(request.uri().path()))
            .context("no matching route")?;

        for m in &self.middlewares {
            m.on_request(&mut request)?;
        }

        let extensions = request.extensions_mut();
        extensions.insert(route.metadata.param_segments.clone());

        let mut response = route.service.0.call(request);

        for m in &self.middlewares {
            m.on_response(&mut response)?;
        }

        Ok(response)
    }

    /// Registers GET route.
    pub fn get<P, V, T, U>(mut self, path: P, service: V) -> Self
    where
        P: Into<String>,
        V: Service<Request<Body>, Response = Response> + Send + Sync + 'static,
    {
        self.routes.entry(Method::GET).or_default().push(
            Route::new(path, BoxCloneService::new(service))
                .expect("tried to register invalid GET route"),
        );
        self
    }

    /// Registers POST route.
    pub fn post<P, V, T, U>(mut self, path: P, service: V) -> Self
    where
        P: Into<String>,
        V: Service<Request<Body>, Response = Response> + Send + Sync + 'static,
    {
        self.routes.entry(Method::POST).or_default().push(
            Route::new(path, BoxCloneService::new(service))
                .expect("tried to register invalid POST route"),
        );
        self
    }

    ///// Registers PUT route.
    // pub fn put<Q, P, H, S>(mut self, path: P, handler: H) -> Self
    // where
    //     Q: 'static,
    //     P: Into<String>,
    //     H: HandlerTrait<Q, S>,
    // {
    //     self.routes.entry(Method::PUT).or_default().push(
    //         Route::new(path, BoxCloneService::new(handler.into_service_with_state(())))
    //             .expect("tried to register invalid PUT route"),
    //     );
    //     self
    // }

    // /// Registers DELETE route.
    // pub fn delete<Q, P, H, S>(mut self, path: P, handler: H) -> Self
    // where
    //     Q: 'static,
    //     P: Into<String>,
    //     H: HandlerTrait<Q, S>,
    // {
    //     self.routes.entry(Method::DELETE).or_default().push(
    //         Route::new(path, BoxCloneService::new(handler.into_service_with_state(())))
    //             .expect("tried to register invalid DELETE route"),
    //     );
    //     self
    // }

    // /// Registers new middleware.
    // pub fn middleware<M>(mut self, m: M) -> Self
    // where
    //     M: Middleware + 'static,
    // {
    //     self.middlewares.push(Box::new(m));
    //     self
    // }
}

const MESSAGE_SIZE: usize = 1024;

/// Takes TcpStream, reads whole content and parses it to a http request.
fn parse_request_from_tcp(stream: &mut TcpStream) -> anyhow::Result<Request<Body>> {
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
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);

    let b_inx = req.parse(&received).unwrap().unwrap();

    httparse_req_to_hyper_request(req, received[b_inx..].to_vec())
}

fn httparse_req_to_hyper_request(
    req: httparse::Request,
    body: Vec<u8>,
) -> anyhow::Result<hyper::Request<Body>> {
    let mut builder = hyper::Request::builder()
        .method(req.method.unwrap())
        .uri(req.path.unwrap());

    for header in req.headers {
        builder = builder.header(header.name, header.value);
    }

    Ok(builder.body(Body::from(body))?)
}

// #[cfg(test)]
mod tests {
    use crate::{
        handler::BoxCloneService,
        server::{HandlerTrait, Route},
    };

    #[test]
    fn test_should_fire_on_path() {
        fn handler() {}

        let r = Route::new(
            "/test",
            BoxCloneService::new(handler.into_service_with_state(())),
        )
        .expect("valid route");

        assert!(r.should_fire_on_path("/test"));
        assert!(!r.should_fire_on_path("/test/test"));
        assert!(!r.should_fire_on_path("/"));

        let r = Route::new("/test/<param1>", handler.into_service_with_state(()).into())
            .expect("valid route");

        assert!(!r.should_fire_on_path("/test"));
        assert!(r.should_fire_on_path("/test/test"));
        assert!(!r.should_fire_on_path("/"));

        let r = Route::new(
            "/test/<param1>/<param2>",
            handler.into_service_with_state(()).into(),
        )
        .expect("valid route");

        assert!(r.should_fire_on_path("/test/1/2"));
        assert!(!r.should_fire_on_path("/test/test"));
        assert!(!r.should_fire_on_path("/"));
    }
}
