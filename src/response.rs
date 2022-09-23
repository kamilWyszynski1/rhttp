use std::collections::HashMap;

use crate::{
    http::{ProtocolVersion, Status},
    request::Request,
};

pub trait Responder {
    fn respond_to(self, req: Request) -> anyhow::Result<Response>;
}

/// Responder implementation for '()', returns default Response (200, HTTP1.1).
///
/// ```rust
/// fn handler(_req: Request) {}
///
/// Server::new().get("/", handler).run()?;
///
/// ```
impl Responder for () {
    fn respond_to(self, _req: Request) -> anyhow::Result<Response> {
        Ok(Response::default())
    }
}

/// Response by defualt should implement Responder.
impl Responder for Response {
    fn respond_to(self, _req: Request) -> anyhow::Result<Response> {
        Ok(self)
    }
}

/// Returns Response with stringified self as a body, returns default Response (200, HTTP1.1).
///
/// ```rust
/// fn handler(_req: Request) -> &'static str {
///     "hello"
/// }
///
/// Server::new().get("/", handler).run()?;
///
/// ```
impl<'a> Responder for &'a str {
    fn respond_to(self, _req: Request) -> anyhow::Result<Response> {
        Ok(Response::build().body(self.to_string()).finalize())
    }
}

/// Returns Response with stringified self as a body, returns default Response (200, HTTP1.1).
///
/// ```rust
/// fn handler(_req: Request) -> String {
///     "hello".into()
/// }
///
/// Server::new().get("/", handler).run()?;
///
/// ```
impl Responder for String {
    fn respond_to(self, _req: Request) -> anyhow::Result<Response> {
        Ok(Response::build().body(self).finalize())
    }
}

impl<T> Responder for anyhow::Result<T>
where
    T: Responder,
{
    fn respond_to(self, req: Request) -> anyhow::Result<Response> {
        match self {
            Ok(r) => Ok(r.respond_to(req)?),
            Err(e) => Ok(Response::build()
                .status(Status::InternalServerError)
                .body(e.to_string())
                .finalize()),
        }
    }
}

/// Builder for Response struct.
#[derive(Default, Debug)]
pub struct ResponseBuilder {
    response: Response,
}

impl ResponseBuilder {
    /// Sets protocol field.
    pub fn protocol(&mut self, protocol: ProtocolVersion) -> &mut Self {
        self.response.protocol = protocol;
        self
    }

    /// Sets status field.
    pub fn status(&mut self, status: Status) -> &mut Self {
        self.response.status = status;
        self
    }

    /// Add single header to headers field.
    /// Call multiple times for multiple headers.
    pub fn header(&mut self, key: String, value: String) -> &mut Self {
        self.response.headers.insert(key, value);
        self
    }

    /// Sets body field.
    pub fn body(&mut self, body: String) -> &mut Self {
        self.response.body = Some(body);
        self
    }

    /// Returns built Response leaving empty at that place.
    pub fn finalize(&mut self) -> Response {
        std::mem::take(&mut self.response)
    }
}

/// Responses consist of the following elements:
///
/// * The version of the HTTP protocol they follow.
/// * A status code, indicating if the request was successful or not, and why.
/// * A status message, a non-authoritative short description of the status code.
/// * HTTP headers, like those for requests.
/// * Optionally, a body containing the fetched resource.
#[derive(Debug, Clone)]
pub struct Response {
    /// The HTTP protocol version used.
    pub protocol: ProtocolVersion,

    /// HTTP status returned.
    pub status: Status,

    /// HTTP headers returned.
    pub headers: HashMap<String, String>,

    /// HTTP body content returned.
    pub body: Option<String>,
}

impl Response {
    fn build() -> ResponseBuilder {
        ResponseBuilder::default()
    }
}

impl Default for Response {
    fn default() -> Self {
        Self {
            protocol: ProtocolVersion::HTTP11,
            status: Status::Ok,
            headers: HashMap::default(),
            body: None,
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Vec<u8>> for Response {
    fn into(self) -> Vec<u8> {
        use std::fmt::Write as _; // import without risk of name clashing

        let mut buf = String::new();

        let (status_code, status_message) = self.status.get_code_message();

        let _ = write!(
            &mut buf,
            "{} {} {}",
            self.protocol, status_code, status_message
        );

        buf.push('\n');

        for (k, v) in self.headers {
            let _ = writeln!(&mut buf, "{}: {}", k, v);
        }

        if let Some(body) = self.body {
            buf.push_str("\n\n");
            buf.push_str(body.as_str())
        }

        buf.into_bytes()
    }
}
