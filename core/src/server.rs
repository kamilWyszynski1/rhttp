use crate::{
    handler::Service,
    response::{response_to_bytes, Response},
};
use anyhow::Ok;
use hyper::{Body, Request};
use log::error;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
};

#[derive(Default)]
pub struct Server<V> {
    host: String,
    port: u32,

    service: Option<V>,
}

impl<V> Server<V>
where
    V: Service<Request<Body>> + Send + Sync + 'static,
{
    pub fn new(host: impl Into<String>, port: u32) -> Self {
        Self {
            host: host.into(),
            port,
            service: None,
        }
    }

    pub fn with_service(mut self, service: V) -> Self {
        self.service = Some(service);
        self
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
    pub fn fire<W>(&self, request: Request<Body>) -> anyhow::Result<Response>
    where
        W: std::io::Write,
    {
        Ok(self.service.as_ref().unwrap().call(request))
    }
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

#[cfg(test)]
mod tests {
    use crate::handler::BoxCloneService;
    use crate::handler::HandlerTrait;
    use crate::route::Route;

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
