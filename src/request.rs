use std::{collections::HashMap, fmt::Debug, str::FromStr};

use crate::http::{Method, ProtocolVersion};
use anyhow::{bail, Context};
use log::debug;

pub trait FromParam: Sized {
    type Inner;

    fn from_param(param: String) -> anyhow::Result<Self::Inner>;
}

// impl FromParam for String {
//     type Inner = Self;

//     fn from_param(param: String) -> anyhow::Result<Self::Inner> {
//         Ok(param)
//     }
// }

// impl<S> FromParam for S
// where
//     S: TryFrom<String>,
//     <S as TryFrom<String>>::Error: Debug,
// {
//     type Inner = S;

//     fn from_param(param: String) -> anyhow::Result<Self::Inner> {
//         let a = S::try_from(param).unwrap();
//         Ok(a)
//     }
// }

impl<S> FromParam for S
where
    S: FromStr,
    <S as FromStr>::Err: Debug,
{
    type Inner = S;

    fn from_param(param: String) -> anyhow::Result<Self::Inner> {
        match S::from_str(&param) {
            Ok(value) => Ok(value),
            Err(e) => bail!("could not convert types: {:?}", e),
        }
    }
}

/// Representation of HTTP Request.
///
/// https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages#body
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// An HTTP method, a verb (like GET, PUT or POST) or a noun (like HEAD or OPTIONS), that describes
    /// the action to be performed. For example, GET indicates that a resource should be fetched or POST means
    /// that data is pushed to the server (creating or modifying a resource, or generating a temporary document to send back).
    pub method: Method,

    /// The request target, usually a URL, or the absolute path of the protocol, port,
    /// and domain are usually characterized by the request context. The format of this
    /// request target varies between different HTTP methods.
    pub url: String,

    /// The HTTP version, which defines the structure of the remaining message,
    /// acting as an indicator of the expected version to use for the response.
    pub version: ProtocolVersion,

    /// HTTP headers from a request follow the same basic structure of an HTTP header:
    /// a case-insensitive string followed by a colon (':') and a value whose structure depends
    /// upon the header. The whole header, including the value, consist of one single line, which can be quite long.
    ///
    /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages#headers
    pub headers: HashMap<String, String>,

    /// The final part of the request is its body. Not all requests have one: requests fetching resources,
    /// like GET, HEAD, DELETE, or OPTIONS, usually don't need one. Some requests send data to the server in
    /// order to update it: as often the case with POST requests (containing HTML form data).
    ///
    /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages#body
    pub body: Vec<u8>,

    metadata: RequestMetadata,
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
            ..Default::default()
        };

        if let Some(rest) = request_line.next() {
            request.url = rest.trim().to_string();
            request.metadata = RequestMetadata::from_url(&request.url)?;

            if let Some(rest) = request_line.next() {
                request.version = rest.trim().try_into()?;
                debug!("version: {}", request.version);
            }
        }

        // parse headers
        for next in lines.by_ref() {
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
        for next in lines {
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

    /// This function is called before handler execution.
    /// We need to somehow provide information about how registered path was structured
    /// so we can use this information during query params retrieval.
    pub fn inject_params_seqments(&mut self, params_segments: HashMap<String, u8>) {
        debug!("injecting: {:?}", params_segments);
        self.metadata.params_segments = params_segments;
    }

    /// Tries to return Inner type of FromParam type specific when calling query.
    /// Injected params segments indicates index of RequestMetadata's segment to get.
    ///
    /// ```rust
    /// fn handler(req: Request) {
    ///     let _: String = req.query::<String>("param1").unwrap();
    /// }
    ///
    /// Server::new("127.0.0.1", 8080)
    ///     .get("/test/<param1>", handler)
    ///     .run()
    ///
    /// ```
    pub fn query<F: FromParam>(&self, query_param: &str) -> anyhow::Result<F::Inner> {
        debug!(
            "query - starting with {:?} segments",
            self.metadata.segments
        );
        let param = self
            .metadata
            .segments
            .get(
                self.metadata
                    .params_segments
                    .get(query_param)
                    .context("there's not wanted param's index")?,
            )
            .context("there's no wanted param")?;

        F::from_param(param.clone())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct RequestMetadata {
    /// Holds indexes of path's segments.
    ///
    /// `/test/hello/world` - > {0: "test": ,1: "hello", 2: "world"}
    segments: HashMap<u8, String>,

    /// Holds params' segments names. This map is created during handler registration.
    ///
    /// `/test/<param1>/<param2>` - ["param1", "param2"].
    params_segments: HashMap<String, u8>,
}

impl RequestMetadata {
    fn from_url(s: &str) -> anyhow::Result<Self> {
        Ok(Self {
            segments: parse_segments(s.to_string())?
                .iter_mut()
                .map(|(k, v)| (*v, k.clone()))
                .collect(),
            ..Default::default()
        })
    }
}

pub fn parse_segments(path: String) -> anyhow::Result<HashMap<String, u8>> {
    let mut segments: HashMap<String, u8> = HashMap::new();

    let mut split = path.split('/');
    if split.next().is_none() {
        bail!("invalid path")
    }

    // call next() one time to skip first "" value.
    split.enumerate().for_each(|(inx, val)| {
        segments.insert(val.to_string(), inx as u8);
    });

    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::Request;
    use crate::http::{Method, ProtocolVersion};
    use std::collections::HashMap;

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
                .into(),
                ..Default::default()
            }
        )
    }
}
