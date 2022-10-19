use crate::{
    handler::{BoxCloneService, Service},
    middleware::Middleware,
    response::Response,
};
use anyhow::bail;
use hyper::{Body, Method, Request};
use std::{collections::HashMap, sync::Arc};

/// RouteGroup enables grouping endpoints with common prefix path.
///
/// ```
/// use core::route::RouteGroup;
/// use core::server::Server;
/// use crate::core::handler::HandlerTraitWithoutState;
///
/// let v1 = RouteGroup::new("/v1").get("/user", (|| "v1").into_service());
/// let v2 = RouteGroup::new("/v2").get("/user", (|| "v2").into_service());
///
/// Server::new("", 8080).groups(vec![v1, v2]).run();
/// ```
#[derive(Clone)]
pub struct RouteGroup {
    prefix: String,
    routes: HashMap<Method, Vec<Route>>,

    /// Registered middlewares on specific RouteGroup. These will
    /// be passed to each route.
    middlewares: Vec<Box<dyn Middleware>>,
}

impl RouteGroup {
    pub fn new<P>(prefix: P) -> Self
    where
        P: ToString,
    {
        Self {
            prefix: prefix.to_string(),
            routes: HashMap::new(),
            middlewares: vec![],
        }
    }

    /// Injects middlewares for registered routes and returns them.
    pub fn routes(&self) -> HashMap<Method, Vec<Route>> {
        let mut routes = self.routes.clone();

        for (_, rs) in routes.iter_mut() {
            for r in rs {
                r.middlewares = self.middlewares.clone();
            }
        }
        routes
    }

    fn construct_path<P: ToString>(&self, path: P) -> String {
        format!("{}{}", self.prefix, path.to_string())
    }

    /// Registers GET route.
    pub fn get<P, V>(mut self, path: P, service: V) -> Self
    where
        P: ToString,
        V: Service<Request<Body>, Response = Response> + Send + Sync + 'static,
    {
        let path = self.construct_path(path);

        self.routes.entry(Method::GET).or_default().push(
            Route::new(path, BoxCloneService::new(service))
                .expect("tried to register invalid GET route"),
        );
        self
    }

    /// Registers POST route.
    pub fn post<P, V>(mut self, path: P, service: V) -> Self
    where
        P: ToString,
        V: Service<Request<Body>, Response = Response> + Send + Sync + 'static,
    {
        let path = self.construct_path(path);

        self.routes.entry(Method::POST).or_default().push(
            Route::new(path, BoxCloneService::new(service))
                .expect("tried to register invalid POST route"),
        );
        self
    }

    /// Registers PUT route.
    pub fn put<P, V>(mut self, path: P, service: V) -> Self
    where
        P: ToString,
        V: Service<Request<Body>, Response = Response> + Send + Sync + 'static,
    {
        let path = self.construct_path(path);

        self.routes.entry(Method::PUT).or_default().push(
            Route::new(path, BoxCloneService::new(service))
                .expect("tried to register invalid PUT route"),
        );
        self
    }

    /// Registers DELETE route.
    pub fn delete<P, V>(mut self, path: P, service: V) -> Self
    where
        P: ToString,
        V: Service<Request<Body>, Response = Response> + Send + Sync + 'static,
    {
        let path = self.construct_path(path);

        self.routes.entry(Method::DELETE).or_default().push(
            Route::new(path, BoxCloneService::new(service))
                .expect("tried to register invalid DELETE route"),
        );
        self
    }

    /// Registers new middleware.
    /// When calling `RouteProvider::routes` every registered middleware
    /// will be copied into route.
    pub fn middleware<M>(mut self, m: M) -> Self
    where
        M: Middleware + 'static,
    {
        self.middlewares.push(Box::new(m));
        self
    }
}

/// Smallest unit of routing logic. Should not be constructed directly.
/// Either use method on `core::server::Server` directly or create those
/// routes using `core::route::RouteGroup` and `core::server::Server::merge_routes` method.
#[derive(Clone)]
pub struct Route {
    pub service: Arc<BoxCloneService<Request<Body>, Response>>,

    /// Contains metadata about registered route.
    pub metadata: RouteMetadata,

    /// Middlewares for single route.
    pub middlewares: Vec<Box<dyn Middleware>>,
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
            middlewares: vec![],
        })
    }

    pub fn middlewares(mut self, middlewares: Vec<Box<dyn Middleware>>) -> Self {
        self.middlewares = middlewares;
        self
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

    pub fn fire(&self, mut request: Request<Body>) -> anyhow::Result<Response> {
        for m in &self.middlewares {
            m.on_request(&mut request)?;
        }

        let mut response = self.service.0.call(request);

        for m in &self.middlewares {
            m.on_response(&mut response)?;
        }
        Ok(response)
    }
}

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
