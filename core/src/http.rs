use anyhow::bail;
use std::fmt::{Debug, Display};

// #[derive(PartialEq, Copy, Clone, Eq)]
// pub enum ProtocolVersion {
//     HTTP10,
//     HTTP11,
//     HTTP2,
//     HTTP3,
// }

// impl Default for ProtocolVersion {
//     fn default() -> Self {
//         Self::HTTP11
//     }
// }

// impl TryFrom<&str> for ProtocolVersion {
//     type Error = anyhow::Error;

//     fn try_from(value: &str) -> Result<Self, Self::Error> {
//         // http method is case sensitive
//         match value.trim() {
//             "HTTP/1.0" => Ok(Self::HTTP10),
//             "HTTP/1.1" => Ok(Self::HTTP11),
//             "HTTP/2.0" => Ok(Self::HTTP2),
//             "HTTP/3.0" => Ok(Self::HTTP3),
//             _ => bail!("invalid http protocol version: {}", value),
//         }
//     }
// }

// impl Display for ProtocolVersion {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_str(match *self {
//             ProtocolVersion::HTTP10 => "HTTP/1.0",
//             ProtocolVersion::HTTP11 => "HTTP/1.1",
//             ProtocolVersion::HTTP2 => "HTTP/2.0",
//             ProtocolVersion::HTTP3 => "HTTP/3.0",
//         })
//     }
// }

// impl Debug for ProtocolVersion {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", self)
//     }
// }

// // An HTTP status code (`status-code` in RFC 7230 et al.).
// ///
// /// Constants are provided for known status codes.
// ///
// /// Status code values in the range 100-999 (inclusive) are supported by this
// /// type. Values in the range 100-599 are semantically classified by the most
// /// significant digit. See [`StatusCode::is_success`], etc. Values above 599
// /// are unclassified but allowed for legacy compatibility, though their use is
// /// discouraged. Applications may interpret such values as protocol errors.
// #[derive(Debug, Clone, Copy)]
// pub enum Status {
//     /// 200 OK
//     /// [[RFC7231, Section 6.3.1](https://tools.ietf.org/html/rfc7231#section-6.3.1)]
//     Ok,

//     /// 201 Created
//     /// [[RFC7231, Section 6.3.2](https://tools.ietf.org/html/rfc7231#section-6.3.2)]
//     Created,

//     /// 400 Bad Request
//     /// [[RFC7231, Section 6.5.1](https://tools.ietf.org/html/rfc7231#section-6.5.1)]
//     BadRequest,

//     /// 403 Forbidden
//     /// [[RFC7231, Section 6.5.3](https://tools.ietf.org/html/rfc7231#section-6.5.3)]
//     Forbidden,

//     /// 404 Not Found
//     /// [[RFC7231, Section 6.5.4](https://tools.ietf.org/html/rfc7231#section-6.5.4)]
//     NotFound,

//     /// 500 Internal Server Error
//     /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/500
//     InternalServerError,
// }

// impl Status {
//     pub fn get_code_message(&self) -> (u16, String) {
//         match *self {
//             Status::Ok => (200, "OK".into()),
//             Status::Created => (201, "Created".into()),
//             Status::BadRequest => (400, "Bad Request".into()),
//             Status::Forbidden => (403, "Forbidden".into()),
//             Status::NotFound => (404, "Not Found".into()),
//             Status::InternalServerError => (500, "Internal Server Error".into()),
//         }
//     }
// }

// HTTP defines a set of request methods to indicate the desired action to be performed
// for a given resource. Although they can also be nouns, these request methods are sometimes
// referred to as HTTP verbs. Each of them implements a different semantic, but some
// common features are shared by a group of them: e.g. a request method can be safe, idempotent, or cacheable.
//
// https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods
// #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
// pub enum Method {
//     /// The HTTP GET method requests a representation of the specified resource.
//     /// Requests using GET should only be used to request data (they shouldn't include data).
//     ///
//     /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/GET
//     Get,

//     /// The HTTP POST method sends data to the server.
//     /// The type of the body of the request is indicated by the Content-Type header.
//     ///
//     /// The difference between PUT and POST is that PUT is idempotent: calling it
//     /// once or several times successively has the same effect (that is no side effect),
//     /// where successive identical POST may have additional effects, like passing an order several times.
//     ///
//     /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/POST
//     Post,

//     /// The HTTP PUT request method creates a new resource or replaces a
//     /// representation of the target resource with the request payload.
//     Put,

//     /// The HTTP DELETE request method deletes the specified resource.
//     ///
//     /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/DELETE
//     Delete,
// }

// impl Default for Method {
//     fn default() -> Self {
//         Self::Get
//     }
// }

// impl TryFrom<&str> for Method {
//     type Error = anyhow::Error;

//     fn try_from(value: &str) -> Result<Self, Self::Error> {
//         // http method is case sensitive
//         match value {
//             "GET" => Ok(Self::Get),
//             "POST" => Ok(Self::Post),
//             "DELETE" => Ok(Self::Delete),
//             "PUT" => Ok(Self::Put),
//             _ => bail!("invalid http method: {}", value),
//         }
//     }
// }
