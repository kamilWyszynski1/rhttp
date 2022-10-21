use anyhow::Ok;
use core::handler::HandlerTraitWithoutState;
use core::request::{ContentType, Host, Json, PathParam, Query, State};
use core::response::{Responder, Response};
use core::route::{Route, RouteGroup, Router};
use hyper::Body;
use hyper::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use tools::TestCaseBuilder;

mod tools;

#[test]
fn test_should_fire_on_path() {
    fn handler() {}

    let r = Route::new("/test", handler.into_service().into()).expect("valid route");

    assert!(r.should_fire_on_path("/test"));
    assert!(!r.should_fire_on_path("/test/test"));
    assert!(!r.should_fire_on_path("/"));

    let r = Route::new("/test/<param1>", handler.into_service().into()).expect("valid route");

    assert!(!r.should_fire_on_path("/test"));
    assert!(r.should_fire_on_path("/test/test"));
    assert!(!r.should_fire_on_path("/"));

    let r =
        Route::new("/test/<param1>/<param2>", handler.into_service().into()).expect("valid route");

    assert!(r.should_fire_on_path("/test/1/2"));
    assert!(!r.should_fire_on_path("/test/test"));
    assert!(!r.should_fire_on_path("/"));
}

#[derive(Serialize, Deserialize)]
struct OwnBody {
    val: String,
    val2: i32,
    val3: bool,
}

impl Responder for OwnBody {
    fn into_response(self) -> anyhow::Result<Response> {
        Ok(hyper::Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(serde_json::to_string(&self)?))?)
    }
}

#[test]
fn test_with_client() -> anyhow::Result<()> {
    fn empty() {}

    fn str() -> &'static str {
        "hello"
    }

    fn string() -> String {
        String::from("hello")
    }

    fn result() -> anyhow::Result<&'static str> {
        Ok("ok")
    }

    fn body_handler_json(Json(body): Json<OwnBody>) -> anyhow::Result<OwnBody> {
        Ok(body)
    }

    fn content_type_handler(ContentType(content_type): ContentType) -> String {
        content_type
    }

    fn host_handler(Host(host): Host) -> String {
        host
    }

    fn param_handler(PathParam(user): PathParam<String>) -> String {
        user
    }

    TestCaseBuilder::new("/", Method::GET, Router::default().get("/", empty))
        .name("empty")
        .run()?;

    TestCaseBuilder::new("/str", Method::GET, Router::default().get("/str", str))
        .name("str")
        .result("hello")
        .run()?;

    TestCaseBuilder::new(
        "/string",
        Method::GET,
        Router::default().get("/string", string),
    )
    .name("string")
    .result("hello")
    .run()?;

    TestCaseBuilder::new(
        "/result",
        Method::GET,
        Router::default().get("/result", result),
    )
    .name("result")
    .result("ok")
    .run()?;

    TestCaseBuilder::new(
        "/content-type",
        Method::GET,
        Router::default().get("/content-type", content_type_handler),
    )
    .name("content-type")
    .header(hyper::header::CONTENT_TYPE, "application/json")
    .result("application/json")
    .run()?;

    TestCaseBuilder::new(
        "/host",
        Method::GET,
        Router::default().get("/host", host_handler),
    )
    .name("host")
    .header(hyper::header::HOST, "localhost")
    .result("localhost")
    .run()?;

    TestCaseBuilder::new(
        "/param/test-user",
        Method::GET,
        Router::default().get("/param/<user>", param_handler),
    )
    .name("param")
    .result("test-user")
    .run()?;

    TestCaseBuilder::new(
        "/body",
        Method::POST,
        Router::default().post("/body", body_handler_json),
    )
    .name("body")
    .body(Body::from(
        r#"{"val":"string value","val2": 123,"val3":true}"#,
    ))
    .result(r#"{"val":"string value","val2":123,"val3":true}"#)
    .run()?;

    Ok(())
}

#[test]
fn test_with_client_2_param_handlers() -> anyhow::Result<()> {
    fn handler(PathParam(user): PathParam<String>, Json(mut body): Json<OwnBody>) -> OwnBody {
        body.val = user;
        body
    }

    let body = r#"{"val":"string value","val2":123,"val3":true}"#;
    let changed_body = r#"{"val":"username","val2":123,"val3":true}"#;

    TestCaseBuilder::new(
        "/body/username",
        Method::POST,
        Router::default().post("/body/<user>", handler),
    )
    .name("handler with path param and body")
    .body(Body::from(body))
    .result(changed_body)
    .run()?;

    Ok(())
}

#[test]
fn test_with_client_3_param_handlers() -> anyhow::Result<()> {
    fn handler(
        PathParam(user): PathParam<String>,
        PathParam(id): PathParam<i32>,
        Json(mut body): Json<OwnBody>,
    ) -> OwnBody {
        body.val = user;
        body.val2 = id;
        body
    }

    let body = r#"{"val":"string value","val2":123,"val3":true}"#;
    let changed_body = r#"{"val":"username","val2":100,"val3":true}"#;

    TestCaseBuilder::new(
        "/body/username/100",
        Method::POST,
        Router::default().post("/body/<user>/<id>", handler),
    )
    .name("handler with path param and body")
    .body(Body::from(body))
    .result(changed_body)
    .run()?;

    Ok(())
}

#[test]
fn handler_query() -> anyhow::Result<()> {
    #[derive(Serialize, Deserialize)]
    struct QueryParams {
        val: String,
        name: String,
        age: i32,
    }

    fn handler(Query(params): Query<QueryParams>) -> String {
        serde_json::to_string(&params).unwrap()
    }

    let body = r#"{"val":"value","name":"john","age":123}"#;
    TestCaseBuilder::new(
        "/query?val=value&name=john&age=123",
        Method::POST,
        Router::default().post("/query", handler),
    )
    .name("handler with path param and body")
    .result(body)
    .run()?;

    Ok(())
}

#[test]
fn test_state() -> anyhow::Result<()> {
    #[derive(Default, Clone, Serialize)]
    struct Config {
        port: i32,
        debug: bool,
        db_host: String,
        db_password: String,
    }

    fn handler(state: State<Config>) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&state.0)?)
    }

    let cfg = Config {
        port: 8080,
        debug: true,
        db_host: "localhost".into(),
        db_password: "1qazxsw2".into(),
    };
    let body = r#"{"port":8080,"debug":true,"db_host":"localhost","db_password":"1qazxsw2"}"#;
    TestCaseBuilder::new(
        "/query?val=value&name=john&age=123",
        Method::POST,
        Router::with_state(cfg).post("/query", handler),
    )
    .name("handler with path param and body")
    .result(body)
    .run()?;

    Ok(())
}

#[test]
fn test_state_with_extractors() -> anyhow::Result<()> {
    #[derive(Default, Clone, Serialize)]
    struct Config {
        port: i32,
        debug: bool,
        db_host: String,
        db_password: String,
    }

    #[derive(Serialize, Deserialize)]
    struct QueryParams {
        val: String,
        name: String,
        age: i32,
    }

    fn handler(
        state: State<Config>,
        Query(params): Query<QueryParams>,
        Json(mut body): Json<OwnBody>,
    ) -> anyhow::Result<String> {
        body.val = state.0.db_host;
        body.val2 = params.age;
        Ok(serde_json::to_string(&body)?)
    }

    let cfg = Config {
        port: 8080,
        debug: true,
        db_host: "localhost".into(),
        db_password: "1qazxsw2".into(),
    };

    let body = r#"{"val":"string value","val2":123,"val3":true}"#;
    let changed_body = r#"{"val":"localhost","val2":123,"val3":true}"#;
    TestCaseBuilder::new(
        "/query?val=value&name=john&age=123",
        Method::POST,
        Router::with_state(cfg).post("/query", handler),
    )
    .name("test_state_with_extractors")
    .body(Body::from(body))
    .result(changed_body)
    .run()?;

    Ok(())
}

#[test]
fn test_route_group() -> anyhow::Result<()> {
    let v1 = RouteGroup::new("/v1")
        .get("/user", (|| "v1").into_service())
        .get("/user2", (|| "v3").into_service());
    let v2 = RouteGroup::new("/v2")
        .get("/user", (|| "v2").into_service())
        .get("/user2", (|| "v4").into_service());

    let app = Router::default().groups(vec![v1, v2]);

    TestCaseBuilder::new("/v1/user", Method::GET, app.clone())
        .name("test_route_group")
        .result("v1")
        .run()?;
    TestCaseBuilder::new("/v1/user2", Method::GET, app.clone())
        .name("test_route_group")
        .result("v3")
        .run()?;

    TestCaseBuilder::new("/v2/user", Method::GET, app.clone())
        .name("test_route_group")
        .result("v2")
        .run()?;
    TestCaseBuilder::new("/v2/user2", Method::GET, app)
        .name("test_route_group")
        .result("v4")
        .run()?;
    Ok(())
}
