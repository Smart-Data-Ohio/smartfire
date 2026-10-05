//! End-to-end behavior of the HTTP layer through a real Axum router (`tower::ServiceExt::oneshot`).

use std::net::SocketAddr;

use axum::Router;
use campfire_kit::exceptions::ErrorPages;
use axum::body::Body as AxumBody;
use axum::extract::ConnectInfo;
use axum::http::{Request as HttpRequest, header};
use campfire_kit::format::{HTML, JSON, TURBO_STREAM};
use campfire_kit::{
    Cookie, Ctx, ExpiresIn, Freshness, Kit, KitConfig, Redirect, Result, SendOptions, StatusCode, action, front, halt, testing,
};
use serde_json::json;
use tower::ServiceExt;

#[derive(Debug)]
struct AppState {
    name: &'static str,
}

#[derive(Clone)]
struct CurrentUser(String);

async fn show(c: &mut Ctx) -> Result {
    let id = c.param_str("id").unwrap_or("").to_string();
    let name = c.state::<AppState>().name;
    Ok(c.html(format!("<p>{name} room {id}</p>")))
}

/// A page with `csrf_meta_tags` and a form (`form_with url: "/form"`), each with its token.
async fn form(c: &mut Ctx) -> Result {
    let tokens = c.authenticity_tokens();
    let (meta, form) = (tokens.global(), tokens.for_form("/form", "post"));
    Ok(c.html(format!(
        "<meta name=\"csrf-token\" content=\"{meta}\"><form method=\"post\" action=\"/form\">\
         <input type=\"hidden\" name=\"authenticity_token\" value=\"{form}\"></form>"
    )))
}

/// `csp_meta_tag`: the request's nonce, in the body.
async fn nonce(c: &mut Ctx) -> Result {
    if c.param_str("write").is_some() {
        c.session().insert("seen", true);
    }
    let nonce = c.content_security_policy_nonce().unwrap_or_default();
    Ok(c.html(format!("<meta name=\"csp-nonce\" content=\"{nonce}\">")))
}

/// A response that sets its own policy (`content_security_policy false` or a per-action one).
async fn own_policy(c: &mut Ctx) -> Result {
    c.set_header("content-security-policy", "default-src 'none'");
    Ok(c.html("own"))
}

async fn create(c: &mut Ctx) -> Result {
    c.verify_authenticity_token()?;
    let body = json!({
        "method": c.request.method.as_str(),
        "original_method": c.request.original_method.as_str(),
        "params": c.params.to_json(),
    });
    c.json(StatusCode::OK, &body)
}

async fn echo(c: &mut Ctx) -> Result {
    let body = json!({
        "method": c.request.method.as_str(),
        "original_method": c.request.original_method.as_str(),
        "params": c.params.to_json(),
        "raw": String::from_utf8_lossy(c.request.raw_post()),
        "remote_ip": c.request.remote_ip()?,
    });
    c.json(StatusCode::OK, &body)
}

async fn upload(c: &mut Ctx) -> Result {
    let user = c.params.require("user")?.clone();
    let file = user.get("avatar").and_then(|p| p.as_file()).cloned();
    let body = json!({
        "method": c.request.method.as_str(),
        "name": user.get("name").and_then(|p| p.as_str()),
        "filename": file.as_ref().map(|f| f.original_filename.clone()),
        "content": file.map(|f| String::from_utf8(f.read().unwrap()).unwrap()),
    });
    c.json(StatusCode::OK, &body)
}

async fn session_set(c: &mut Ctx) -> Result {
    let value = c.param_str("value").unwrap_or("").to_string();
    c.session().insert("return_to_after_authenticating", value);
    Ok(c.head(StatusCode::OK))
}

async fn session_get(c: &mut Ctx) -> Result {
    let session = c.session();
    let body = json!({ "value": session.get("return_to_after_authenticating"), "id": session.id() });
    c.json(StatusCode::OK, &body)
}

async fn session_reset(c: &mut Ctx) -> Result {
    c.reset_session();
    Ok(c.head(StatusCode::OK))
}

async fn noop(c: &mut Ctx) -> Result {
    Ok(c.head(StatusCode::NO_CONTENT))
}

async fn notice(c: &mut Ctx) -> Result {
    c.redirect_to_with("/flash", Redirect { notice: Some("✓".into()), ..Redirect::default() })
}

async fn show_flash(c: &mut Ctx) -> Result {
    let notice = c.flash().notice().map(str::to_string);
    c.json(StatusCode::OK, &json!({ "notice": notice }))
}

async fn sign_in(c: &mut Ctx) -> Result {
    c.cookies.set_signed("session_token", Cookie::new("tok123").permanent().httponly())?;
    c.cookies.set("last_room", Cookie::new("7").permanent());
    Ok(c.head(StatusCode::OK))
}

/// `sign_in` from an `ActionController::Live` controller.
async fn live_sign_in(c: &mut Ctx) -> Result {
    c.use_live_response();
    sign_in(c).await
}

async fn whoami(c: &mut Ctx) -> Result {
    let token = c.cookies.signed("session_token");
    c.json(StatusCode::OK, &json!({ "token": token, "last_room": c.cookies.get("last_room") }))
}

async fn sign_out(c: &mut Ctx) -> Result {
    c.cookies.delete("session_token");
    Ok(c.head(StatusCode::OK))
}

fn require_user(c: &mut Ctx) -> Result<()> {
    match c.request.header("x-user") {
        Some(user) => {
            c.set_current(CurrentUser(user.to_string()));
            Ok(())
        }
        None => halt(c.redirect_to("/session/new")?),
    }
}

fn ensure_admin(c: &mut Ctx) -> Result<()> {
    if c.current::<CurrentUser>().is_some_and(|u| u.0 == "admin") { Ok(()) } else { halt(c.head(StatusCode::FORBIDDEN)) }
}

async fn admin(c: &mut Ctx) -> Result {
    c.set_header("x-version", "42");
    require_user(c)?;
    ensure_admin(c)?;
    let user = c.current::<CurrentUser>().unwrap().0.clone();
    Ok(c.html(format!("hi {user}")))
}

async fn messages(c: &mut Ctx) -> Result {
    let format = c.respond_to(&[&HTML, &TURBO_STREAM, &JSON])?;
    if format == &TURBO_STREAM {
        Ok(c.turbo_stream("<turbo-stream action=\"append\"></turbo-stream>"))
    } else if format == &JSON {
        c.json(StatusCode::OK, &json!([]))
    } else {
        Ok(c.render(StatusCode::OK, &HTML, "<p>messages</p>"))
    }
}

async fn autocomplete(c: &mut Ctx) -> Result {
    let format = c.respond_to(&[&HTML, &JSON])?;
    Ok(c.render(StatusCode::OK, format, "x"))
}

async fn redirects(c: &mut Ctx) -> Result {
    match c.param_str("to").unwrap_or("") {
        "relative" => c.redirect_to("rooms/1"),
        "other" => c.redirect_to("https://evil.example/"),
        "other_allowed" => {
            c.redirect_to_with("https://docs.example/", Redirect { allow_other_host: true, ..Redirect::default() })
        }
        "see_other" => c.redirect_to_with("/rooms", Redirect { status: Some(StatusCode::SEE_OTHER), ..Redirect::default() }),
        "back" => c.redirect_back_or_to("/fallback"),
        _ => c.redirect_to("/rooms/1?x=1"),
    }
}

async fn created(c: &mut Ctx) -> Result {
    c.head_with_location(StatusCode::CREATED, "/rooms/1/messages/2")
}

async fn file(c: &mut Ctx) -> Result {
    let path = c.param_str("path").unwrap().to_string();
    let mut options = SendOptions::inline("image/png");
    options.ranges = c.param_str("ranges").is_some();
    c.send_file(path, options)
}

async fn logo(c: &mut Ctx) -> Result {
    if c.stale(Freshness::etag("accounts/1-20240601")) {
        c.expires_in(300, ExpiresIn { public: true, stale_while_revalidate: Some(604800), ..ExpiresIn::default() });
        return Ok(c.send_data("PNG", SendOptions::inline("image/png")));
    }
    Ok(c.head(StatusCode::NOT_MODIFIED))
}

async fn index_fresh(c: &mut Ctx) -> Result {
    if let Some(not_modified) = c.fresh_when(Freshness::etag("messages/1-2")) {
        return Ok(not_modified);
    }
    Ok(c.html("<p>page</p>"))
}

fn kit_with(config: KitConfig) -> Kit {
    Kit::new(config, testing::crypto(), testing::frozen_clock(), AppState { name: "Campfire" })
}

fn app_with(config: KitConfig) -> Router {
    let router = Router::new()
        .route("/rooms/{id}", campfire_kit::get(show))
        .route("/form", campfire_kit::get(form).post(action(create)))
        .route("/echo/{id}", campfire_kit::get(echo).post(action(echo)).patch(action(echo)).delete(action(echo)))
        .route("/upload", campfire_kit::patch(upload).post(action(upload)))
        .route("/session", campfire_kit::get(session_get).post(action(session_set)).delete(action(session_reset)))
        .route("/noop", campfire_kit::get(noop))
        .route("/notice", campfire_kit::get(notice))
        .route("/flash", campfire_kit::get(show_flash))
        .route("/sign_in", campfire_kit::get(sign_in))
        .route("/live_sign_in", campfire_kit::get(live_sign_in))
        .route("/whoami", campfire_kit::get(whoami))
        .route("/sign_out", campfire_kit::get(sign_out))
        .route("/admin", campfire_kit::get(admin))
        .route("/messages", campfire_kit::get(messages).post(action(messages)))
        .route("/messages.{format}", campfire_kit::get(messages))
        .route("/autocomplete", campfire_kit::get(autocomplete))
        .route("/redirect", campfire_kit::get(redirects))
        .route("/created", campfire_kit::get(created))
        .route("/file", campfire_kit::get(file))
        .route("/logo", campfire_kit::get(logo))
        .route("/fresh", campfire_kit::get(index_fresh))
        .route("/nonce", campfire_kit::get(nonce))
        .route("/own_policy", campfire_kit::get(own_policy));
    campfire_kit::app(router, kit_with(config))
}

fn app() -> Router {
    let error_pages = ErrorPages::new([(404, "<h1>Not found</h1>".into()), (422, "<h1>Unprocessable</h1>".into())]);
    app_with(KitConfig { error_pages, ..KitConfig::default() })
}

struct Reply {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: Vec<u8>,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    fn cookies(&self) -> Vec<String> {
        self.headers.get_all(header::SET_COOKIE).iter().map(|v| v.to_str().unwrap().to_string()).collect()
    }

    /// A `Cookie` request header carrying every cookie this response set.
    fn cookie_jar(&self) -> String {
        self.cookies().iter().map(|c| c.split(';').next().unwrap().to_string()).collect::<Vec<_>>().join("; ")
    }

    fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap()
    }

    fn text(&self) -> String {
        String::from_utf8(self.body.clone()).unwrap()
    }
}

async fn send(app: &Router, request: HttpRequest<AxumBody>) -> Reply {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec();
    Reply { status, headers, body }
}

fn get(uri: &str) -> axum::http::request::Builder {
    HttpRequest::get(uri).header(header::HOST, "chat.example.com")
}

fn post(uri: &str) -> axum::http::request::Builder {
    HttpRequest::post(uri).header(header::HOST, "chat.example.com")
}

fn form_post(uri: &str, body: &str) -> HttpRequest<AxumBody> {
    post(uri).header(header::CONTENT_TYPE, "application/x-www-form-urlencoded").body(AxumBody::from(body.to_string())).unwrap()
}

#[tokio::test]
async fn renders_html_with_rails_headers() {
    let reply = send(&app(), get("/rooms/5").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.text(), "<p>Campfire room 5</p>");
    assert_eq!(reply.header("content-type"), Some("text/html; charset=utf-8"));
    assert_eq!(reply.header("x-frame-options"), Some("SAMEORIGIN"));
    assert_eq!(reply.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(reply.header("referrer-policy"), Some("strict-origin-when-cross-origin"));
    assert_eq!(reply.header("cache-control"), Some("max-age=0, private, must-revalidate"));
    assert!(reply.header("etag").unwrap().starts_with("W/\""));
    assert_eq!(reply.header("x-request-id").map(str::len), Some(36));
    assert!(reply.cookies().is_empty(), "no session touched, no cookie");
}

#[tokio::test]
async fn conditional_get_on_body_etag() {
    let app = app();
    let first = send(&app, get("/rooms/5").body(AxumBody::empty()).unwrap()).await;
    let etag = first.header("etag").unwrap().to_string();
    let second = send(&app, get("/rooms/5").header(header::IF_NONE_MATCH, &etag).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(second.status, StatusCode::NOT_MODIFIED);
    assert!(second.body.is_empty());
    assert_eq!(second.header("content-type"), None);

    // Pinned Rack::ConditionalGet compares one exact string; lists and `*`
    // differ from Rails' controller-level fresh_when checks.
    for if_none_match in [format!("\"other\", {etag}"), "*".to_string()] {
        let listed = send(&app, get("/rooms/5").header(header::IF_NONE_MATCH, &if_none_match).body(AxumBody::empty()).unwrap()).await;
        assert_eq!(listed.status, StatusCode::OK, "{if_none_match}");
        assert_eq!(listed.text(), first.text());
    }
    let other = send(&app, get("/rooms/5").header(header::IF_NONE_MATCH, "\"other\"").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(other.status, StatusCode::OK);
}

#[tokio::test]
async fn head_requests_drop_the_body() {
    let reply = send(&app(), get("/rooms/5").method("HEAD").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body.is_empty());
    assert_eq!(reply.header("content-length"), Some("22"));
}

#[tokio::test]
async fn unknown_routes_and_methods_are_rails_404s() {
    let app = app();
    let reply = send(&app, get("/nope").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.text(), "<h1>Not found</h1>");
    let reply = send(&app, HttpRequest::put("/rooms/1").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let reply = send(&app, get("/nope.json").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(reply.text(), r#"{"status":404,"error":"Not Found"}"#);
}

#[tokio::test]
async fn params_merge_query_over_body_and_path_over_both() {
    let reply = send(&app(), form_post("/echo/9?b=query&id=q", "a[b][]=1&a[b][]=2&b=body&id=body")).await;
    let json = reply.json();
    assert_eq!(json["params"], json!({"a": {"b": ["1", "2"]}, "b": "query", "id": "9"}));
    assert_eq!(json["raw"], "a[b][]=1&a[b][]=2&b=body&id=body");
}

#[tokio::test]
async fn malformed_params_are_400() {
    let reply = send(&app(), get("/echo/1?a=1&a[b]=2").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let reply = send(&app(), form_post("/echo/1", "a=%")).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let request = post("/echo/1").header(header::CONTENT_TYPE, "application/json").body(AxumBody::from("{nope")).unwrap();
    assert_eq!(send(&app(), request).await.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ordinary_form_limit_remains_four_mib_including_trailing_nul() {
    // The disk parser's Rack-compatible NUL handling must not change the ordinary
    // adapter or POST method-override middleware's existing size limit on main.
    for method in [axum::http::Method::POST, axum::http::Method::PATCH] {
        let bytes = vec![b'x'; campfire_kit::params::FORM_BYTESIZE_LIMIT];
        let request = HttpRequest::builder().method(method.clone()).uri("/echo/1")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(AxumBody::from(bytes.clone())).unwrap();
        assert_eq!(send(&app(), request).await.status, StatusCode::OK, "{method}");
        let mut bytes = bytes;
        bytes.push(0);
        let request = HttpRequest::builder().method(method.clone()).uri("/echo/1")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(AxumBody::from(bytes)).unwrap();
        assert_eq!(send(&app(), request).await.status, StatusCode::BAD_REQUEST, "{method}");
    }
}

#[tokio::test]
async fn ordinary_multipart_routes_retain_main_count_and_blank_filename_limits() {
    for (filename, count) in [(Some("x.bin"), 128), (None, 4096), (Some(""), 129)] {
        let file = filename.map(|name| format!("; filename=\"{name}\"")).unwrap_or_default();
        let part = format!("--ordinary-boundary\r\nContent-Disposition: form-data; name=\"a[]\"{file}\r\n\r\nx\r\n");
        let body = part.repeat(count) + "--ordinary-boundary--\r\n";
        let request = post("/echo/1").header(header::CONTENT_TYPE, "multipart/form-data; boundary=ordinary-boundary")
            .body(AxumBody::from(body)).unwrap();
        assert_eq!(send(&app(), request).await.status, StatusCode::OK, "{filename:?}, {count}");
    }
}

#[tokio::test]
async fn method_override_from_form_param_and_header() {
    let app = app();
    let reply = send(&app, form_post("/echo/1", "_method=patch&x=1")).await;
    assert_eq!(reply.json()["method"], "PATCH");
    assert_eq!(reply.json()["original_method"], "POST");

    let reply = send(&app, form_post("/echo/1", "_method=delete")).await;
    assert_eq!(reply.json()["method"], "DELETE");

    let request = post("/echo/1").header("x-http-method-override", "patch").body(AxumBody::from("")).unwrap();
    assert_eq!(send(&app, request).await.json()["method"], "PATCH");

    // JSON bodies aren't forms: `_method` there doesn't count.
    let request = post("/echo/1")
        .header(header::CONTENT_TYPE, "application/json")
        .body(AxumBody::from(r#"{"_method":"patch","a":[1,null,2]}"#))
        .unwrap();
    let reply = send(&app, request).await;
    assert_eq!(reply.json()["method"], "POST");
    assert_eq!(reply.json()["params"]["a"], json!([1, 2]));

    let reply = send(&app, form_post("/echo/1", "_method=bogus")).await;
    assert_eq!(reply.json()["method"], "POST");
}

#[tokio::test]
async fn pr196_r4_json_parser_observes_effective_method_through_real_router() {
    fn parser(method: &axum::http::Method, _: &str, _: &[u8]) -> Option<std::result::Result<campfire_kit::ParamMap, campfire_kit::params::ParamError>> {
        let mut params = campfire_kit::ParamMap::new();
        params.insert("parser_method", campfire_kit::Param::Str(method.to_string()));
        Some(Ok(params))
    }
    async fn selected(c: &mut Ctx) -> Result {
        let parsed = c.param_str("parser_method").unwrap().to_owned();
        c.set_header("x-parser-method", &parsed);
        let original = c.request.original_method.to_string();
        c.set_header("x-original-method", &original);
        Ok(c.head(StatusCode::OK))
    }
    let router = Router::new().route("/parser", axum::routing::any(action(selected).json_body_parser(parser)));
    let app = campfire_kit::app(router, kit_with(KitConfig::default()));
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../vectors/bot-ui-render-replay.json")).unwrap();
    let mut failures = Vec::new();
    for case in oracle["methods"].as_array().unwrap() {
        let request = post("/parser")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-http-method-override", case["override"].as_str().unwrap())
            .body(AxumBody::from("{\"value\":true}" )).unwrap();
        let reply = send(&app, request).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.header("x-original-method"), Some("POST"));
        if reply.header("x-parser-method") != case["effective_method"].as_str() {
            failures.push((case["override"].clone(), reply.header("x-parser-method").map(str::to_owned)));
        }
    }
    println!("R4 adapter effective methods: 7 compared; {} mismatches", failures.len());
    assert!(failures.is_empty(), "{failures:?}");
}

#[tokio::test]
async fn multipart_uploads_with_method_override() {
    let boundary = "----campfire";
    let body = format!(
        "--{b}\r\nContent-Disposition: form-data; name=\"_method\"\r\n\r\npatch\r\n\
         --{b}\r\nContent-Disposition: form-data; name=\"user[name]\"\r\n\r\nJo\r\n\
         --{b}\r\nContent-Disposition: form-data; name=\"user[avatar]\"; filename=\"me.png\"\r\nContent-Type: image/png\r\n\r\nPNGDATA\r\n\
         --{b}--\r\n",
        b = boundary
    );
    let request = post("/upload")
        .header(header::CONTENT_TYPE, format!("multipart/form-data; boundary={boundary}"))
        .body(AxumBody::from(body))
        .unwrap();
    let reply = send(&app(), request).await;
    assert_eq!(reply.json(), json!({"method": "PATCH", "name": "Jo", "filename": "me.png", "content": "PNGDATA"}));
}

#[tokio::test]
async fn missing_required_param_is_400() {
    let reply = send(&app(), form_post("/upload", "other=1")).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
}

/// The test app as Campfire runs it in production: behind TLS (`assume_ssl`) with `force_ssl`.
fn ssl_app() -> Router {
    let error_pages = ErrorPages::new([(422, "<h1>Unprocessable</h1>".into())]);
    let mut config = KitConfig { error_pages, force_ssl: true, ..KitConfig::default() };
    config.proxy.assume_ssl = true;
    app_with(config)
}

/// The meta and form tokens of a page from `/form`, and the session cookie that came with it.
async fn form_page(app: &Router, cookie: Option<&str>) -> (String, String, String) {
    let mut request = get("/form");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let page = send(app, request.body(AxumBody::empty()).unwrap()).await;
    let text = page.text();
    let meta = text.split("content=\"").nth(1).unwrap().split('"').next().unwrap().to_string();
    let form = text.split("value=\"").nth(1).unwrap().split('"').next().unwrap().to_string();
    let jar = if page.cookies().is_empty() { cookie.unwrap_or("").to_string() } else { page.cookie_jar() };
    (jar, meta, form)
}

fn post_form(cookie: &str, header_token: Option<&str>) -> axum::http::request::Builder {
    let mut request = post("/form").header(header::COOKIE, cookie).header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    if let Some(token) = header_token {
        request = request.header("x-csrf-token", token);
    }
    request
}

fn form_body(token: Option<&str>) -> AxumBody {
    match token {
        Some(token) => AxumBody::from(format!("x=1&authenticity_token={}", campfire_kit::cookies::escape(token))),
        None => AxumBody::from("x=1"),
    }
}

#[tokio::test]
async fn forgery_protection_accepts_the_pages_tokens() {
    let app = ssl_app();
    let (cookie, meta, form) = form_page(&app, None).await;
    assert!(cookie.starts_with("_campfire_session="), "rendering tokens starts a session");
    for (param, header) in [(Some(form.as_str()), None), (Some(meta.as_str()), None), (None, Some(meta.as_str())), (Some("bad"), Some(meta.as_str())), (Some(form.as_str()), Some("bad"))] {
        let ok = send(&app, post_form(&cookie, header).body(form_body(param)).unwrap()).await;
        assert_eq!(ok.status, StatusCode::OK, "{param:?} {header:?}");
        assert_eq!(ok.json()["params"]["x"], "1");
    }
    // The session keeps its token: a second page's tokens are masked differently but work too.
    let (same_cookie, meta2, form2) = form_page(&app, Some(&cookie)).await;
    assert_eq!(same_cookie, cookie, "an unchanged session isn't rewritten");
    assert_ne!(form2, form);
    assert_ne!(meta2, meta);
    let ok = send(&app, post_form(&cookie, None).body(form_body(Some(&form2))).unwrap()).await;
    assert_eq!(ok.status, StatusCode::OK);
}

#[tokio::test]
async fn forgery_protection_rejects_missing_forged_and_foreign_tokens() {
    let app = ssl_app();
    let (cookie, _, form) = form_page(&app, None).await;
    let (other_cookie, other_meta, other_form) = form_page(&app, None).await;
    let tampered = format!("{}{}", if form.starts_with('A') { "B" } else { "A" }, &form[1..]);
    let forged = "z1FAXTN4P9BfpfzcU7YdmtlVpR0ZwggU1QBiTBlO3s7pdPcS8wG4zZ0KWmz2M-rf5IhDc4JyEjleeWd3AWoXxw";
    let cases: [(&str, &str, Option<&str>, Option<&str>); 7] = [
        ("no token", &cookie, None, None),
        ("forged", &cookie, Some(forged), None),
        ("tampered", &cookie, Some(&tampered), None),
        ("another session's form token", &cookie, Some(&other_form), None),
        ("another session's meta token", &cookie, None, Some(&other_meta)),
        ("the other session's cookie", &other_cookie, Some(&form), None),
        ("no session", "", Some(&form), None),
    ];
    for (label, cookie, param, header) in cases {
        let forged = send(&app, post_form(cookie, header).body(form_body(param)).unwrap()).await;
        assert_eq!(forged.status, StatusCode::UNPROCESSABLE_ENTITY, "{label}");
        assert_eq!(forged.text(), "<h1>Unprocessable</h1>");
        assert!(forged.cookies().is_empty(), "{label}: errors don't commit cookies");
    }
}

#[tokio::test]
async fn sec_fetch_site_no_longer_stands_in_for_a_token() {
    for app in [ssl_app(), app()] {
        let same_origin = post("/form").header("sec-fetch-site", "same-origin").body(AxumBody::from("x=1")).unwrap();
        assert_eq!(send(&app, same_origin).await.status, StatusCode::UNPROCESSABLE_ENTITY);
        let (cookie, _, form) = form_page(&app, None).await;
        let cross_site = post_form(&cookie, None).header("sec-fetch-site", "cross-site").body(form_body(Some(&form))).unwrap();
        assert_eq!(send(&app, cross_site).await.status, StatusCode::OK, "the token decides, as in Rails");
    }
}

#[tokio::test]
async fn pages_without_tokens_start_no_session() {
    let page = send(&app(), get("/rooms/5").body(AxumBody::empty()).unwrap()).await;
    assert!(page.cookies().is_empty(), "a page that renders no token doesn't start a session");
}

#[tokio::test]
async fn forgery_protection_checks_the_origin() {
    let app = ssl_app();
    let (cookie, _, form) = form_page(&app, None).await;
    let with_origin = |origin: &str| post_form(&cookie, None).header(header::ORIGIN, origin).body(form_body(Some(&form))).unwrap();
    assert_eq!(send(&app, with_origin("https://chat.example.com")).await.status, StatusCode::OK);
    assert_eq!(send(&app, with_origin("https://evil.example")).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(send(&app, with_origin("http://chat.example.com")).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(send(&app, with_origin("null")).await.status, StatusCode::UNPROCESSABLE_ENTITY);
}

fn csp_app() -> Router {
    use campfire_kit::csp::{ContentSecurityPolicy, Source};
    use std::sync::Arc;
    let policy = ContentSecurityPolicy::new()
        .directive("default-src", ["'self'".into()])
        .directive("script-src", ["'self'".into()])
        .directive("connect-src", ["'self'".into(), Source::Dynamic(Arc::new(|| vec!["wss://lk.test".into()]))])
        .directive("report-uri", ["/csp_reports".into()])
        .nonce(Arc::new(|id: Option<&str>| format!("n-{}", id.unwrap_or("random"))), &["script-src"]);
    let error_pages = ErrorPages::new([(404, "<h1>Not found</h1>".into())]);
    app_with(KitConfig { error_pages, content_security_policy: Some(Arc::new(policy)), ..KitConfig::default() })
}

#[tokio::test]
async fn content_security_policy_goes_on_controller_responses() {
    let app = csp_app();
    let page = send(&app, get("/rooms/5").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(
        page.header("content-security-policy"),
        Some("default-src 'self'; script-src 'self' 'nonce-n-random'; connect-src 'self' wss://lk.test; report-uri /csp_reports")
    );
    let redirect = send(&app, get("/redirect").body(AxumBody::empty()).unwrap()).await;
    assert!(redirect.header("content-security-policy").is_some(), "redirects carry it too");

    let etag = send(&app, get("/fresh").body(AxumBody::empty()).unwrap()).await.header("etag").unwrap().to_string();
    let not_modified = send(&app, get("/fresh").header(header::IF_NONE_MATCH, etag).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(not_modified.status, StatusCode::NOT_MODIFIED);
    assert_eq!(not_modified.header("content-security-policy"), None, "not on a 304");

    let own = send(&app, get("/own_policy").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(own.header("content-security-policy"), Some("default-src 'none'"), "an action's own policy stays");

    let missing = send(&app, get("/nowhere").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(missing.header("content-security-policy"), None, "not on error pages");

    let without = send(&app_with(KitConfig::default()), get("/rooms/5").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(without.header("content-security-policy"), None);
}

#[tokio::test]
async fn csp_nonce_follows_the_session_id_rails_reports() {
    let app = csp_app();
    let nonce_of = |reply: &Reply| reply.text().split("content=\"").nth(1).unwrap().split('"').next().unwrap().to_string();

    // No session cookie and nothing written: no session id yet, so a random nonce.
    let first = send(&app, get("/nonce").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(nonce_of(&first), "n-random");
    assert!(first.header("content-security-policy").unwrap().contains("'nonce-n-random'"), "the header and the page agree");

    // A request that writes the session has its new id.
    let written = send(&app, get("/nonce?write=1").body(AxumBody::empty()).unwrap()).await;
    let cookie = written.cookie_jar();
    let id = send(&app, get("/session").header(header::COOKIE, &cookie).body(AxumBody::empty()).unwrap()).await.json()["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(nonce_of(&written), format!("n-{id}"));

    // Later requests carry the cookie, and the nonce stays the session's.
    let later = send(&app, get("/nonce").header(header::COOKIE, &cookie).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(nonce_of(&later), format!("n-{id}"));
    assert!(later.header("content-security-policy").unwrap().contains(&format!("'nonce-n-{id}'")));
}

#[tokio::test]
async fn session_cookie_is_written_only_when_the_session_changes() {
    let app = app();
    let untouched = send(&app, get("/noop").body(AxumBody::empty()).unwrap()).await;
    assert!(untouched.cookies().is_empty());

    let set = send(&app, form_post("/session", "value=%2Frooms%2F1")).await;
    assert_eq!(set.status, StatusCode::OK);
    let cookie = set.cookie_jar();
    assert!(cookie.starts_with("_campfire_session="));

    // Reading it, or not touching it, sends no cookie back: the one the browser has stays.
    let read = send(&app, get("/session").header(header::COOKIE, &cookie).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(read.json()["value"], "/rooms/1");
    assert!(read.cookies().is_empty(), "{:?}", read.cookies());
    let id = read.json()["id"].as_str().unwrap().to_string();
    assert_eq!(id.len(), 32);
    let noop = send(&app, get("/noop").header(header::COOKIE, &cookie).body(AxumBody::empty()).unwrap()).await;
    assert!(noop.cookies().is_empty());
    let same = form_post("/session", "value=%2Frooms%2F1");
    let (mut parts, body) = same.into_parts();
    parts.headers.insert(header::COOKIE, cookie.parse().unwrap());
    let same = send(&app, HttpRequest::from_parts(parts, body)).await;
    assert!(same.cookies().is_empty(), "writing the value it already holds changes nothing");
    let again = send(&app, get("/session").header(header::COOKIE, &cookie).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(again.json()["id"], id.as_str());

    // Resetting leaves nothing to keep, so the cookie goes.
    let reset = send(
        &app,
        HttpRequest::delete("/session").header(header::COOKIE, &cookie).body(AxumBody::empty()).unwrap(),
    )
    .await;
    assert!(reset.cookies().iter().any(|c| c.starts_with("_campfire_session=;")), "{:?}", reset.cookies());
    let after = send(&app, get("/session").header(header::COOKIE, reset.cookie_jar()).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(after.json()["value"], serde_json::Value::Null);
    assert_ne!(after.json()["id"], id.as_str());

    // A tampered cookie reads as an empty session.
    let bogus = send(&app, get("/session").header(header::COOKIE, "_campfire_session=garbage").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(bogus.json()["value"], serde_json::Value::Null);
}

#[tokio::test]
async fn flash_survives_exactly_one_redirect() {
    let app = app();
    let redirect = send(&app, get("/notice").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(redirect.status, StatusCode::FOUND);
    assert_eq!(redirect.header("location"), Some("http://chat.example.com/flash"));
    let cookie = redirect.cookie_jar();

    let shown = send(&app, get("/flash").header(header::COOKIE, &cookie).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(shown.json()["notice"], "✓");
    // Shown, the flash is swept, and with it the only thing the session held.
    assert!(shown.cookies().iter().any(|c| c.starts_with("_campfire_session=;")), "{:?}", shown.cookies());
    let gone = send(&app, get("/flash").header(header::COOKIE, shown.cookie_jar()).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(gone.json()["notice"], serde_json::Value::Null);
}

#[tokio::test]
async fn signed_permanent_cookies_and_deletion() {
    let app = app();
    let signed_in = send(&app, get("/sign_in").body(AxumBody::empty()).unwrap()).await;
    let cookies = signed_in.cookies();
    assert!(cookies.iter().any(|c| c.starts_with("session_token=")
        && c.ends_with("; path=/; expires=Wed, 01 Jun 2044 12:00:00 GMT; httponly; samesite=lax")));
    assert!(cookies.contains(&"last_room=7; path=/; expires=Wed, 01 Jun 2044 12:00:00 GMT; samesite=lax".to_string()));

    // Rails' Live responses send the action's cookies twice; this sends them once.
    let live = send(&app, get("/live_sign_in").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(live.cookies().len(), 2, "{:?}", live.cookies());

    let jar = signed_in.cookie_jar();
    let me = send(&app, get("/whoami").header(header::COOKIE, &jar).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(me.json(), json!({"token": "tok123", "last_room": "7"}));

    let forged = send(&app, get("/whoami").header(header::COOKIE, "session_token=tok123").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(forged.json()["token"], serde_json::Value::Null);

    let out = send(&app, get("/sign_out").header(header::COOKIE, &jar).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(out.cookies(), vec!["session_token=; path=/; max-age=0; expires=Thu, 01 Jan 1970 00:00:00 GMT; samesite=lax"]);
    let nothing = send(&app, get("/sign_out").body(AxumBody::empty()).unwrap()).await;
    assert!(nothing.cookies().is_empty());
}

#[tokio::test]
async fn before_actions_halt_the_chain() {
    let app = app();
    let anonymous = send(&app, get("/admin").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(anonymous.status, StatusCode::FOUND);
    assert_eq!(anonymous.header("location"), Some("http://chat.example.com/session/new"));
    assert_eq!(anonymous.header("x-version"), Some("42"), "headers set before the halt survive");

    let member = send(&app, get("/admin").header("x-user", "jo").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(member.status, StatusCode::FORBIDDEN);
    assert_eq!(member.header("content-type"), Some("text/html"));
    assert!(member.body.is_empty());

    let admin = send(&app, get("/admin").header("x-user", "admin").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(admin.text(), "hi admin");
}

#[tokio::test]
async fn respond_to_negotiates_like_rails() {
    let app = app();
    let browser = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
    let html = send(&app, get("/messages").header(header::ACCEPT, browser).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(html.header("content-type"), Some("text/html; charset=utf-8"));

    let turbo = "text/vnd.turbo-stream.html, text/html, application/xhtml+xml";
    let stream = send(&app, post("/messages").header(header::ACCEPT, turbo).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(stream.header("content-type"), Some("text/vnd.turbo-stream.html; charset=utf-8"));

    let json = send(&app, get("/messages.json").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(json.header("content-type"), Some("application/json; charset=utf-8"));
    let json = send(&app, get("/messages?format=json").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(json.header("content-type"), Some("application/json; charset=utf-8"));

    let unacceptable = send(&app, get("/messages").header(header::ACCEPT, "image/png").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(unacceptable.status, StatusCode::NOT_ACCEPTABLE);
    let invalid = send(&app, get("/messages").header(header::ACCEPT, "garbage").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(invalid.status, StatusCode::NOT_ACCEPTABLE);

    let any = send(&app, get("/autocomplete").header(header::ACCEPT, "*/*").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(any.header("content-type"), Some("text/html; charset=utf-8"));
}

#[tokio::test]
async fn redirects_like_rails() {
    let app = app();
    let plain = send(&app, get("/redirect").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(plain.status, StatusCode::FOUND);
    assert_eq!(plain.header("location"), Some("http://chat.example.com/rooms/1?x=1"));
    assert_eq!(plain.header("content-type"), Some("text/html; charset=utf-8"));
    assert_eq!(plain.header("cache-control"), Some("no-cache"));

    let see_other = send(&app, get("/redirect?to=see_other").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(see_other.status, StatusCode::SEE_OTHER);

    assert_eq!(send(&app, get("/redirect?to=relative").body(AxumBody::empty()).unwrap()).await.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(send(&app, get("/redirect?to=other").body(AxumBody::empty()).unwrap()).await.status, StatusCode::INTERNAL_SERVER_ERROR);
    let allowed = send(&app, get("/redirect?to=other_allowed").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(allowed.header("location"), Some("https://docs.example/"));

    let back = send(
        &app,
        get("/redirect?to=back").header(header::REFERER, "http://chat.example.com/rooms/3").body(AxumBody::empty()).unwrap(),
    )
    .await;
    assert_eq!(back.header("location"), Some("http://chat.example.com/rooms/3"));
    let foreign = send(
        &app,
        get("/redirect?to=back").header(header::REFERER, "https://evil.example/x").body(AxumBody::empty()).unwrap(),
    )
    .await;
    assert_eq!(foreign.header("location"), Some("http://chat.example.com/fallback"));
}

#[tokio::test]
async fn head_with_location() {
    let reply = send(&app(), get("/created").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(reply.status, StatusCode::CREATED);
    assert_eq!(reply.header("location"), Some("http://chat.example.com/rooms/1/messages/2"));
    assert_eq!(reply.header("content-type"), Some("text/html"));
}

#[tokio::test]
async fn send_file_with_disposition_and_ranges() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("logo.png");
    std::fs::write(&path, b"0123456789").unwrap();
    let encoded = campfire_kit::cookies::escape(path.to_str().unwrap());
    let app = app();

    let whole = send(&app, get(&format!("/file?path={encoded}")).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(whole.body, b"0123456789");
    assert_eq!(whole.header("content-type"), Some("image/png"));
    assert_eq!(whole.header("content-disposition"), Some("inline; filename=\"logo.png\"; filename*=UTF-8''logo.png"));
    assert_eq!(whole.header("content-transfer-encoding"), Some("binary"));
    assert_eq!(whole.header("content-length"), Some("10"));

    let ignored = send(&app, get(&format!("/file?path={encoded}")).header(header::RANGE, "bytes=2-4").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(ignored.status, StatusCode::OK);

    let ranged = send(
        &app,
        get(&format!("/file?path={encoded}&ranges=1")).header(header::RANGE, "bytes=2-4").body(AxumBody::empty()).unwrap(),
    )
    .await;
    assert_eq!(ranged.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(ranged.body, b"234");
    assert_eq!(ranged.header("content-range"), Some("bytes 2-4/10"));

    let unsatisfiable = send(
        &app,
        get(&format!("/file?path={encoded}&ranges=1")).header(header::RANGE, "bytes=20-").body(AxumBody::empty()).unwrap(),
    )
    .await;
    assert_eq!(unsatisfiable.status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(unsatisfiable.header("content-range"), Some("bytes */10"));

    let missing = send(&app, get("/file?path=%2Fnope").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(missing.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn stale_and_expires_in() {
    let app = app();
    let first = send(&app, get("/logo").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(first.header("cache-control"), Some("max-age=300, public, stale-while-revalidate=604800"));
    let etag = first.header("etag").unwrap().to_string();
    assert!(etag.starts_with("W/\""));

    let second = send(&app, get("/logo").header(header::IF_NONE_MATCH, &etag).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(second.status, StatusCode::NOT_MODIFIED);

    let fresh = send(&app, get("/fresh").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(fresh.header("cache-control"), Some("max-age=0, private, must-revalidate"));
    let etag = fresh.header("etag").unwrap().to_string();
    let cached = send(&app, get("/fresh").header(header::IF_NONE_MATCH, &etag).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
    let listed = send(&app, get("/fresh").header(header::IF_NONE_MATCH, format!("\"other\", {etag}")).body(AxumBody::empty()).unwrap()).await;
    assert_eq!(listed.status, StatusCode::NOT_MODIFIED);

    // Turbo Frame requests get a different ETag (turbo-rails' frame etagger).
    let frame = send(&app, get("/fresh").header("turbo-frame", "x").body(AxumBody::empty()).unwrap()).await;
    assert_ne!(frame.header("etag").unwrap(), etag);
}

#[tokio::test]
async fn remote_ip_from_peer_and_proxies() {
    let app = app();
    let mut request = get("/echo/1").header("x-forwarded-for", "203.0.113.9, 10.0.0.1").body(AxumBody::empty()).unwrap();
    request.extensions_mut().insert(ConnectInfo("127.0.0.1:5000".parse::<SocketAddr>().unwrap()));
    assert_eq!(send(&app, request).await.json()["remote_ip"], "203.0.113.9");

    let mut request = get("/echo/1").body(AxumBody::empty()).unwrap();
    request.extensions_mut().insert(ConnectInfo("198.51.100.4:5000".parse::<SocketAddr>().unwrap()));
    assert_eq!(send(&app, request).await.json()["remote_ip"], "198.51.100.4");

    let request = get("/echo/1").header("client-ip", "1.1.1.1").header("x-forwarded-for", "2.2.2.2").body(AxumBody::empty()).unwrap();
    assert_eq!(send(&app, request).await.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn force_ssl_redirects_and_hardens() {
    let app = app_with(KitConfig { force_ssl: true, ..KitConfig::default() });
    let plain = send(&app, get("/rooms/1?x=1").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(plain.status, StatusCode::MOVED_PERMANENTLY);
    assert_eq!(plain.header("location"), Some("https://chat.example.com/rooms/1?x=1"));
    let plain_post = send(&app, form_post("/echo/1", "")).await;
    assert_eq!(plain_post.status, StatusCode::PERMANENT_REDIRECT);

    let production = app_with(KitConfig::production(false));
    let secure = send(&production, get("/sign_in").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(secure.status, StatusCode::OK);
    // Our `ssl_options`: `hsts: { expires: 1.year, subdomains: true }` (`1.year` is 365.2425 days).
    assert_eq!(secure.header("strict-transport-security"), Some("max-age=31556952; includeSubDomains"));
    assert!(secure.cookies().iter().all(|c| c.ends_with("; secure")));

    let https = send(&production, get("/redirect").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(https.header("location"), Some("https://chat.example.com/rooms/1?x=1"));
}

#[tokio::test]
async fn body_limit_is_413() {
    let app = app_with(KitConfig { max_body_bytes: Some(16), ..KitConfig::default() });
    let reply = send(&app, form_post("/echo/1", &"a=1&".repeat(20))).await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    let reply = send(&app, HttpRequest::patch("/echo/1").body(AxumBody::from("x".repeat(40))).unwrap()).await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn request_ids_are_sanitized_passthroughs() {
    let reply = send(&app(), get("/rooms/1").header("x-request-id", "abc-123<script>").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(reply.header("x-request-id"), Some("abc-123script"));
}

async fn slow(c: &mut Ctx) -> Result {
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    Ok(c.head(StatusCode::OK))
}

#[tokio::test]
async fn request_timeout_is_408() {
    let config = KitConfig { request_timeout: Some(std::time::Duration::from_millis(20)), ..KitConfig::default() };
    let app = campfire_kit::app(Router::new().route("/slow", campfire_kit::get(slow)), kit_with(config));
    let reply = send(&app, get("/slow").body(AxumBody::empty()).unwrap()).await;
    assert_eq!(reply.status, StatusCode::REQUEST_TIMEOUT);
}

#[tokio::test]
async fn serves_with_peer_addresses_and_shuts_down_gracefully() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let shutdown = front::Shutdown::when(async {
        let _ = stopped.await;
    });
    let service = front::app_service(app());
    let server = tokio::spawn(front::serve_plain(listener, service, front::Protocol::Http1, front::Options::default(), shutdown.clone()));

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream.write_all(b"GET /echo/1 HTTP/1.1\r\nHost: chat.example.com\r\nConnection: close\r\n\r\n").await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains(r#""remote_ip":"127.0.0.1""#));

    stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), server).await.unwrap().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), shutdown.drained()).await.unwrap();
}

/// `rate_limit to: 2, within: 1.minute, only: :create` with the default `with:` (a 429), and a
/// second, named limit that renders its own rejection.
async fn limited(c: &mut Ctx) -> Result {
    c.rate_limit(&campfire_kit::RateLimit::new("limited", 2, jiff::SignedDuration::from_mins(1)))?;
    let by_name = campfire_kit::RateLimit::new("limited", 3, jiff::SignedDuration::from_mins(1)).named("by_user");
    let user = c.request.header("x-user").unwrap_or("nobody").to_string();
    if c.rate_limited(&by_name, Some(&user))? {
        return Ok(c.render(StatusCode::TOO_MANY_REQUESTS, &HTML, "slow down"));
    }
    Ok(c.head(StatusCode::NO_CONTENT))
}

#[tokio::test]
async fn rate_limit_hits_the_limit_then_resets() {
    let clock = std::sync::Arc::new(campfire_kit::FrozenClock::new("2026-01-01T12:00:00Z".parse().unwrap()));
    let error_pages = ErrorPages::new([(429, "<h1>Too many</h1>".into())]);
    let kit = Kit::new(KitConfig { error_pages, ..KitConfig::default() }, testing::crypto(), clock.clone(), AppState { name: "Campfire" });
    let app = campfire_kit::app(Router::new().route("/limited", campfire_kit::get(limited)), kit);
    let hit = |ip: &'static str, user: &'static str| {
        let app = app.clone();
        async move {
            let mut request = get("/limited").header("x-user", user).body(AxumBody::empty()).unwrap();
            request.extensions_mut().insert(ConnectInfo(format!("{ip}:5000").parse::<SocketAddr>().unwrap()));
            send(&app, request).await
        }
    };

    let statuses = [hit("10.0.0.1", "a").await.status, hit("10.0.0.1", "a").await.status, hit("10.0.0.1", "a").await.status];
    assert_eq!(statuses, [StatusCode::NO_CONTENT, StatusCode::NO_CONTENT, StatusCode::TOO_MANY_REQUESTS]);
    let limited = hit("10.0.0.1", "a").await;
    assert_eq!((limited.status, limited.text().as_str()), (StatusCode::TOO_MANY_REQUESTS, "<h1>Too many</h1>"));
    assert_eq!(hit("10.0.0.2", "a").await.status, StatusCode::NO_CONTENT, "another IP has its own count");

    // The named limit counts by user: "a" has made 3 requests that got past the first limit.
    let by_user = hit("10.0.0.3", "a").await;
    assert_eq!((by_user.status, by_user.text().as_str()), (StatusCode::TOO_MANY_REQUESTS, "slow down"));

    clock.advance(jiff::SignedDuration::from_secs(59));
    assert_eq!(hit("10.0.0.1", "b").await.status, StatusCode::TOO_MANY_REQUESTS, "still inside the window");
    clock.advance(jiff::SignedDuration::from_secs(1));
    assert_eq!(hit("10.0.0.1", "b").await.status, StatusCode::NO_CONTENT, "the window from the first request is over");
    assert_eq!(hit("10.0.0.3", "a").await.status, StatusCode::NO_CONTENT);
}
