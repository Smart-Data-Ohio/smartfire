//! HTTP regressions with expected values recorded by cache-reaction-review.rb in pinned Rails.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Boost, Message, NewMessage};
use campfire_kit::clock::FrozenClock;
use serde_json::{Value, json};
use std::sync::Arc;

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/messaging/cache-reaction-review.json")).unwrap()
}

async fn fixture() -> (TestApp, Arc<FrozenClock>, i64, i64) {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.unwrap();
    let (source, reply) = app.db().write(|tx| {
        let source = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID,
            markdown_source: Some("Review before edit".into()), client_message_id: Some("review-cache-source".into()), ..Default::default() })?;
        let reply = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: JASON,
            markdown_source: Some("Review reply".into()), client_message_id: Some("review-cache-reply".into()),
            reply_to_message_id: Some(source.id), ..Default::default() })?;
        Ok((source.id, reply.id))
    }).await.unwrap();
    assert_eq!(json!(source), oracle()["source_id"]);
    assert_eq!(json!(reply), oracle()["reply_id"]);
    (app, clock, source, reply)
}

#[tokio::test]
async fn review_reaction_pairs_match_ruby_strip_and_legacy_classification() {
    let (app, _, source, _) = fixture().await;
    let mut viewer = app.david();
    for row in oracle()["reactions"].as_array().unwrap() {
        app.db().write(move |tx| {
            for boost in Boost::for_message(tx.conn(), source)? { boost.destroy(tx)?; }
            Ok(())
        }).await.unwrap();
        let content = row["content"].as_str().unwrap();
        for count in row["counts"].as_array().unwrap() {
            let response = viewer.write(Req::new(Method::POST, &format!("/messages/{source}/boosts"))
                .header("content-type", "application/json").body(json!({"boost": {"content": content}}).to_string())).await;
            assert_eq!(response.status, StatusCode::FOUND, "{}", response.text());
            let boosts = app.db().read(move |conn| Boost::for_message(conn, source)).await.unwrap();
            assert_eq!(json!(boosts.len()), *count, "Rails pair for {content:?}");
        }
        let stored = app.db().read(move |conn| Ok(Boost::for_message(conn, source)?.into_iter().map(|boost| boost.content).collect::<Vec<_>>())).await.unwrap();
        assert_eq!(json!(stored), row["stored"], "{content:?}");
    }
}
