#![allow(dead_code)]
use crate::{response::Responder, server::Route};
use anyhow::Context;
use hyper::{Body, Method, Request};
use std::collections::HashMap;

pub struct Client {
    routes: HashMap<Method, Vec<Route>>,
}

impl Client {
    pub fn new(routes: HashMap<Method, Vec<Route>>) -> Self {
        Self { routes }
    }

    pub fn send(&self, mut request: Request<Body>) -> anyhow::Result<impl Responder> {
        let route = self
            .routes
            .get(request.method())
            .with_context(|| format!("not registered routes for {:?} method", request.method()))?
            .iter()
            .find(|route| route.should_fire_on_path(request.uri().path()))
            .context("no matching route")?
            .clone();

        request
            .extensions_mut()
            .insert(route.metadata.param_segments);

        Ok(route.service.0.call(request))
    }
}
