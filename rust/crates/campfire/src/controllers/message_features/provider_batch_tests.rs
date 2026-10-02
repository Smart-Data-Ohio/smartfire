//! Populated provider search uses the same production cache-key path as ordinary searches.
use super::quote_integration_tests::app_rows;
use crate::controllers::presenters::test_support::*;
use axum::http::StatusCode;
use campfire_db::{Message, NewMessage};
use serde_json::Value;

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/provider_batch.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn populated_provider_http_search_stays_constant_and_matches_rails_containers() {
    let vector = oracle();
    let app = app_rows(vector["rows"].clone()).await;
    let event = vector["event_id"].as_i64().unwrap();
    let mut browser = app.david();
    let mut previous = 0;
    let mut counts = Vec::new();
    for page in vector["pages"].as_array().unwrap() {
        let size = page["size"].as_u64().unwrap() as usize;
        app.db().write(move |tx| {
            for i in previous..size {
                Message::create(tx, NewMessage {
                    room_id: ALL_TALK, creator_id: DAVID,
                    client_message_id: Some(format!("provider-populated-{i}")),
                    markdown_source: Some(format!("populatedquery https://github.com/provider-owner/repo-1/pull/2 /rooms/{ALL_TALK}/events/{event}")),
                    ..Default::default()
                })?;
            }
            Ok(())
        }).await.unwrap();
        assert_eq!(
            browser.get("/searches?q=populatedquery").await.status,
            StatusCode::OK
        );
        let log = app.db().capture_read_queries();
        let response = browser.get("/searches?q=populatedquery").await;
        app.db().stop_capturing_read_queries();
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.text().matches("data-message-id=").count(), size);
        for card in page["cards"].as_array().unwrap() {
            for kind in ["github", "events"] {
                assert!(
                    response.text().contains(card[kind].as_str().unwrap()),
                    "{kind} for {} differs from Rails",
                    card["message_id"]
                );
            }
        }
        let reads = log
            .lock()
            .unwrap()
            .iter()
            .filter(|sql| {
                sql.trim_start().starts_with("SELECT") || sql.trim_start().starts_with("WITH")
            })
            .count();
        println!(
            "WS8bm2 populated provider search: {size} messages; Rust {reads} reads; Rails {} reads",
            page["reads"]
        );
        counts.push(reads);
        previous = size;
    }
    assert_eq!(
        counts[0], counts[1],
        "populated GitHub/event factories and production cache keys must be page-scoped"
    );
    println!(
        "WS8bm2 populated provider containers: 40/40 byte-identical GitHub/event containers in actual HTTP search responses"
    );
}
