//! Named root/thread Drive contracts through authorized requests and pinned Rails responses.
use std::sync::Arc;
use axum::http::Method;
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, ThreadMembership, Tx};
use campfire_kit::clock::FrozenClock;
use serde_json::{Value, json};
use crate::controllers::presenters::test_support::*;

pub(crate) fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/drive-controllers.json")).unwrap() }

pub(crate) fn setup(tx: &mut Tx<'_>, row: &Value) -> campfire_db::Result<()> {
    let thread = if row["mode"] == "thread" {
        let thread = ChannelThread::create(tx, NewChannelThread {room_id: ALL_TALK, creator_id: DAVID, name: Some(format!("Drive {}", row["name"].as_str().unwrap())), ..Default::default()})?;
        assert_eq!(thread.id, row["thread_id"]);
        ThreadMembership::join(tx, thread.id, DAVID)?;
        Some(thread)
    } else {None};
    if row["setup"].is_object() {
        let attrs = NewMessage {room_id: ALL_TALK, creator_id: DAVID,
            markdown_source: row["setup"]["source"].as_str().map(str::to_owned),
            client_message_id: row["client_id"].as_str().map(str::to_owned),
            drive_file_ids: row["setup"]["drive"].as_array().unwrap().iter().map(|id| id.as_str().unwrap().to_string()).collect(), ..Default::default()};
        let message = if let Some(mut thread) = thread {thread.post_message(tx, DAVID, attrs)?} else {Message::create(tx, attrs)?};
        assert_eq!(message.id, row["setup"]["id"]);
    }
    Ok(())
}

pub(crate) fn request(row: &Value) -> Req {
    Req::new(Method::from_bytes(row["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(), row["path"].as_str().unwrap())
        .header("content-type", "application/json").header("accept", "application/json")
        .body(json!({"message": row["input"]}).to_string())
}

#[tokio::test]
async fn root_and_thread_drive_requests_match_rails_bytes_order_json_validation_and_rollback() {
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    for row in oracle()["rows"].as_array().unwrap() {
        let fixture = row.clone();
        app.db().write(move |tx| setup(tx, &fixture)).await.unwrap();
        let counts = app.db().read(counts).await.unwrap();
        let response = app.sign_in(row["viewer"].as_i64().unwrap()).await.write(request(row)).await;
    if row["content_type"].as_str().is_some_and(|mime| mime.contains("turbo-stream")) {
        assert_eq!(response.status.as_u16(), if row["status"] == 200 { 201 } else { row["status"].as_u64().unwrap() as u16 });
        assert!(response.text().is_empty());
    } else {
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{}/{}: {}", row["mode"], row["name"], response.text());
        assert_eq!(response.location(), row["location"].as_str());
        assert_eq!(response.content_type(), row["content_type"].as_str());
        if response.text() != row["body"].as_str().unwrap() { rails_mismatch(&response.text(), row["body"].as_str().unwrap(), &format!("Drive {}/{}", row["mode"], row["name"])); }
    }
        let key = row["client_id"].as_str().unwrap().to_string();
        let saved = app.db().read(move |conn| {
            let message = Message::find_duplicate(conn, ALL_TALK, DAVID, &key)?;
            message.map(|m| Ok(json!({"id":m.id,"source":m.markdown_source,"drive":m.drive_file_ids(conn)?}))).transpose()
        }).await.unwrap();
        assert_eq!(json!(saved), row["saved"], "{}/{} saved rows", row["mode"], row["name"]);
        if row["extra"]["nonadmin"].is_object() {
            app.db().write(|tx| {tx.conn().execute("UPDATE users SET role=0 WHERE id=?", [JASON])?; Ok(())}).await.unwrap();
            let denied = app.sign_in(JASON).await.write(request(row)).await;
            assert_eq!(denied.status.as_u16(), row["extra"]["nonadmin"]["status"].as_u64().unwrap() as u16);
            assert_eq!(denied.text(), row["extra"]["nonadmin"]["body"].as_str().unwrap());
            app.db().write(|tx| {tx.conn().execute("UPDATE users SET role=1 WHERE id=?", [JASON])?; Ok(())}).await.unwrap();
        }

        let after = app.db().read(self::counts).await.unwrap();
        assert_eq!(json!({"messages":after.0-counts.0,"drive":after.1-counts.1}), row["delta"], "{}/{} row delta", row["mode"], row["name"]);
    }
    println!("WS8bm Drive writes: 30 complete Rails response/row comparisons; rejected creates and edits leave message and attachment counts unchanged");
}
fn counts(conn: &campfire_db::Connection) -> campfire_db::Result<(i64,i64)> {
    Ok((conn.query_row("SELECT COUNT(*) FROM messages",[],|r|r.get(0))?,conn.query_row("SELECT COUNT(*) FROM drive_attachments",[],|r|r.get(0))?))
}

// test/controllers/messages_drive_attachments_test.rb:208: the persisted rows
// must survive the real show route and presenter, with the generic Rails chip.
