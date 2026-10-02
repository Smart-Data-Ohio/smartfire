use super::quote_integration_tests::app_rows;
use crate::controllers::presenters::test_support::*;
use axum::http::StatusCode;
use campfire_db::{Message, NewMessage};
use serde_json::Value;

#[tokio::test]
async fn private_and_unknown_pages_skip_unused_discussions_without_changing_rails_cards() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/private_provider_pages.json"
    ))
    .unwrap();
    let app = app_rows(vector["rows"].clone()).await;
    let mut browser = app.david();
    let mut counts = std::collections::HashMap::new();
    let mut previous = std::collections::HashMap::new();
    let mut differences = Vec::new();
    for page in vector["pages"].as_array().unwrap() {
        let kind = page["kind"].as_str().unwrap().to_owned();
        let size = page["size"].as_u64().unwrap() as usize;
        let start = *previous.get(&kind).unwrap_or(&0);
        let links = page["links"].as_str().unwrap().to_owned();
        let category = kind.clone();
        app.db()
            .write(move |tx| {
                for i in start..size {
                    Message::create(
                        tx,
                        NewMessage {
                            room_id: QUIET_CORNER,
                            creator_id: DAVID,
                            client_message_id: Some(format!("cost-{category}-{i}")),
                            markdown_source: Some(format!("{category}onlyquery {links}")),
                            ..Default::default()
                        },
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let path = format!("/searches?q={kind}onlyquery");
        assert_eq!(browser.get(&path).await.status, StatusCode::OK);
        let log = app.db().capture_read_queries();
        let response = browser.get(&path).await;
        app.db().stop_capturing_read_queries();
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.text().matches("data-message-id=").count(), size);
        for card in page["cards"].as_array().unwrap() {
            assert!(
                response.text().contains(card["html"].as_str().unwrap()),
                "{kind}: {}",
                card["id"]
            );
        }
        let sql = log.lock().unwrap();
        let mappings = sql
            .iter()
            .filter(|s| s.contains("FROM github_pull_request_threads t"))
            .count();
        let reads = sql
            .iter()
            .filter(|s| s.trim_start().starts_with("SELECT") || s.trim_start().starts_with("WITH"))
            .count();
        println!(
            "WS8bm2 {kind} provider page: {size} messages; Rust {reads} reads / {mappings} discussion lookups; Rails {} reads",
            page["reads"]
        );
        if mappings != usize::from(kind == "mixed") {
            differences.push(format!(
                "{kind} {size}: {mappings} unused discussion lookups"
            ));
        }
        if let Some(before) = counts.insert(kind.clone(), reads) {
            assert_eq!(before, reads, "{kind} page cost must stay constant");
        }
        previous.insert(kind, size);
    }
    assert!(
        differences.is_empty(),
        "only public cards can use discussion mappings: {differences:?}"
    );
    println!(
        "WS8bm2 private/unknown/mixed pages: 60/60 Rails card containers; six production warm HTTP searches"
    );
}
