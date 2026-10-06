use super::*;
use campfire_app::integrations::opengraph::fetch;
use rails_compat::Secrets;
use rails_compat::verifiers::embed_image;
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[test]
fn ws15e_signed_image_urls_match_rails_in_both_directions() {
    let secrets = Secrets::new(
        include_str!("../../../../../parity/.env.reference").lines().find_map(|line| line.strip_prefix("SECRET_KEY_BASE=")).unwrap(),
    );
    let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../../vectors/ws15e_embeds.json")).unwrap();
    for entry in vectors["signed"].as_array().unwrap() {
        let signed = embed_image::sign(&secrets, entry["url"].as_str().unwrap());
        // Rails verifies this exact byte string in generate.rb; Rust verifies Rails' output.
        assert_eq!(signed, entry["signed"].as_str().unwrap());
        assert_eq!(verified_url(&secrets, &signed, jiff::Timestamp::now()).as_deref(), entry["verified_url"].as_str());
        assert_eq!(verified_url(&secrets, &format!("{signed}x"), jiff::Timestamp::now()), None);
        assert_eq!(signed_path(&secrets, entry["url"].as_str().unwrap()), entry["path"].as_str().unwrap());
    }
}

#[tokio::test]
async fn ws15e_rendered_embed_html_matches_rails_and_uses_the_proxy() {
    let app = crate::controllers::presenters::test_support::TestApp::boot().await.expect("build parity seed before this test");
    let secrets = app.booted.app.secrets.clone();
    let html = app
        .db()
        .read(move |conn| {
            let resolver = crate::controllers::presenters::DbResolver::new(conn, &secrets, jiff::Timestamp::now());
            let embed = campfire_richtext::attachables::OpengraphEmbed {
                href: Some("https://example.com/page".into()),
                url: Some("https://example.com/image.png".into()),
                filename: Some("Title".into()),
                description: Some("Description".into()),
            };
            Ok(campfire_richtext::attachables::render_opengraph_embed(&embed, &resolver.render_context(Some("campfire.test".into())))
                .unwrap())
        })
        .await
        .unwrap();
    let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../../vectors/ws15e_embeds.json")).unwrap();
    assert_eq!(html, vectors["embed_html"].as_str().unwrap());
    assert!(!html.contains("https://example.com/image.png"));
}

#[tokio::test]
async fn ws15e_image_proxy_redirect_budget_is_ten_requests() {
    let mut routes: Vec<Route> = (0..10)
        .map(|i| Route::new("GET", "images.example.com", &format!("/loop-{i}"), 302).header("Location", &format!("/loop-{}", i + 1)))
        .collect();
    routes.extend((0..10).map(|i| {
        if i == 9 {
            Route::new("GET", "images.example.com", &format!("/ok-{i}"), 200).header("Content-Type", "image/png").body("image")
        } else {
            Route::new("GET", "images.example.com", &format!("/ok-{i}"), 302).header("Location", &format!("/ok-{}", i + 1))
        }
    }));
    let server = FakeServer::start_ws15e(routes).await;
    let resolver = Arc::new(FakeResolver::new([("images.example.com", vec!["93.184.216.34"])]));
    let dialer = Arc::new(MappingDialer {
        public: HashSet::from(["93.184.216.34".parse().unwrap()]),
        to: server.addr,
        dialed: Mutex::new(Vec::new()),
    });
    let net = network(resolver, dialer);
    assert!(matches!(
        fetch(&net, "http://images.example.com/loop-0").await,
        Err(ProxyError::Fetch(fetch::FetchError::TooManyRedirects))
    ));
    assert_eq!(server.received().len(), 10);
    assert_eq!(fetch(&net, "http://images.example.com/ok-0").await.unwrap().body, b"image");
    assert_eq!(server.received().len(), 20);
}
