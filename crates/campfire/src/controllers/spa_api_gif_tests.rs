//! KLIPY GIF proxy through the real HTTP stack, with a local fake KLIPY server
//! (`FakeServer` over named TLS), never the real network.

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use crate::controllers::presenters::test_support::{
    ALL_TALK, Browser, DAVID, Reply, Req, TestApp, seed_clock,
};
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route};

const KEY: &str = "test-klipy-key";
const GIF_BYTES: &[u8] =
    include_bytes!("../../../../test-support/attachments/stickers/animated.gif");

fn item_json() -> Value {
    json!({
        "id": 8041071659142944u64,
        "slug": "hello-hi-662",
        "title": "Hello Wave",
        "type": "gif",
        "tags": ["hello"],
        "file": {
            "md": {
                "webp": {"url": "https://static.klipy.com/ii/x/md.webp", "width": 498, "height": 498, "size": 1200},
                "gif": {"url": "https://static.klipy.com/ii/x/md.gif", "width": 498, "height": 498, "size": 9000}
            },
            "sm": {
                "webp": {"url": "https://static.klipy.com/ii/x/sm.webp", "width": 220, "height": 220, "size": 400},
                "jpg": {"url": "https://static.klipy.com/ii/x/sm.jpg", "width": 220, "height": 220, "size": 100},
                "gif": {"url": "https://static.klipy.com/ii/x/sm.gif", "width": 220, "height": 220, "size": 2000}
            }
        }
    })
}

fn list_body() -> String {
    json!({"result": true, "data": {"data": [item_json()], "current_page": 1, "per_page": 24, "has_next": true}})
        .to_string()
}

async fn klipy_app() -> (TestApp, FakeServer) {
    let (server, roots) = FakeServer::start_named_tls_ws15e(
        vec![],
        vec![
            "api.klipy.com".into(),
            "static.klipy.com".into(),
            "static.klipy.co".into(),
        ],
    )
    .await;
    let resolver = Arc::new(FakeResolver::new([
        ("api.klipy.com", vec!["93.184.216.34"]),
        ("static.klipy.com", vec!["93.184.216.35"]),
        ("static.klipy.co", vec!["93.184.216.36"]),
    ]));
    let dialer = Arc::new(MappingDialer {
        public: [
            "93.184.216.34".parse().unwrap(),
            "93.184.216.35".parse().unwrap(),
            "93.184.216.36".parse().unwrap(),
        ]
        .into(),
        to: server.addr,
        dialed: Default::default(),
    });
    let network = crate::net::Network {
        resolver,
        dialer,
        tls: crate::net::tls_config(roots),
    };
    let state = crate::integrations::klipy::State::new(network, "https://api.klipy.com".into());
    let app = TestApp::boot_with_klipy(seed_clock(), state, &[("KLIPY_API_KEY", KEY)])
        .await
        .expect("restored default seed");
    (app, server)
}

fn customer(app: &TestApp) -> String {
    crate::integrations::klipy::customer_id(&app.booted.app.config.secret_key_base, DAVID)
}

fn search_path(app: &TestApp, q: &str, page: u32) -> String {
    format!(
        "/api/v1/{KEY}/gifs/search?q={q}&page={page}&per_page=24&customer_id={}&content_filter=high",
        customer(app)
    )
}

fn trending_path(app: &TestApp, page: u32) -> String {
    format!(
        "/api/v1/{KEY}/gifs/trending?page={page}&per_page=24&customer_id={}&content_filter=high",
        customer(app)
    )
}

fn json_route(host: &str, path: &str, status: u16, body: String) -> Route {
    Route::new("GET", host, path, status)
        .header("Content-Type", "application/json")
        .body(body)
}

fn parse<T: serde::de::DeserializeOwned>(reply: &Reply) -> T {
    serde_json::from_slice(&reply.body)
        .unwrap_or_else(|error| panic!("{error}: {}", reply.text()))
}

fn tag(reply: &Reply) -> String {
    let envelope: api::ApiErrorResponse = parse(reply);
    serde_json::to_value(&envelope.error).unwrap()["_tag"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn get(b: &mut Browser<'_>, path: &str) -> Reply {
    b.send(Req::new(Method::GET, path).header("accept", "application/json"))
        .await
}

#[tokio::test]
async fn gifs_report_unavailable_without_a_key() {
    let Some(app) = TestApp::boot_seed_with_env("default", seed_clock(), &[]).await else {
        return;
    };
    let mut b = app.sign_in(DAVID).await;
    let config = get(&mut b, "/api/v1/gifs/config").await;
    assert_eq!(config.status, StatusCode::OK, "{}", config.text());
    let config: api::GifConfig = parse(&config);
    assert!(!config.enabled);
    let trending = get(&mut b, "/api/v1/gifs/trending").await;
    assert_eq!(trending.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(tag(&trending), "Unavailable");
}

#[tokio::test]
async fn gifs_search_trending_and_categories_proxy_klipy() {
    let (app, server) = klipy_app().await;
    server.replace_routes(vec![
        json_route("api.klipy.com", &search_path(&app, "hello", 1), 200, list_body()),
        json_route("api.klipy.com", &trending_path(&app, 1), 200, list_body()),
        json_route(
            "api.klipy.com",
            &format!("/api/v1/{KEY}/gifs/categories"),
            200,
            json!({"result": true, "data": {"locale": "en_US", "categories": [
                {"category": "smile", "query": "smile", "preview_url": "https://static.klipy.com/ii/c/smile.jpg"}
            ]}})
            .to_string(),
        ),
    ]);
    let mut b = app.sign_in(DAVID).await;

    let config: api::GifConfig = parse(&get(&mut b, "/api/v1/gifs/config").await);
    assert!(config.enabled);

    let reply = get(&mut b, "/api/v1/gifs/search?q=hello&page=1").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert!(!reply.text().contains(KEY), "the key must not reach the browser");
    let list: api::GifList = parse(&reply);
    assert_eq!(list.items.len(), 1);
    let item = &list.items[0];
    assert_eq!(item.slug, "hello-hi-662");
    assert_eq!(item.klipy_id, "8041071659142944");
    assert_eq!(item.title, "Hello Wave");
    assert!(item.preview_url.starts_with("/api/v1/gifs/media?u="), "{}", item.preview_url);
    assert!(item.still_url.starts_with("/api/v1/gifs/media?u="), "{}", item.still_url);
    assert_eq!((item.width, item.height), (220, 220));
    assert!(list.has_next);
    assert_eq!(list.page, 1);

    let reply = get(&mut b, "/api/v1/gifs/trending").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let list: api::GifList = parse(&reply);
    assert_eq!(list.items.len(), 1);

    let reply = get(&mut b, "/api/v1/gifs/categories").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let categories: api::GifCategoryList = parse(&reply);
    assert_eq!(categories.categories.len(), 1);
    assert_eq!(categories.categories[0].name, "smile");
    assert_eq!(categories.categories[0].query, "smile");

    let received = server.received();
    let search = received
        .iter()
        .find(|r| r.target.contains("/gifs/search"))
        .expect("KLIPY search was called");
    assert!(search.target.contains("customer_id="), "{}", search.target);
    assert!(
        !search.target.contains(&DAVID.to_string()),
        "the raw user id must not go to KLIPY: {}",
        search.target
    );
    assert!(search.target.contains("content_filter=high"), "{}", search.target);
}

#[tokio::test]
async fn gifs_provider_errors_become_unavailable_not_500() {
    let (app, server) = klipy_app().await;
    server.replace_routes(vec![json_route(
        "api.klipy.com",
        &trending_path(&app, 1),
        500,
        json!({"result": false, "errors": {"message": ["boom"]}}).to_string(),
    )]);
    let mut b = app.sign_in(DAVID).await;
    let reply = get(&mut b, "/api/v1/gifs/trending").await;
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE, "{}", reply.text());
    assert_eq!(tag(&reply), "Unavailable");
    assert!(reply.text().contains("GIFs are unavailable right now"), "{}", reply.text());
}

#[tokio::test]
async fn gifs_identical_trending_requests_are_cached() {
    let (app, server) = klipy_app().await;
    server.replace_routes(vec![json_route(
        "api.klipy.com",
        &trending_path(&app, 1),
        200,
        list_body(),
    )]);
    let mut b = app.sign_in(DAVID).await;
    for _ in 0..2 {
        let reply = get(&mut b, "/api/v1/gifs/trending").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    }
    let calls = server
        .received()
        .iter()
        .filter(|r| r.target.contains("/gifs/trending"))
        .count();
    assert_eq!(calls, 1, "the second identical request must come from the cache");
}

#[tokio::test]
async fn gifs_search_is_rate_limited_per_user() {
    let (app, server) = klipy_app().await;
    let routes = (0..40)
        .map(|i| {
            json_route(
                "api.klipy.com",
                &search_path(&app, &format!("q{i}"), 1),
                200,
                list_body(),
            )
        })
        .collect();
    server.replace_routes(routes);
    let mut b = app.sign_in(DAVID).await;
    let mut limited = None;
    for i in 0..40 {
        let reply = get(&mut b, &format!("/api/v1/gifs/search?q=q{i}")).await;
        if reply.status == StatusCode::TOO_MANY_REQUESTS {
            limited = Some(reply);
            break;
        }
        assert_eq!(reply.status, StatusCode::OK, "q{i}: {}", reply.text());
    }
    let limited = limited.expect("searching must eventually be rate limited");
    assert_eq!(tag(&limited), "RateLimited");
}

#[tokio::test]
async fn gifs_media_proxy_enforces_host_allowlist_type_and_size() {
    let (app, server) = klipy_app().await;
    let big = vec![7u8; 6 * 1024 * 1024];
    server.replace_routes(vec![
        Route::new("GET", "static.klipy.com", "/ii/x/sm.webp", 200)
            .header("Content-Type", "image/webp")
            .body(GIF_BYTES.to_vec()),
        Route::new("GET", "static.klipy.com", "/ii/x/big.webp", 200)
            .header("Content-Type", "image/webp")
            .body(big),
        Route::new("GET", "static.klipy.com", "/ii/x/page.html", 200)
            .header("Content-Type", "text/html")
            .body("<html></html>".to_string()),
    ]);
    let mut b = app.sign_in(DAVID).await;
    let ok = get(
        &mut b,
        "/api/v1/gifs/media?u=https%3A%2F%2Fstatic.klipy.com%2Fii%2Fx%2Fsm.webp",
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.text());
    assert_eq!(ok.body, GIF_BYTES);
    assert_eq!(ok.content_type(), Some("image/webp"));

    let evil = get(
        &mut b,
        "/api/v1/gifs/media?u=https%3A%2F%2Fevil.example%2Fx.webp",
    )
    .await;
    assert_eq!(evil.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", evil.text());
    assert_eq!(tag(&evil), "Validation");

    let html = get(
        &mut b,
        "/api/v1/gifs/media?u=https%3A%2F%2Fstatic.klipy.com%2Fii%2Fx%2Fpage.html",
    )
    .await;
    assert_eq!(html.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", html.text());

    let big = get(
        &mut b,
        "/api/v1/gifs/media?u=https%3A%2F%2Fstatic.klipy.com%2Fii%2Fx%2Fbig.webp",
    )
    .await;
    assert_eq!(big.status, StatusCode::SERVICE_UNAVAILABLE, "{}", big.text());
}

#[tokio::test]
async fn gifs_require_sign_in() {
    let (app, _server) = klipy_app().await;
    let mut b = app.anonymous();
    let reply = get(&mut b, "/api/v1/gifs/trending").await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.text());
}

#[tokio::test]
async fn sending_a_gif_stores_it_as_a_normal_attachment() {
    let (app, server) = klipy_app().await;
    server.replace_routes(vec![
        json_route(
            "api.klipy.com",
            &format!("/api/v1/{KEY}/gifs/items?slugs=hello-hi-662"),
            200,
            list_body(),
        ),
        Route::new("GET", "static.klipy.com", "/ii/x/md.webp", 200)
            .header("Content-Type", "image/webp")
            .body(GIF_BYTES.to_vec()),
    ]);
    let mut b = app.sign_in(DAVID).await;
    let body = json!({
        "clientMessageId": "gif-send-1",
        "markdownSource": "",
        "replyToMessageId": null,
        "replyNotifyAuthor": null,
        "attachmentSignedId": null,
        "gifSlug": "hello-hi-662"
    });
    let reply = b
        .write(
            Req::new(Method::POST, &format!("/api/v1/rooms/{ALL_TALK}/messages"))
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(body.to_string()),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    assert!(!reply.text().contains("klipy.com"), "the timeline must not hotlink KLIPY");
    let message: api::MessageDTO = parse(&reply);
    let attachment = message
        .attachment
        .as_ref()
        .or_else(|| message.attachments.as_ref().and_then(|files| files.first()))
        .expect("the GIF is a normal attachment");
    assert_eq!(attachment.byte_size, GIF_BYTES.len() as i64);
    assert_eq!(attachment.preview, api::AttachmentPreview::Image);
    assert!(
        attachment.filename.to_lowercase().contains("hello"),
        "filename carries the GIF title: {}",
        attachment.filename
    );
    assert!(attachment.url.contains("/rails/active_storage/"), "{}", attachment.url);

    // The same blob carries the KLIPY id/title in its metadata.
    let metadata: String = app
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT metadata FROM active_storage_blobs WHERE filename LIKE 'hello%' ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    assert!(metadata.contains("8041071659142944"), "{metadata}");
    assert!(metadata.contains("Hello Wave"), "{metadata}");

    // And it shows up in the room's Files pane like any other upload.
    let files = get(&mut b, &format!("/api/v1/rooms/{ALL_TALK}/files")).await;
    assert_eq!(files.status, StatusCode::OK, "{}", files.text());
    assert!(files.text().to_lowercase().contains("hello"), "{}", files.text());
}
