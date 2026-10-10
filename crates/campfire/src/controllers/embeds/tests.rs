use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use campfire_kit::{ActionFn, Ctx, Kit, KitConfig, RailsCrypto};
use futures_util::future::BoxFuture;

use crate::controllers::presenters::test_support::*;
use crate::net::Network;
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};

#[derive(Clone)]
struct ImageAction(Network);
impl<'a> ActionFn<'a> for ImageAction {
    type Fut = BoxFuture<'a, campfire_kit::Result>;
    fn call(&self, c: &'a mut Ctx) -> Self::Fut {
        c.set_current(self.0.clone());
        Box::pin(crate::controllers::dispatch(c))
    }
}

fn install_network(app: &mut TestApp, net: Network) {
    let kit = Kit::new(
        KitConfig::production(true),
        Arc::new(RailsCrypto::new(app.booted.app.secrets.clone())),
        seed_clock(),
        app.booted.app.clone(),
    );
    let routes = axum::Router::new().route("/{*path}", campfire_kit::get(ImageAction(net)));
    app.booted.router = campfire_kit::app(routes, kit);
}

fn path(app: &TestApp, url: &str) -> String {
    crate::integrations::image_proxy::signed_path(&app.booted.app.secrets, url)
}

fn net(server: &FakeServer, resolver: Arc<FakeResolver>) -> Network {
    let dialer = Arc::new(MappingDialer {
        public: HashSet::from(["93.184.216.34".parse().unwrap()]),
        to: server.addr,
        dialed: Mutex::new(Vec::new()),
    });
    network(resolver, dialer)
}

#[tokio::test]
async fn ws15e_image_proxy_requires_sign_in_and_rejects_invalid_signatures() {
    let mut app = TestApp::boot().await.expect("build the parity seed before this test");
    let server = FakeServer::start_ws15e(vec![
        Route::new("GET", "images.example.com", "/image.png", 200).header("Content-Type", "image/png").body("PNG-BYTES"),
    ])
    .await;
    install_network(&mut app, net(&server, Arc::new(FakeResolver::new([("images.example.com", vec!["93.184.216.34"])]))));
    let signed = path(&app, "http://images.example.com/image.png");
    let anonymous = app.anonymous().get(&signed).await;
    assert_eq!(anonymous.status, campfire_kit::StatusCode::FOUND);
    assert_eq!(server.received().len(), 0);
    let mut david = app.david();
    let wrong_name = rails_compat::app_verifier(&app.booted.app.secrets, "fixture_wrong_image").generate(
        &serde_json::json!("http://images.example.com/image.png"),
        None,
        None,
    );
    let not_string = rails_compat::app_verifier(&app.booted.app.secrets, "embed_image").generate(&serde_json::json!(123), None, None);
    for path in [
        "/embeds/image/bogus".to_string(),
        format!("{signed}x"),
        format!("/embeds/image/{wrong_name}"),
        format!("/embeds/image/{not_string}"),
        path(&app, "javascript:alert(1)"),
        path(&app, "//images.example.com/a.png"),
    ] {
        let response = david.get(&path).await;
        assert_eq!(response.status, campfire_kit::StatusCode::NOT_FOUND, "{path}");
        assert!(response.body.is_empty());
    }
    assert_eq!(server.received().len(), 0);
}

#[tokio::test]
async fn ws15e_image_proxy_has_rails_status_body_and_header_matrix() {
    let mut app = TestApp::boot().await.expect("build the parity seed before this test");
    let types = [
        "image/jpeg",
        "image/png",
        "image/gif",
        "image/webp",
        "image/avif",
        "image/bmp",
        "image/x-icon",
        "image/vnd.microsoft.icon",
        "image/svg+xml",
        "text/html",
        "application/octet-stream",
    ];
    let mut routes: Vec<Route> = types
        .iter()
        .enumerate()
        .map(|(i, mime)| Route::new("GET", "images.example.com", &format!("/mime-{i}"), 200).header("Content-Type", mime).body("PNG-BYTES"))
        .collect();
    routes.push(Route::new("GET", "images.example.com", "/bad-status", 500).header("Content-Type", "image/png"));
    routes.push(Route::new("GET", "images.example.com", "/no-type", 200).body("x"));
    routes.push(
        Route::new("GET", "images.example.com", "/large-header", 200)
            .header("Content-Type", "image/png")
            .header("Content-Length", "6291456")
            .body("x"),
    );
    let mut large =
        Route::new("GET", "images.example.com", "/large-body", 200)
            .header("Content-Type", "image/png")
            .body(vec![b'x'; 5 * 1024 * 1024 + 1]);
    large.chunked = true;
    routes.push(large);
    let mut gzipped =
        Route::new("GET", "images.example.com", "/gzip-large", 200)
            .header("Content-Type", "image/png")
            .body(vec![b'x'; 5 * 1024 * 1024 + 1]);
    gzipped.gzip = true;
    routes.push(gzipped);
    let server = FakeServer::start_ws15e(routes).await;
    install_network(&mut app, net(&server, Arc::new(FakeResolver::new([("images.example.com", vec!["93.184.216.34"])]))));
    let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../../vectors/ws15e_embeds.json")).unwrap();
    let expected = &vectors["image_response"];
    let mut david = app.david();
    for (i, mime) in types.iter().enumerate() {
        let response = david.get(&path(&app, &format!("http://images.example.com/mime-{i}"))).await;
        if i < 8 {
            assert_eq!(response.status, campfire_kit::StatusCode::OK);
            assert_eq!(response.body, b"PNG-BYTES");
            assert_eq!(response.headers["content-type"], *mime);
            assert_eq!(response.headers["cache-control"], expected["cache_control"].as_str().unwrap());
            assert_eq!(response.headers["content-disposition"], expected["content_disposition"].as_str().unwrap());
            assert_eq!(response.headers["content-transfer-encoding"], expected["content_transfer_encoding"].as_str().unwrap());
        } else {
            assert_eq!(response.status, campfire_kit::StatusCode::BAD_GATEWAY);
            assert!(response.body.is_empty());
        }
    }
    for target in ["bad-status", "no-type", "large-header", "large-body", "gzip-large"] {
        let response = david.get(&path(&app, &format!("http://images.example.com/{target}"))).await;
        assert_eq!(response.status, campfire_kit::StatusCode::BAD_GATEWAY, "{target}");
        assert!(response.body.is_empty());
    }
}

#[tokio::test]
async fn ws15e_image_proxy_denies_ssrf_and_dns_failures_before_fetching() {
    let mut app = TestApp::boot().await.expect("build the parity seed before this test");
    let server = FakeServer::start_ws15e(vec![
        Route::new("GET", "images.example.com", "/private", 302).header("Location", "http://2130706433/secret"),
        Route::new("GET", "images.example.com", "/bad-redirect", 302).header("Location", "javascript:alert(1)"),
    ])
    .await;
    let resolver = Arc::new(FakeResolver::new([
        ("images.example.com", vec!["93.184.216.34"]),
        ("internal.example.com", vec!["169.254.169.254"]),
        ("empty.example.com", Vec::new()),
    ]));
    install_network(&mut app, net(&server, resolver));
    let mut david = app.david();
    for url in [
        "http://127.0.0.1/secret",
        "http://2130706433/secret",
        "http://[::1]/secret",
        "http://internal.example.com/secret",
        "http://empty.example.com/secret",
        "http://missing.example.com/secret",
    ] {
        let response = david.get(&path(&app, url)).await;
        assert_eq!(response.status, campfire_kit::StatusCode::NOT_FOUND, "{url}");
        assert!(response.body.is_empty());
    }
    assert_eq!(server.received().len(), 0);
    let response = david.get(&path(&app, "http://images.example.com/private")).await;
    assert_eq!(response.status, campfire_kit::StatusCode::NOT_FOUND);
    assert_eq!(server.received().len(), 1);
    let response = david.get(&path(&app, "http://images.example.com/bad-redirect")).await;
    assert_eq!(response.status, campfire_kit::StatusCode::BAD_GATEWAY);
    assert_eq!(server.received().len(), 2);
}

#[tokio::test]
async fn ws15e_room_message_embeds_render_signed_proxy_urls() {
    let app = TestApp::boot().await.expect("build the parity seed before this test");
    app.db().write(|tx| {
        campfire_db::Message::create(tx, campfire_db::NewMessage {
            room_id: ALL_TALK, creator_id: DAVID, client_message_id: Some("ws15e-embed-proxy".into()),
            body: Some(r#"<div class="trix-content"><p>https://example.com/page</p><action-text-attachment content-type="application/vnd.actiontext.opengraph-embed" href="https://example.com/page" url="https://images.example.com/room-photo.png" filename="Example" caption="A page"></action-text-attachment></div>"#.into()),
            attachment_blob_id: None, thread_id: None, system_note: false, streaming: false,
            ..Default::default()
        })?;
        Ok(())
    }).await.unwrap();
    let response = app.david().get(&format!("/api/v1/rooms/{ALL_TALK}/messages")).await;
    assert_eq!(response.status, campfire_kit::StatusCode::OK);
    let html = response.json()["messages"].as_array().unwrap().iter().filter_map(|message| message["bodyHtml"].as_str()).collect::<Vec<_>>().join("\n");
    assert!(html.contains(&path(&app, "https://images.example.com/room-photo.png")));
    assert!(!html.contains("src=\"https://images.example.com/room-photo.png"));
}
