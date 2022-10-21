use core::{
    handler::Service,
    response::{body_to_bytes, Response},
    server::Server,
};
use hyper::{Body, Method, Request};
use std::collections::HashMap;

struct Client<V> {
    server: Server<V>,
}

impl<V> Client<V>
where
    V: Service<Request<Body>> + Send + Sync + 'static,
{
    fn new(service: V) -> Self {
        Self {
            server: Server::new("", 0).with_service(service),
        }
    }

    fn send(&self, request: Request<Body>) -> anyhow::Result<Response> {
        self.server.fire::<std::io::BufWriter<Vec<u8>>>(request)
    }
}

pub struct TestCaseBuilder<V> {
    name: Option<String>,
    service: V,

    /// Url of a request.
    url: String,
    method: Method,

    body: Option<Body>,
    headers: Option<HashMap<String, String>>,
    result: Option<Vec<u8>>,
}

impl<V> TestCaseBuilder<V>
where
    V: Service<Request<Body>> + Send + Sync + 'static,
{
    pub fn new<T>(url: T, method: Method, service: V) -> Self
    where
        T: ToString,
        V: Service<Request<Body>> + Send + Sync + 'static,
    {
        Self {
            name: None,
            url: url.to_string(),
            method,
            service,
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

    pub fn header<K, L>(mut self, key: K, value: L) -> Self
    where
        K: ToString,
        L: ToString,
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

        let client = Client::new(self.service);

        let res: Response = client.send(req)?;
        let body_bytes: Vec<u8> = body_to_bytes(res.into_body())?.into();

        let res = self.result.unwrap_or_default().to_vec();
        assert_eq!(
            body_bytes,
            res,
            "test case {}, left: {}, right: {}",
            self.name.unwrap_or_default(),
            std::str::from_utf8(&body_bytes).unwrap(),
            std::str::from_utf8(&res).unwrap()
        );

        Ok(())
    }
}
