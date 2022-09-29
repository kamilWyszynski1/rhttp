use core::handler::{HandlerTrait, HandlerTraitWithoutState};
use core::request::{ContentType, Host, Json, PathParam, Query, State};
use core::response::{Responder, Response};
use core::server::Route;
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

    TestCaseBuilder::new("/", "/", Method::GET, empty.into_service())
        .name("empty")
        .run()?;

    TestCaseBuilder::new("/str", "/str", Method::GET, str.into_service())
        .name("str")
        .result("hello")
        .run()?;

    TestCaseBuilder::new("/string", "/string", Method::GET, string.into_service())
        .name("string")
        .result("hello")
        .run()?;

    TestCaseBuilder::new("/result", "/result", Method::GET, result.into_service())
        .name("result")
        .result("ok")
        .run()?;

    TestCaseBuilder::new(
        "/content-type",
        "/content-type",
        Method::GET,
        content_type_handler.into_service(),
    )
    .name("content-type")
    .header(hyper::header::CONTENT_TYPE, "application/json")
    .result("application/json")
    .run()?;

    TestCaseBuilder::new("/host", "/host", Method::GET, host_handler.into_service())
        .name("host")
        .header(hyper::header::HOST, "localhost")
        .result("localhost")
        .run()?;

    TestCaseBuilder::new(
        "/param/<user>",
        "/param/test-user",
        Method::GET,
        param_handler.into_service(),
    )
    .name("param")
    .result("test-user")
    .run()?;

    TestCaseBuilder::new(
        "/body",
        "/body",
        Method::POST,
        body_handler_json.into_service(),
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
        "/body/<user>",
        "/body/username",
        Method::POST,
        handler.into_service(),
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
        "/body/<user>/<id>",
        "/body/username/100",
        Method::POST,
        handler.into_service(),
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
        "/query",
        "/query?val=value&name=john&age=123",
        Method::POST,
        handler.into_service(),
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
        "/query",
        "/query?val=value&name=john&age=123",
        Method::POST,
        handler.into_service_with_state(cfg),
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
        "/query",
        "/query?val=value&name=john&age=123",
        Method::POST,
        handler.into_service_with_state(cfg),
    )
    .name("test_state_with_extractors")
    .body(Body::from(body))
    .result(changed_body)
    .run()?;

    Ok(())
}
