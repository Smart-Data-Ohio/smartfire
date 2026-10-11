//! The configured Google sign-in criterion, through the SPA's sign-in page.
use crate::controllers::presenters::test_support::*;
use serde_json::Value;

#[tokio::test]
async fn configured_sign_in_offers_google_with_its_domains() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_sign_in_google.json"
    ))
    .unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        let environment = case["environment"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str().unwrap()))
            .collect::<Vec<_>>();
        let app = TestApp::boot_with_clock_and_env(
            std::sync::Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap())),
            &environment,
        )
        .await
        .expect("seed required");
        // The retained sign-in page redirects to the SPA's, whose inline boot carries the Google
        // configuration the retained page drew.
        let mut browser = app.anonymous();
        let retained = browser.get("/session/new").await;
        assert_eq!(retained.status, axum::http::StatusCode::FOUND);
        assert_eq!(retained.location(), Some("http://campfire.test/app/session/new"));
        let page = browser.get("/app/session/new").await;
        assert_eq!(page.status, axum::http::StatusCode::OK);
        let methods = crate::controllers::spa::signed_out_contracts_tests::inline_boot(&page)
            ["signInMethods"]
            .clone();
        let configured = case["configured"].as_bool().unwrap();
        assert_eq!(
            methods["google"].as_bool(),
            Some(configured),
            "{}: configuration",
            case["name"]
        );
        if configured {
            assert_eq!(methods["googleDomains"], case["domains"], "{}: domains", case["name"]);
        }
    }
}
