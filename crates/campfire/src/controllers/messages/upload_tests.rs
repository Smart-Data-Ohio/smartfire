use std::sync::Arc;
use axum::http::{Method,StatusCode};
use campfire_db::{ChannelThread,Message,NewChannelThread};
use campfire_kit::clock::FrozenClock;
use serde_json::{Value,json};
use crate::controllers::presenters::test_support::*;

fn oracle() -> Value {serde_json::from_str(include_str!("../../../../../vectors/messaging/signed-attachments.json")).unwrap()}
#[tokio::test]
async fn signed_root_and_thread_uploads_require_ownership_and_keep_rails_signature_checks() {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.unwrap().without_job_runner().await;
    let thread = app.db().write(|tx| {
        let thread = ChannelThread::create(tx, NewChannelThread {room_id: ALL_TALK, creator_id: DAVID, ..Default::default()})?;
        for thread_id in [None, Some(thread.id)] {
            Message::create(tx, campfire_db::NewMessage {
                room_id: ALL_TALK, creator_id: DAVID, thread_id,
                markdown_source: Some("Preserved".into()), ..Default::default()
            })?;
        }
        Ok(thread.id)
    }).await.unwrap();
    assert_eq!(oracle()["thread_id"], thread);
    // The recorded reusable blob capabilities now fail ownership. Signature failures retain
    // their Rails responses; replay coverage uses genuinely claimed uploads in grouped_files_tests.
    for row in oracle()["rows"].as_array().unwrap().iter().filter(|row| !row["name"].as_str().unwrap().starts_with("expired_retry")) {
        clock.set(row["now"].as_str().unwrap().parse().unwrap());
        let before = app.db().read(|conn| Ok((
            conn.query_row("SELECT COUNT(*) FROM messages", [], |row| row.get::<_, i64>(0))?,
            conn.query_row("SELECT COUNT(*) FROM active_storage_attachments", [], |row| row.get::<_, i64>(0))?,
        ))).await.unwrap();
        let response = app.david().write(Req::new(Method::from_bytes(row["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(), row["path"].as_str().unwrap())
            .header("content-type", "application/json").header("accept", "application/json").body(row["input"].to_string())).await;
        let claims_seed = row["status"].as_u64().unwrap() < 400
            && row["input"]["message"]["attachment"].as_str().is_some_and(|value| !value.is_empty());
        let expected = if claims_seed { 422 } else { row["status"].as_u64().unwrap() as u16 };
        assert_eq!(response.status.as_u16(), expected, "{}: {}", row["name"], response.text());
        if claims_seed && response.content_type().is_some_and(|kind| kind.contains("json")) {
            assert!(response.text().contains("isn't yours"), "{}", response.text());
        }
        let after = app.db().read(|conn| Ok((
            conn.query_row("SELECT COUNT(*) FROM messages", [], |row| row.get::<_, i64>(0))?,
            conn.query_row("SELECT COUNT(*) FROM active_storage_attachments", [], |row| row.get::<_, i64>(0))?,
        ))).await.unwrap();
        assert_eq!(after, before, "{} must not claim a seed blob", row["name"]);
    }
}

#[tokio::test]
async fn signed_attachment_scope_and_enqueue_rollback_preserve_existing_blob() {
    let app=TestApp::boot().await.unwrap();
    let thread=app.db().write(|tx|ChannelThread::create(tx,NewChannelThread{room_id:ALL_TALK,creator_id:DAVID,..Default::default()})).await.unwrap();
    super::attachment_processing_tests::fixture_upload(&app, 13, DAVID).await;
    let signed=campfire_storage::paths::signed_blob_id(&*app.booted.app.storage.verifier,13,None);
    let body=json!({"message":{"client_message_id":"signed-rollback","attachment":signed}}).to_string();
    assert_eq!(app.sign_in(KEVIN).await.write(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/threads/{}/messages.json",thread.id))
        .header("content-type","application/json").header("accept","application/json").body(body.clone())).await.status,StatusCode::NOT_FOUND);
    app.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER ws8bm_signed_reject_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'signed attachment enqueue failed'); END;")?;Ok(())}).await.unwrap();
    let response=app.david().write(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/messages.turbo_stream"))
        .header("content-type","application/json").body(body)).await;
    assert_eq!(response.status,StatusCode::INTERNAL_SERVER_ERROR);
    let blob=app.db().read(|conn| {assert!(Message::find_duplicate(conn,ALL_TALK,DAVID,"signed-rollback")?.is_none());
        Ok(campfire_storage::Blob::find(conn,13).unwrap().unwrap())}).await.unwrap();
    assert!(!app.booted.app.storage.service.download(&blob.key).unwrap().is_empty());
}
