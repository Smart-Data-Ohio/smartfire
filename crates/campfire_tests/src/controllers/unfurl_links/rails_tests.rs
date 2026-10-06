//! The seven remaining pinned composer cases, through the registered HTTP action.
use crate::{
    controllers::presenters::test_support::*,
    integrations::{net::Network, test_support::*},
};
use axum::http::{Method, StatusCode};
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
struct NetworkAction(Network, Arc<std::sync::Mutex<Vec<String>>>);
impl<'a> campfire_kit::ActionFn<'a> for NetworkAction {
    type Fut = futures_util::future::BoxFuture<'a, campfire_kit::Result>;
    fn call(&self, c: &'a mut campfire_kit::Ctx) -> Self::Fut {
        c.set_current(self.0.clone());
        let captured = self.1.clone();
        Box::pin(async move {
            let result = crate::controllers::dispatch(c).await;
            if let Err(campfire_kit::Error::ParameterMissing(name)) = &result {
                captured.lock().unwrap().push(name.clone());
            }
            result
        })
    }
}

async fn setup(
    body: &str,
) -> (
    TestApp,
    FakeServer,
    Arc<FakeResolver>,
    Arc<std::sync::Mutex<Vec<String>>>,
) {
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    let (server, roots) = FakeServer::start_named_tls_ws15e(
        vec![
            Route::new("GET", "www.example.com", "/", 200)
                .header("Content-Type", "text/html")
                .body(body),
            Route::new(
                "GET",
                "fxtwitter.com",
                "/dhh/status/834146806594433025",
                200,
            )
            .header("Content-Type", "text/html")
            .body(body),
            Route::new("HEAD", "example.com", "/image.png", 200)
                .header("Content-Type", "image/png"),
        ],
        ["www.example.com", "example.com", "fxtwitter.com"]
            .map(str::to_owned)
            .into(),
    )
    .await;
    let resolver = Arc::new(FakeResolver::new([
        ("www.example.com", vec!["93.184.216.34"]),
        ("example.com", vec!["93.184.216.34"]),
        ("fxtwitter.com", vec!["93.184.216.34"]),
    ]));
    let dialer = Arc::new(MappingDialer {
        public: ["93.184.216.34".parse().unwrap()].into(),
        to: server.addr,
        dialed: Default::default(),
    });
    let missing = Arc::new(std::sync::Mutex::new(Vec::new()));
    let action = NetworkAction(
        Network {
            resolver: resolver.clone(),
            dialer,
            tls: crate::net::tls_config(roots),
        },
        missing.clone(),
    );
    let kit = campfire_kit::Kit::new(
        campfire_kit::KitConfig::production(true),
        Arc::new(campfire_kit::RailsCrypto::new(
            app.booted.app.secrets.clone(),
        )),
        seed_clock(),
        app.booted.app.clone(),
    );
    let routes = axum::Router::new()
        .route(
            "/{*path}",
            campfire_kit::get(action.clone()).merge(campfire_kit::post(action.clone())),
        )
        .route("/", campfire_kit::get(action));
    app.booted.router = campfire_kit::app(routes, kit);
    (app, server, resolver, missing)
}

fn metadata(title: &str, description: &str) -> String {
    format!(
        r#"<html><head><meta property="og:url" content="https://example.com"><meta property="og:title" content="{title}"><meta property="og:description" content="{description}"><meta property="og:image" content="https://example.com/image.png"></head></html>"#
    )
}
async fn post(app: &TestApp, url: &str) -> Reply {
    app.david()
        .write(Req::new(Method::POST, "/unfurl_link").form(&[("url", url)]))
        .await
}

#[tokio::test]
async fn ws15e_rails_composer_create() {
    let (app, server, _, _missing) = setup(&metadata("Hey!", "desc..")).await;
    let r = post(&app, "https://www.example.com").await;
    assert_eq!(r.status, StatusCode::OK);
    let json = r.json();
    assert_eq!(json["title"], "Hey!");
    assert_eq!(json["url"], "https://example.com");
    assert_eq!(json["image"], "https://example.com/image.png");
    assert_eq!(json["description"], "desc..");
    assert_eq!(server.received().len(), 2);
}
#[tokio::test]
async fn ws15e_rails_composer_encoded_markup() {
    let tag = "&#x3c;&#x69;&#x6d;&#x67;&#x20;&#x73;&#x72;&#x63;&#x3d;&#x61;&#x20;&#x6f;&#x6e;&#x65;&#x72;&#x72;&#x6f;&#x72;&#x3d;&#x70;&#x72;&#x6f;&#x6d;&#x70;&#x74;&#x28;&#x31;&#x29;&#x3e;";
    let (app, _s, _, _missing) =
        setup(&metadata(&format!("{tag}Hey!"), &format!("{tag}desc.."))).await;
    let r = post(&app, "https://www.example.com").await;
    assert_eq!(r.status, StatusCode::OK);
    let json = r.json();
    assert_eq!(json["title"], "Hey!");
    assert_eq!(json["description"], "desc..");
    assert!(!r.text().contains("onerror"));
}
#[tokio::test]
async fn ws15e_rails_composer_missing_tags() {
    let (app, _s, _, _missing) = setup("<html><head></head></html>").await;
    let r = post(&app, "https://www.example.com").await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert!(r.body.is_empty());
}
#[tokio::test]
async fn ws15e_rails_composer_only_markup() {
    let tag = "<img src='x' onerror='alert(document.domain)'/>";
    let (app, _s, _, _missing) = setup(&metadata(tag, tag)).await;
    let r = post(&app, "https://www.example.com").await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert!(r.body.is_empty());
}
#[tokio::test]
async fn ws15e_rails_composer_missing_url() {
    let (app, server, resolver, missing) = setup("").await;
    let r = post(&app, "").await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        missing.lock().unwrap().as_slice(),
        ["url"],
        "original ParameterMissing(url) reaches the adapter"
    );
    assert!(server.received().is_empty());
    assert!(resolver.lookups().is_empty());
}
async fn assert_twitter(host: &str) {
    let (app, server, resolver, _missing) = setup(&metadata("Hey!", "desc..")).await;
    let r = post(
        &app,
        &format!("https://{host}/dhh/status/834146806594433025"),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["title"], "Hey!");
    let requests = server.received();
    assert_eq!(requests[0].target, "/dhh/status/834146806594433025");
    assert_eq!(requests[0].header("Host"), Some("fxtwitter.com"));
    assert!(!resolver.lookups().iter().any(|h| h == host));
}
#[tokio::test]
async fn ws15e_rails_composer_twitter() {
    assert_twitter("twitter.com").await;
}
#[tokio::test]
async fn ws15e_rails_composer_x() {
    assert_twitter("x.com").await;
}

async fn special_card(url: &str) {
    let (app, server, resolver, _missing) = setup(&metadata("Hey!", "desc..")).await;
    let reply = post(&app, url).await;
    assert_eq!(
        reply.status,
        StatusCode::NO_CONTENT,
        "original {url}: special card response"
    );
    assert!(reply.body.is_empty());
    assert!(server.received().is_empty());
    assert!(resolver.lookups().is_empty());
}
#[tokio::test]
async fn original_github_pr_has_no_unfurl() {
    special_card("https://github.com/rails/rails/pull/123").await;
}
#[tokio::test]
async fn original_fizzy_card_has_no_unfurl() {
    special_card("https://app.fizzy.do/897362094/cards/579").await;
}
