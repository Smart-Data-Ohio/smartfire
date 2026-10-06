//! Real Rails query-string coercions, including exception response bytes.
use crate::controllers::presenters::test_support::*;
use serde_json::{Value, json};

async fn cases(key: &str) {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/user_coercions.json"
    )).unwrap();
    let app = TestApp::boot_with_test_clock(std::sync::Arc::new(
        campfire_kit::clock::FrozenClock::new(SEED_NOW.parse().unwrap())
    )).await.expect("default seed required").without_job_runner().await;
    let mut browser = app.david();
    for case in oracle["cases"].as_array().unwrap().iter().filter(|c| c["key"] == key) {
        let path = case["path"].as_str().unwrap();
        let response = browser.get(path).await;
        assert_eq!(response.status.as_u16(), case["status"].as_u64().unwrap() as u16, "{path}");
        assert_eq!(response.header("link"), case["links"].as_str(), "{path}");
        if response.status.is_success() {
            let names = response.json().as_array().unwrap().iter()
                .map(|u| u["markdown_display_name"].clone()).collect::<Vec<_>>();
            assert_eq!(json!(names), case["names"], "{path}");
        } else {
            assert_eq!(response.text(), case["error_body"].as_str().unwrap(), "{path}");
        }
    }
}
#[tokio::test]
async fn user_coercion_query_requests_match_rails() { cases("query").await; }
#[tokio::test]
async fn user_coercion_page_requests_match_rails() { cases("page").await; }
#[tokio::test]
async fn user_coercion_room_requests_match_rails() { cases("room_id").await; }
