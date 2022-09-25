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

    pub fn send(&self, request: Request<Body>) -> anyhow::Result<impl Responder> {
        Ok(self
            .routes
            .get(request.method())
            .with_context(|| format!("not registered routes for {:?} method", request.method()))?
            .iter()
            .find(|route| route.should_fire_on_path(request.uri().to_string()))
            .context("no matching route")?
            .clone()
            .service
            .0
            .call(request))
    }
}
