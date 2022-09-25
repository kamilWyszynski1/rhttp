use std::{collections::HashMap, str::FromStr};

use anyhow::{Context, Ok};
use hyper::{
    body::Bytes,
    header::{HeaderName, CONTENT_TYPE, HOST},
    http::{request::Parts, HeaderValue},
    Body, HeaderMap, Request,
};
use serde::de::DeserializeOwned;

mod private {
    #[derive(Debug, Clone, Copy)]
    pub enum ViaRequest {}
}

/// Allows various types to be created from Request.
pub trait FromRequest<B, M = private::ViaRequest>: Sized {
    fn from_request(req: Request<B>) -> anyhow::Result<Self>;
}

/// Implement FromRequest for every variant of Request<B>.
impl<B> FromRequest<B> for Request<B> {
    fn from_request(req: Request<B>) -> anyhow::Result<Self> {
        Ok(req)
    }
}

/// Implement FromRequest for String for B in Body variant.
///
/// This allows to create handler like that:
///
/// ```rust
/// fn handler(s: String) {}
/// ```
impl FromRequest<Body> for String {
    fn from_request(req: Request<Body>) -> anyhow::Result<Self> {
        let bytes: Bytes = futures_executor::block_on(hyper::body::to_bytes(req.into_body()))?;
        let string = std::str::from_utf8(&bytes)?.to_owned();

        Ok(string)
    }
}

/// Placeholder for value that can be deserialized from JSON.
/// It implements FromRequest<Body> in order to allow user quick and easy usage
/// of deserializable structs as body types in their handlers.
///
/// ```rust
/// use serde::Deserialize;
/// use core::request::Json;
///
/// #[derive(Deserialize)]
/// struct OwnBody {
///     val: String,
///     val2: i32
/// }
///
/// fn handler(Json(body): Json<OwnBody>) {}
/// ```
pub struct Json<T>(pub T);

impl<T> FromRequest<Body> for Json<T>
where
    T: DeserializeOwned,
{
    fn from_request(req: Request<Body>) -> anyhow::Result<Self> {
        let bytes: Bytes = futures_executor::block_on(hyper::body::to_bytes(req.into_body()))?;
        let deserializer = &mut serde_json::Deserializer::from_slice(&bytes);

        let value = T::deserialize(deserializer)?;
        Ok(Json(value))
    }
}

/// Trait is implemented for types that can be turned from HeaderMap by specific key.
///
/// Multiple, commonly used headers from hyper crate implements this trait.
/// That allows to deserialize them straight into handler's param.
///
/// ```rust
/// use core::request::ContentType;
///
/// fn handler_header(ContentType(content_type): ContentType) -> anyhow::Result<String> {
///     Ok(content_type)
/// }
/// ```
pub trait TypedHeader: Sized {
    /// Returns header's key.
    fn key() -> HeaderName;

    /// Tries to create Self from HeaderValue.
    fn try_from_header_value(header_value: &HeaderValue) -> anyhow::Result<Self>;

    /// Default implementation that uses `key` and `try_from_header_value` functions
    /// to turn `map: HeaderMap<HeaderValue>` into `anyhow::Result<Self>`.
    fn try_from_header_map(map: &HeaderMap<HeaderValue>) -> anyhow::Result<Self> {
        Self::try_from_header_value(map.get(Self::key()).context("header not found")?)
    }
}

/// Macro for faster TypedHeaderTrait implementations.
macro_rules! derive_header {
    ($type:ident(_), name: $name:ident) => {
        impl TypedHeader for $type {
            fn key() -> HeaderName {
                $name
            }

            fn try_from_header_value(header_value: &HeaderValue) -> anyhow::Result<Self> {
                Ok($type(header_value.to_str()?.to_string()))
            }
        }
    };
}

// TODO: implement more headers.
pub struct ContentType(pub String);
derive_header!(ContentType(_), name: CONTENT_TYPE);

pub struct Host(pub String);
derive_header!(Host(_), name: HOST);

/// Types that implements this trait can be created from request's parts.
/// This trait shouldn't be used directly, rather than that use some of its
/// implementations like TypedHeader or PathParam.
pub trait FromRequestParts: Sized {
    fn from_request_parts(parts: &mut Parts) -> anyhow::Result<Self>;
}

/// Implement FromRequestParts for every type that implements TypedHeader trait.  
impl<T> FromRequestParts for T
where
    T: TypedHeader,
{
    fn from_request_parts(parts: &mut Parts) -> anyhow::Result<Self> {
        T::try_from_header_map(&parts.headers)
    }
}

/// Implements FromRequest for every type that implements FromRequestParts trait.
/// This implementation allows to use ContentType, Host, etc. structs as parameters
/// in server's handlers.
impl<T, B> FromRequest<B> for T
where
    T: FromRequestParts,
{
    fn from_request(req: Request<B>) -> anyhow::Result<Self> {
        let (mut b, _) = req.into_parts();
        T::from_request_parts(&mut b)
    }
}

/// PathParamOrdering wrapper type for storing state of what params were already read.
#[derive(Default, Clone, Copy)]
struct PathParamOrdering(usize);

impl PathParamOrdering {
    fn increment(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Container for values that are retrieved from query params.
/// If inner type implements FromStr trait this container can be used
/// in handler to get direct access for query param value.
///
/// ```rust
/// use core::request::PathParam;
/// use core::server::Server;
///
/// fn handler(PathParam(name): PathParam<String>) {}
///
/// Server::new("127.0.0.1", 8080).get("/<name>", handler);
/// ```
pub struct PathParam<T>(pub T);

impl<T> FromRequestParts for PathParam<T>
where
    T: 'static,
    T: FromStr,
    <T as FromStr>::Err: std::error::Error + Sync + Send,
{
    fn from_request_parts(parts: &mut Parts) -> anyhow::Result<Self> {
        let path = parts.uri.to_string();

        let segments = parts
            .extensions
            .get::<HashMap<usize, usize>>()
            .context("no segments provided")?
            .clone();

        let binding = PathParamOrdering(0);
        let ordering = parts
            .extensions
            .get::<PathParamOrdering>()
            .unwrap_or(&binding);

        let order_in_path = segments
            .get(&ordering.0)
            .context("no value for wanted ordering")?;

        let value_to_parse = path
            .split('/')
            .nth(*order_in_path + 1) // +1 because we have to skip first '/' as path starts with that.
            .context("invalid value from a string")?;

        let parsed = PathParam(T::from_str(value_to_parse)?);

        parts.extensions.insert(ordering.increment());

        Ok(parsed)
    }
}
