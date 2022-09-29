use core::{
    handler::{BoxCloneService, Service},
    response::{body_to_bytes, Response},
    server::{Route, Server},
};
use hyper::{Body, Method, Request};
use std::collections::HashMap;

struct Client {
    server: Server,
}

impl Client {
    fn new(routes: HashMap<Method, Vec<Route>>) -> anyhow::Result<Self> {
        Ok(Self {
            server: Server::new_with_routes(routes),
        })
    }

    fn send(&self, request: Request<Body>) -> anyhow::Result<Response> {
        self.server.fire::<std::io::BufWriter<Vec<u8>>>(request)
    }
}

pub struct TestCaseBuilder {
    name: Option<String>,

    /// Path for registered handler.
    path: String,

    /// Url of a request.
    url: String,
    method: Method,
    handler: BoxCloneService<Request<Body>, Response>,

    body: Option<Body>,
    headers: Option<HashMap<String, String>>,
    result: Option<Vec<u8>>,
}

impl TestCaseBuilder {
    pub fn new<T, V>(path: T, url: T, method: Method, service: V) -> Self
    where
        T: ToString,
        V: Service<Request<Body>, Response = Response> + Send + Sync + 'static,
    {
        Self {
            name: None,
            path: path.to_string(),
            url: url.to_string(),
            method,
            handler: BoxCloneService::new(service),
            headers: None,
            body: None,
            result: None,
        }
    }

    pub fn name<T: ToString>(mut self, name: T) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn body(mut self, body: Body) -> Self {
        self.body = Some(body);
        self
    }

    pub fn result(mut self, result: &str) -> Self {
        self.result = Some(result.as_bytes().to_vec());
        self
    }

    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: ToString,
        V: ToString,
    {
        let mut m = self.headers.unwrap_or_default();
        m.insert(key.to_string(), value.to_string());
        self.headers = Some(m);
        self
    }

    pub fn run(self) -> anyhow::Result<()> {
        let mut builder = Request::builder().uri(self.url).method(self.method.clone());

        for (key, value) in self.headers.unwrap_or_default().into_iter() {
            builder = builder.header(key, value);
        }

        let req = builder.body(self.body.unwrap_or_default())?;

        let route = Route::new(self.path, self.handler)?;
        let client = Client::new(HashMap::from([(self.method, vec![route])]))?;

        let res: Response = client.send(req)?;
        let body_bytes: Vec<u8> = body_to_bytes(res.into_body())?.into();

        assert_eq!(
            body_bytes,
            self.result.unwrap_or_default(),
            "test case {}",
            self.name.unwrap_or_default()
        );

        Ok(())
    }
}
