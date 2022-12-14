use crate::{
    handler::{BoxCloneService, HandlerTrait, Service},
    middleware::Middleware,
    response::Response,
};
use anyhow::{bail, Context};
use hyper::{Body, Method, Request};
use std::{collections::HashMap, sync::Arc};

/// Main entity that delegates all routing in an application.
#[derive(Clone)]
pub struct Router<S> {
    state: Arc<S>,
    routes: HashMap<Method, Vec<Route>>,

    /// Registered middlewares that will be run during request handling.
    /// These are global middlewares, note that each route can have
    /// its own middleware so we can have different behaviors based on route.
    middlewares: Vec<Box<dyn Middleware>>,
}

impl Router<()> {
    pub fn default() -> Self {
        Self::with_state(())
    }
}

impl<S> Router<S> {
    fn call(&self, mut request: Request<Body>) -> anyhow::Result<Response> {
        let route = self
            .routes
            .get(request.method())
            .with_context(|| format!("not registered routes for {:?} method", request.method()))?
            .iter()
            .find(|route| route.should_fire_on_path(request.uri().path()))
            .context("no matching route")?;

        let extensions = request.extensions_mut();
        extensions.insert(route.metadata.param_segments.clone());

        let response = route.fire(request)?;

        Ok(response)
    }
}

impl<S> Router<S>
where
    S: Send + Sync + 'static,
{
    /// Creates new Router with given state. For that point we can only add handlers with coresponding state.
    ///
    /// ```
    /// use core::route::Router;
    /// use core::request::State;
    ///
    /// fn handler(state: State<i32>) {}
    ///
    /// let app = Router::with_state(100).get("/", handler);
    /// ```
    pub fn with_state(state: S) -> Self {
        Self {
            state: Arc::new(state),
            routes: HashMap::new(),
            middlewares: vec![],
        }
    }

    fn register_path<P, H, Q: 'static>(mut self, method: Method, path: P, handler: H) -> Self
    where
        P: ToString,
        H: HandlerTrait<Q, S>,
    {
        self.routes.entry(method).or_default().push(
            Route::new(
                path.to_string(),
                BoxCloneService::new(handler.into_service_with_state_arc(self.state.clone())),
            )
            .expect("tried to register invalid GET route"),
        );
        self
    }

    pub fn get<P, H, Q: 'static>(self, path: P, handler: H) -> Self
    where
        P: ToString,
        H: HandlerTrait<Q, S>,
    {
        self.register_path(Method::GET, path, handler)
    }

    pub fn post<P, H, Q: 'static>(self, path: P, handler: H) -> Self
    where
        P: ToString,
        H: HandlerTrait<Q, S>,
    {
        self.register_path(Method::POST, path, handler)
    }

    /// Takes vector of `route::RouteGroup` and adds them to already registerd routes.
    pub fn groups(mut self, groups: Vec<RouteGroup>) -> Self {
        groups.into_iter().for_each(|rg| {
            for (method, rs) in rg.routes() {
                for r in rs {
                    self.routes.entry(method.clone()).or_default().push(r);
                }
            }
        });
        self
    }
}

impl<S> Service<Request<Body>> for Router<S> {
    fn call(&self, req: Request<Body>) -> Response {
        match self.call(req) {
            Ok(response) => response,
            Err(err) => hyper::Response::builder()
                .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                .body(hyper::Body::from(err.to_string()))
                .unwrap(),
        }
    }
}

/// RouteGroup enables grouping endpoints with common prefix path.
///
/// ```
/// use core::route::RouteGroup;
/// use core::route::Router;
/// use crate::core::handler::HandlerTraitWithoutState;
///
/// let v1 = RouteGroup::new("/v1").get("/user", (|| "v1").into_service());
/// let v2 = RouteGroup::new("/v2").get("/user", (|| "v2").into_service());
///
/// Router::default().groups(vec![v1, v2]);
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
        V: Service<Request<Body>> + Send + Sync + 'static,
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
        V: Service<Request<Body>> + Send + Sync + 'static,
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
        V: Service<Request<Body>> + Send + Sync + 'static,
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
        V: Service<Request<Body>> + Send + Sync + 'static,
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
    pub service: Arc<BoxCloneService<Request<Body>>>,

    /// Contains metadata about registered route.
    pub metadata: RouteMetadata,

    /// Middlewares for single route.
    pub middlewares: Vec<Box<dyn Middleware>>,
}

impl Route {
    /// Creates new Route, tries to parse path into RouteMetadata.
    pub fn new<P>(path: P, handler: BoxCloneService<Request<Body>>) -> anyhow::Result<Self>
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
    /// `/test/<param1>/<param2>` - { 0: 1, 1: 2 }.
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
