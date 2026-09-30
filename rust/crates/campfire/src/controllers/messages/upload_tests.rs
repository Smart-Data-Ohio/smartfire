use std::sync::Arc;
use axum::http::{Method,StatusCode};
use campfire_db::{ChannelThread,Message,NewChannelThread};
use campfire_kit::clock::FrozenClock;
use serde_json::{Value,json};
use crate::controllers::presenters::test_support::*;

fn oracle() -> Value {serde_json::from_str(include_str!("../../../../../vectors/messaging/signed-attachments.json")).unwrap()}
#[tokio::test]
async fn signed_root_and_thread_attachments_match_rails_response_and_blob_rows() {
    let clock=Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.unwrap();
    let id = app.db().write(|tx| ChannelThread::create(tx,NewChannelThread {room_id:ALL_TALK,creator_id:DAVID,name:Some("Signed attachments".into()),..Default::default()}).map(|t|t.id)).await.unwrap();
    assert_eq!(oracle()["thread_id"],id);
    for row in oracle()["rows"].as_array().unwrap() {
        clock.set(row["now"].as_str().unwrap().parse().unwrap());
        let response = app.david().write(Req::new(Method::from_bytes(row["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),row["path"].as_str().unwrap())
            .header("content-type","application/json").header("accept","application/json").body(row["input"].to_string())).await;
        let name = row["name"].as_str().unwrap();
        assert_eq!(response.status.as_u16(),row["status"].as_u64().unwrap() as u16,"{name}: {}",response.text());
        assert_eq!(response.header("cache-control"),row["cache_control"].as_str(),"{name}");
        assert_eq!(response.content_type(),row["content_type"].as_str(),"{name}");
        assert_eq!(response.location(),row["location"].as_str(),"{name}");
        if response.text()!=row["body"].as_str().unwrap(){rails_mismatch(&response.text(),row["body"].as_str().unwrap(),name);}
        let client=row["input"]["message"]["client_message_id"].as_str().unwrap().to_string();
        let state=app.db().read(move |conn| {
            let m=Message::find_duplicate(conn,ALL_TALK,DAVID,&client)?;
            let state=m.map(|m| {
                let blob=m.attachment(conn)?.map(|(_,b)|b);
                Ok::<_,campfire_db::Error>(json!({"id":m.id,"client_message_id":m.client_message_id,"markdown_source":m.markdown_source,
                    "attachment_id":blob.as_ref().map(|b|b.id),"content_type":blob.as_ref().and_then(|b|b.content_type.clone()),
                    "metadata":blob.and_then(|b|b.metadata).map(|s|serde_json::from_str::<Value>(&s).unwrap())}))
            }).transpose()?;
            let count=conn.query_row("SELECT COUNT(*) FROM messages",[],|r|r.get::<_,i64>(0))?;
            Ok((json!(state),count))
        }).await.unwrap();
        assert_eq!(state.0,row["message"],"{name}");assert_eq!(state.1,row["message_count"],"{name}");
    }
}

#[tokio::test]
async fn signed_attachment_scope_and_enqueue_rollback_preserve_existing_blob() {
    let app=TestApp::boot().await.unwrap();
    let thread=app.db().write(|tx|ChannelThread::create(tx,NewChannelThread{room_id:ALL_TALK,creator_id:DAVID,..Default::default()})).await.unwrap();
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
