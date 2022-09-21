use anyhow::bail;
use log::debug;
use std::{
    any,
    collections::HashMap,
    fmt::{Debug, Display},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};
pub struct EchoServer {
    host: String,
    port: u32,
}

#[derive(PartialEq, Eq)]
enum ProtocolVersion {
    HTTP10,
    HTTP11,
    HTTP2,
    HTTP3,
}

impl TryFrom<&str> for ProtocolVersion {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // http method is case sensitive
        match value.trim() {
            "HTTP/1.0" => Ok(Self::HTTP10),
            "HTTP/1.1" => Ok(Self::HTTP11),
            "HTTP/2.0" => Ok(Self::HTTP2),
            "HTTP/3.0" => Ok(Self::HTTP3),
            _ => bail!("invalid http protocol version: {}", value),
        }
    }
}

impl Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match *self {
            ProtocolVersion::HTTP10 => "HTTP/1.0",
            ProtocolVersion::HTTP11 => "HTTP/1.1",
            ProtocolVersion::HTTP2 => "HTTP/2.0",
            ProtocolVersion::HTTP3 => "HTTP/3.0",
        })
    }
}

impl Debug for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

// An HTTP status code (`status-code` in RFC 7230 et al.).
///
/// Constants are provided for known status codes.
///
/// Status code values in the range 100-999 (inclusive) are supported by this
/// type. Values in the range 100-599 are semantically classified by the most
/// significant digit. See [`StatusCode::is_success`], etc. Values above 599
/// are unclassified but allowed for legacy compatibility, though their use is
/// discouraged. Applications may interpret such values as protocol errors.
#[derive(Debug)]
enum ResponseStatus {
    /// 200 OK
    /// [[RFC7231, Section 6.3.1](https://tools.ietf.org/html/rfc7231#section-6.3.1)]
    Ok,

    /// 201 Created
    /// [[RFC7231, Section 6.3.2](https://tools.ietf.org/html/rfc7231#section-6.3.2)]
    Created,

    /// 400 Bad Request
    /// [[RFC7231, Section 6.5.1](https://tools.ietf.org/html/rfc7231#section-6.5.1)]
    BadRequest,

    /// 403 Forbidden
    /// [[RFC7231, Section 6.5.3](https://tools.ietf.org/html/rfc7231#section-6.5.3)]
    Forbidden,

    /// 404 Not Found
    /// [[RFC7231, Section 6.5.4](https://tools.ietf.org/html/rfc7231#section-6.5.4)]
    NotFound,
    //TODO: implement rest of response codes.
}

impl ResponseStatus {
    fn get_code_message(&self) -> (u16, String) {
        match *self {
            ResponseStatus::Ok => (200, "OK".into()),
            ResponseStatus::Created => (201, "Created".into()),
            ResponseStatus::BadRequest => (400, "Bad Request".into()),
            ResponseStatus::Forbidden => (403, "Forbidden".into()),
            ResponseStatus::NotFound => (404, "Not Found".into()),
        }
    }
}

/// Responses consist of the following elements:
///
/// * The version of the HTTP protocol they follow.
/// * A status code, indicating if the request was successful or not, and why.
/// * A status message, a non-authoritative short description of the status code.
/// * HTTP headers, like those for requests.
/// * Optionally, a body containing the fetched resource.
#[derive(Debug)]
struct Response {
    protocol: ProtocolVersion,
    status: ResponseStatus,
    headers: HashMap<String, String>,
    body: Option<String>,
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

        buf.push_str("\n");

        for (k, v) in self.headers {
            let _ = writeln!(&mut buf, "{}: {}", k, v);
        }

        if let Some(body) = self.body {
            buf.push_str("\n\n");
            buf.push_str(body.as_str())
        }

        debug!("response string: {}", buf);

        buf.into_bytes()
    }
}

/// HTTP defines a set of request methods to indicate the desired action to be performed
/// for a given resource. Although they can also be nouns, these request methods are sometimes
/// referred to as HTTP verbs. Each of them implements a different semantic, but some
/// common features are shared by a group of them: e.g. a request method can be safe, idempotent, or cacheable.
///
/// https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods
#[derive(Debug, PartialEq, Eq)]
enum Method {
    /// The HTTP GET method requests a representation of the specified resource.
    /// Requests using GET should only be used to request data (they shouldn't include data).
    ///
    /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/GET
    Get,

    /// The HTTP POST method sends data to the server.
    /// The type of the body of the request is indicated by the Content-Type header.
    ///
    /// The difference between PUT and POST is that PUT is idempotent: calling it
    /// once or several times successively has the same effect (that is no side effect),
    /// where successive identical POST may have additional effects, like passing an order several times.
    ///
    /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/POST
    Post,

    /// The HTTP PUT request method creates a new resource or replaces a
    /// representation of the target resource with the request payload.
    Put,

    /// The HTTP DELETE request method deletes the specified resource.
    ///
    /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/DELETE
    Delete,
}

impl TryFrom<&str> for Method {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // http method is case sensitive
        match value {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            "DELETE" => Ok(Self::Delete),
            "PUT" => Ok(Self::Put),
            _ => bail!("invalid http method: {}", value),
        }
    }
}

/// Representation of HTTP Request.
///
/// https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages#body
#[derive(Debug, PartialEq, Eq)]
struct Request {
    /// An HTTP method, a verb (like GET, PUT or POST) or a noun (like HEAD or OPTIONS), that describes
    /// the action to be performed. For example, GET indicates that a resource should be fetched or POST means
    /// that data is pushed to the server (creating or modifying a resource, or generating a temporary document to send back).
    method: Method,

    /// The request target, usually a URL, or the absolute path of the protocol, port,
    /// and domain are usually characterized by the request context. The format of this
    /// request target varies between different HTTP methods.
    url: String,

    /// The HTTP version, which defines the structure of the remaining message,
    /// acting as an indicator of the expected version to use for the response.
    version: ProtocolVersion,

    /// HTTP headers from a request follow the same basic structure of an HTTP header:
    /// a case-insensitive string followed by a colon (':') and a value whose structure depends
    /// upon the header. The whole header, including the value, consist of one single line, which can be quite long.
    ///
    /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages#headers
    headers: HashMap<String, String>,

    /// The final part of the request is its body. Not all requests have one: requests fetching resources,
    /// like GET, HEAD, DELETE, or OPTIONS, usually don't need one. Some requests send data to the server in
    /// order to update it: as often the case with POST requests (containing HTML form data).
    ///
    /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages#body
    body: Vec<u8>,
}

impl Request {
    pub fn parse(s: String) -> anyhow::Result<Self> {
        let mut lines = s.split("\r\n");

        // parse request line
        let mut request_line = lines.next().unwrap().split(' ');
        let method: Method = request_line.next().unwrap().try_into()?;

        let mut request = Self {
            method,
            url: String::new(),
            version: ProtocolVersion::HTTP11, // default protocol version.
            headers: HashMap::new(),
            body: Vec::new(),
        };

        if let Some(rest) = request_line.next() {
            request.url = rest.trim().to_string();

            if let Some(rest) = request_line.next() {
                request.version = rest.trim().try_into()?;
            }
        }

        // parse headers
        while let Some(next) = lines.next() {
            if next.is_empty() {
                break;
            }
            match next.split_once(':') {
                Some((key, value)) => {
                    request
                        .headers
                        .insert(key.trim().to_string(), value.trim().to_string());
                }
                None => {
                    break;
                }
            }
        }

        // parse body
        let mut body = String::new();
        while let Some(next) = lines.next() {
            if next.is_empty() {
                break;
            }
            body.push_str(next);
        }
        if !body.is_empty() {
            request.body = body.into()
        }

        Ok(request)
    }
}

impl EchoServer {
    pub fn new(host: impl Into<String>, port: u32) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port))?;

        for stream in listener.incoming() {
            let stream = stream?;
            thread::spawn(move || {
                handle_connection_http(stream).expect("could not handle connection")
            });
        }
        Ok(())
    }
}

const MESSAGE_SIZE: usize = 1024;

fn handle_connection_http(mut stream: TcpStream) -> anyhow::Result<()> {
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

    let request = Request::parse(String::from_utf8(received)?)?;

    debug!("{:?}", request);

    let response = Response {
        protocol: ProtocolVersion::HTTP10,
        status_code: 200,
        status_message: "OK".into(),
        headers: HashMap::from([
            ("Content-Type".into(), "text/html".into()),
            ("Server".into(), "My Own".into()),
        ]),
        body: None,
    };

    println!("responding with: {:?}", response);

    let response_bytes: Vec<u8> = response.into();
    stream.write_all(&response_bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Method, ProtocolVersion, Request};

    #[test]
    fn test_request_parse() {
        let content = r#"POST /api/authors HTTP/1.1
Host: myWebApi.com
Content-Type: application/json
Cache-Control: no-cache

{
     "Name": "Felipe Gavilán",
     "Age": 999
}"#;

        let request = Request::parse(content.to_string()).expect("failed to parse request");
        assert_eq!(
            request,
            Request {
                method: Method::Post,
                url: "/api/authors".into(),
                version: ProtocolVersion::HTTP11,
                headers: HashMap::from([
                    ("Host".into(), "myWebApi.com".into()),
                    ("Content-Type".into(), "application/json".into()),
                    ("Cache-Control".into(), "no-cache".into()),
                ]),
                body: r#"{
                    "Name": "Felipe Gavilán",
                    "Age": 999
               }"#
                .into()
            }
        )
    }
}
