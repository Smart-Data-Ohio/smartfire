//! Waiting flash survives durable sidebar aliases until the SPA shell consumes it.
use crate::controllers::presenters::test_support::{TestApp, Req};
use axum::http::{Method, StatusCode};

#[tokio::test]
async fn profile_flash_survives_sidebar_aliases() {
    let t = TestApp::boot_frozen().await.unwrap();
    let mut browser = t.david();
    let response = browser.write(Req::new(Method::PATCH, "/users/me/profile")
        .form(&[("user[time_zone]", "Asia/Tokyo")])).await;
    assert_eq!(response.status, StatusCode::FOUND);
    for _ in 0..4 {
        let response = browser.send(Req::new(Method::GET, "/users/me/sidebar")
            .header("turbo-frame", "ui_matrix")).await;
        assert_eq!(response.status, StatusCode::FOUND);
        assert!(response.body.is_empty());
        assert_eq!(browser.flash()["notice"], "✓");
    }
    let shell = browser.get("/app/").await;
    assert_eq!(shell.status, StatusCode::OK);
    assert!(shell.text().contains("✓"));
    assert_eq!(browser.flash(), serde_json::json!(null));
}
