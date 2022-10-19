use bytes::{BufMut, Bytes, BytesMut};
use hyper::{Body, StatusCode};

pub type Response = hyper::Response<Body>;

pub trait Responder {
    fn into_response(self) -> anyhow::Result<Response>;
}

pub fn body_to_bytes(body: Body) -> anyhow::Result<Bytes> {
    let body_bytes = futures_executor::block_on(hyper::body::to_bytes(body))?;
    Ok(body_bytes)
}

pub fn response_to_bytes(response: Response) -> anyhow::Result<Vec<u8>> {
    use std::fmt::Write as _; // import without risk of name clashing

    let mut buffer = BytesMut::with_capacity(1024 * 8); // 8kB
    let status = response.status();
    let (status_code, status_message) = (status.as_u16(), status.as_str());

    let _ = write!(
        &mut buffer,
        "{:?} {} {}",
        response.version(),
        status_code,
        status_message
    );

    buffer.write_char('\n')?;

    for (k, v) in response.headers() {
        let _ = writeln!(&mut buffer, "{}: ", k);
        buffer.put(v.as_bytes());
        buffer.write_char('\n')?;
    }

    let body_bytes = body_to_bytes(response.into_body())?;
    if body_bytes.is_empty() {
        return Ok(buffer.to_vec());
    }

    buffer.write_str("\n\n")?;
    buffer.put(body_bytes);

    Ok(buffer.to_vec())
}

/// Responder implementation for '()', returns default Response (200, HTTP1.1).
///
/// ```rust
/// use core::server::Server;
/// use hyper::Request;
/// use crate::core::handler::HandlerTraitWithoutState;
///
/// fn handler() {}
///
/// Server::new("127.0.0.1", 8080).get("/", handler.into_service());
/// ```
impl Responder for () {
    fn into_response(self) -> anyhow::Result<Response> {
        Ok(Response::default())
    }
}

/// Response by defualt should implement Responder.
impl Responder for Response {
    fn into_response(self) -> anyhow::Result<Response> {
        Ok(self)
    }
}

/// Returns Response with stringified self as a body, returns default Response (200, HTTP1.1).
///
/// ```rust
/// use core::server::Server;
/// use hyper::Request;
/// use crate::core::handler::HandlerTraitWithoutState;
///
/// fn handler() -> &'static str {
///     "hello"
/// }
///
/// Server::new("127.0.0.1", 8080).get("/", handler.into_service());
///
/// ```
impl<'a> Responder for &'a str {
    fn into_response(self) -> anyhow::Result<Response> {
        Ok(hyper::Response::builder().body(Body::from(self.to_string()))?)
    }
}

/// Returns Response with stringified self as a body, returns default Response (200, HTTP1.1).
///
/// ```rust
/// use core::server::Server;
/// use hyper::Request;
/// use crate::core::handler::HandlerTraitWithoutState;
///
/// fn handler() -> String {
///     "hello".into()
/// }
///
/// Server::new("127.0.0.1", 8080).get("/", handler.into_service());
/// ```
impl Responder for String {
    fn into_response(self) -> anyhow::Result<Response> {
        Ok(hyper::Response::builder().body(Body::from(self))?)
    }
}

impl Responder for i32 {
    fn into_response(self) -> anyhow::Result<Response> {
        self.to_string().into_response()
    }
}

impl Responder for bool {
    fn into_response(self) -> anyhow::Result<Response> {
        self.to_string().into_response()
    }
}

impl<T> Responder for anyhow::Result<T>
where
    T: Responder,
{
    fn into_response(self) -> anyhow::Result<Response> {
        match self {
            Ok(r) => Ok(r.into_response()?),
            Err(e) => Ok(hyper::Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(e.to_string()))?),
        }
    }
}
