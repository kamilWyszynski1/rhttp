use std::collections::HashMap;

use crate::http::{ProtocolVersion, Request, ResponseStatus};

pub trait Responder {
    fn respond_to(self, req: Request) -> anyhow::Result<Response>;
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
    pub protocol: ProtocolVersion,
    pub status: ResponseStatus,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
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
