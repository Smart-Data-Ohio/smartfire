use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage};
use crate::controllers::presenters::test_support::*;

#[tokio::test]
async fn forward_endpoints_scope_sources_and_reject_bots_and_forgery_first() {
    let app = TestApp::boot().await.unwrap();
    let (root, child, thread) = app.db().write(|tx| {
        let root = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: DAVID, markdown_source: Some("Forward source".into()), ..Default::default()})?;
        let thread = ChannelThread::create(tx, NewChannelThread {room_id: ALL_TALK, creator_id: JASON, name: Some("Forward thread".into()), ..Default::default()})?;
        let child = Message::create(tx, NewMessage {room_id: ALL_TALK, thread_id: Some(thread.id), creator_id: JASON, markdown_source: Some("Child".into()), ..Default::default()})?;
        Ok((root.id, child.id, thread.id))
    }).await.unwrap();
    let mut browser = app.david();
    for path in [format!("/rooms/{QUIET_CORNER}/messages/{root}/forwards/destinations.json"), format!("/rooms/{ALL_TALK}/messages/{child}/forwards/destinations.json"), format!("/rooms/{ALL_TALK}/threads/{thread}/messages/{root}/forwards/destinations.json")] {
        assert_eq!(browser.get(&path).await.status, StatusCode::NOT_FOUND);
    }
    let path = format!("/rooms/{ALL_TALK}/messages/{root}/forwards.json");
    assert_eq!(browser.send(Req::new(Method::POST, &path).header("origin", "https://forged.test")
        .header("content-type", "application/json").body(serde_json::json!({"destinations": [{"room_id": QUIET_CORNER}]}).to_string())).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.anonymous().get(&format!("/rooms/{ALL_TALK}/messages/{root}/forwards/destinations.json?bot_key={BENDER_KEY}")).await.status, StatusCode::FORBIDDEN);
    assert_eq!(app.sign_in(KEVIN).await.write(Req::new(Method::POST, &path)).await.status, StatusCode::NOT_FOUND);
}

use std::sync::Arc;
use serde_json::{Value, json};
use campfire_db::{Room, RoomType, ThreadMembership};
use campfire_kit::clock::FrozenClock;
fn oracle() -> Value {serde_json::from_str(include_str!("../../../../vectors/messaging/forwards.json")).unwrap()}

async fn fixture() -> TestApp {
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    app.db().write(|tx| {
        let source = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: DAVID, markdown_source: Some("**Snapshot**".into()), client_message_id: Some("forward-source".into()), drive_file_ids: vec!["abcdefghij".into()], ..Default::default()})?;
        let open = ChannelThread::create(tx, NewChannelThread {room_id: ALL_TALK, creator_id: DAVID, name: Some("Forward open".into()), ..Default::default()})?;
        let mut locked = ChannelThread::create(tx, NewChannelThread {room_id: ALL_TALK, creator_id: DAVID, name: Some("Forward locked".into()), ..Default::default()})?;
        locked.lock_conversation(tx)?;
        let stale = ChannelThread::create(tx, NewChannelThread {room_id: ALL_TALK, creator_id: DAVID, name: Some("Forward stale".into()), ..Default::default()})?;
        tx.conn().execute("UPDATE channel_threads SET last_activity_at = ?, auto_archive_after_minutes = 60 WHERE id = ?", (tx.now().since(jiff::SignedDuration::from_hours(-2)), stale.id))?;
        let child = Message::create(tx, NewMessage {room_id: ALL_TALK, thread_id: Some(open.id), creator_id: DAVID, markdown_source: Some("Nested source".into()), client_message_id: Some("forward-child".into()), ..Default::default()})?;
        let board = Room::create_for(tx, RoomType::Board, Some("Forward board"), DAVID, &[DAVID])?;
        let copy = Message::create(tx, NewMessage {room_id: QUIET_CORNER, creator_id: DAVID, body: source.body_html(tx.conn())?, forwarded_from_message_id: Some(source.id), forwarded_at: Some(tx.now()), forwarded_markdown: true, client_message_id: Some("forward-copy".into()), ..Default::default()})?;
        let nested_copy = Message::create(tx, NewMessage {room_id: ALL_TALK, thread_id: Some(open.id), creator_id: DAVID, body: child.body_html(tx.conn())?, forwarded_from_message_id: Some(child.id), forwarded_at: Some(tx.now()), forwarded_markdown: true, client_message_id: Some("forward-nested-copy".into()), ..Default::default()})?;
        tx.conn().execute("UPDATE channel_threads SET closed_at = NULL, last_activity_at = ? WHERE id = ?", (tx.now().since(jiff::SignedDuration::from_hours(-2)), stale.id))?;
        for (key, id) in [("source_id",source.id),("thread_id",open.id),("locked_id",locked.id),("stale_id",stale.id),("child_id",child.id),("board_id",board.id),("copy_id",copy.id),("nested_copy_id",nested_copy.id)] {assert_eq!(oracle()[key],id);}
        Ok(())
    }).await.unwrap();
    app
}
#[tokio::test]
async fn pickers_refusals_and_private_source_urls_match_rails_bytes() {
    let app = fixture().await;
    let mut browser = app.david();
    for row in oracle()["rows"].as_array().unwrap() {
        if row["name"] == "source_inaccessible" {app.db().write(|tx| {tx.conn().execute("DELETE FROM memberships WHERE room_id = ? AND user_id = ?", (ALL_TALK,DAVID))?;Ok(())}).await.unwrap();}
        let response = browser.write(Req::new(Method::from_bytes(row["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(), row["path"].as_str().unwrap())
            .header("content-type","application/json").body(row["input"].to_string())).await;
        let name = row["name"].as_str().unwrap();
        assert_eq!(response.status.as_u16(),row["status"].as_u64().unwrap() as u16,"{name}: {}",response.text());
        assert_eq!(response.header("cache-control"),row["cache_control"].as_str(),"{name}");
        assert_eq!(response.content_type(),row["content_type"].as_str(),"{name}");
        if response.text()!=row["body"].as_str().unwrap() {rails_mismatch(&response.text(),row["body"].as_str().unwrap(),name);}
        let count=app.db().read(|conn|Ok(conn.query_row("SELECT COUNT(*) FROM messages",[],|row|row.get::<_,i64>(0))?)).await.unwrap();
        assert_eq!(row["message_count"],count);
    }
    let id = oracle()["stale_id"].as_i64().unwrap();
    assert!(app.db().read(move|conn|ChannelThread::find(conn,id)).await.unwrap().closed_at.is_none());
}

#[tokio::test]
async fn forward_snapshots_drive_ids_and_thread_membership_survive_source_edit_and_deletion() {
    let app = fixture().await;
    let source = oracle()["source_id"].as_i64().unwrap();
    let thread = oracle()["stale_id"].as_i64().unwrap();
    let response = app.david().write(Req::new(Method::POST,&format!("/messages/{source}/forwards.json"))
        .header("content-type","application/json").body(json!({"forward":{"note":"Note <&>","destinations":[{"room_id":QUIET_CORNER},{"room_id":ALL_TALK,"thread_id":thread}]}}).to_string())).await;
    assert_eq!(response.status,StatusCode::CREATED,"{}",response.text());
    assert_eq!(response.header("cache-control"),Some("no-store"));
    let ids=response.json()["forwards"].as_array().unwrap().iter().map(|row|row["message"]["id"].as_i64().unwrap()).collect::<Vec<_>>();
    assert_eq!(ids.len(),2);
    app.db().write(move|tx|{let mut message=Message::find(tx.conn(),source)?;message.edit(tx,campfire_db::MessageChanges{markdown_source:Some("Changed".into()),..Default::default()})?;message.destroy(tx)}).await.unwrap();
    app.db().read(move|conn| {
        let first=Message::find(conn,ids[0])?;
        let second=Message::find(conn,ids[1])?;
        assert_ne!(first.client_message_id,second.client_message_id);
        assert!(first.client_message_id.parse::<uuid::Uuid>().is_ok());
        for message in [first,second] {
            assert_eq!(message.body_html(conn)?.as_deref(),Some("<p><strong>Snapshot</strong></p>"));
            assert_eq!(message.forward_note.as_deref(),Some("Note <&>"));
            assert!(message.forwarded_markdown);
            assert_eq!(message.forwarded_from_message_id,None);
            assert_eq!(message.drive_file_ids(conn)?,vec!["abcdefghij"]);
        }
        let thread=ChannelThread::find(conn,thread)?;
        assert!(thread.closed_at.is_none());
        assert!(ThreadMembership::find_by_thread_and_user(conn,thread.id,DAVID)?.is_some());
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn forwarded_attachment_is_a_private_copy_and_failed_enqueue_removes_all_copies() {
    let app = fixture().await;
    let uploaded=app.david().write(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/messages.turbo_stream"))
        .multipart(&[("message[client_message_id]","copy-upload")],("message[attachment]","forward.txt","text/plain",b"Private copy\n"))).await;
    assert_eq!(uploaded.status,StatusCode::OK,"{}",uploaded.text());
    let id=app.db().read(|conn|Ok(Message::find_duplicate(conn,ALL_TALK,DAVID,"copy-upload")?.unwrap().id)).await.unwrap();
    let path=format!("/messages/{id}/forwards.json");
    let response=app.david().write(Req::new(Method::POST,&path).header("content-type","application/json")
        .body(json!({"destinations":[{"room_id":QUIET_CORNER}]}).to_string())).await;
    assert_eq!(response.status,StatusCode::CREATED,"{}",response.text());
    let copy=response.json()["forwards"][0]["message"]["id"].as_i64().unwrap();
    let (original,copied)=app.db().read(move|conn|Ok((Message::find(conn,id)?.attachment(conn)?.unwrap().1,Message::find(conn,copy)?.attachment(conn)?.unwrap().1))).await.unwrap();
    assert_ne!(original.id,copied.id);
    assert_ne!(original.key,copied.key);
    assert_eq!(original.checksum,copied.checksum);
    assert_eq!(app.booted.app.storage.service.download(&copied.key).unwrap(),b"Private copy\n");
    let files_before=storage_keys(&app);
    let before=app.db().read(|conn|Ok(conn.query_row("SELECT COUNT(*) FROM messages",[],|row|row.get::<_,i64>(0))?)).await.unwrap();
    app.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER ws8bm_forward_reject_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'WS8bm copy rollback'); END;")?;Ok(())}).await.unwrap();
    let rejected=app.david().write(Req::new(Method::POST,&path).header("content-type","application/json")
        .body(json!({"destinations":[{"room_id":QUIET_CORNER},{"room_id":ALL_TALK}]}).to_string())).await;
    assert_eq!(rejected.status,StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(storage_keys(&app),files_before);
    let after=app.db().read(|conn|Ok(conn.query_row("SELECT COUNT(*) FROM messages",[],|row|row.get::<_,i64>(0))?)).await.unwrap();
    assert_eq!(after,before);
}
fn storage_keys(app:&TestApp)->Vec<String> {
    fn visit(path:&std::path::Path,keys:&mut Vec<String>){for entry in std::fs::read_dir(path).unwrap(){let entry=entry.unwrap();if entry.file_type().unwrap().is_dir(){visit(&entry.path(),keys)}else{keys.push(entry.path().to_string_lossy().into_owned())}}}
    let mut keys=Vec::new();visit(app.booted.app.storage.service.root(),&mut keys);keys.sort();keys
}

#[tokio::test]
async fn attachment_processing_failure_rolls_back_every_forward_and_thread_side_effect() {
    let app = fixture().await;
    let thread = oracle()["stale_id"].as_i64().unwrap();
    let uploaded = app.david().write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages.turbo_stream"))
        .multipart(&[("message[client_message_id]", "processing-rollback")], ("message[attachment]", "source.txt", "text/plain", b"Forward rollback\n"))).await;
    assert_eq!(uploaded.status, StatusCode::OK, "{}", uploaded.text());
    let (source, counts, previous_thread) = app.db().write(move |tx| {
        let source = Message::find_duplicate(tx.conn(), ALL_TALK, DAVID, "processing-rollback")?.unwrap();
        // Put the target back into its pre-post state; a successful forward would join/reopen it.
        tx.conn().execute("DELETE FROM thread_memberships WHERE thread_id = ? AND user_id = ?", (thread, DAVID))?;
        tx.conn().execute("UPDATE channel_threads SET closed_at = ?, last_activity_at = ? WHERE id = ?", (tx.now(), tx.now().since(jiff::SignedDuration::from_hours(-2)), thread))?;
        tx.conn().execute_batch("CREATE TRIGGER ws8bm_forward_reject_analysis BEFORE UPDATE OF metadata ON active_storage_blobs WHEN NEW.id > (SELECT blob_id FROM active_storage_attachments WHERE record_type = 'Message' AND record_id = (SELECT id FROM messages WHERE client_message_id = 'processing-rollback')) BEGIN SELECT RAISE(ABORT, 'WS8bm processing rollback'); END;")?;
        Ok((source.id, forward_row_counts(tx.conn())?, ChannelThread::find(tx.conn(), thread)?))
    }).await.unwrap();
    let files = storage_keys(&app);
    let response = app.david().write(Req::new(Method::POST, &format!("/messages/{source}/forwards.json"))
        .header("content-type", "application/json").body(json!({"destinations": [{"room_id": ALL_TALK, "thread_id": thread}, {"room_id": QUIET_CORNER}]}).to_string())).await;
    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR, "{}", response.text());
    app.db().read(move |conn| {
        assert_eq!(forward_row_counts(conn)?, counts, "processing must participate in the forward transaction");
        assert_eq!(ChannelThread::find(conn, thread)?, previous_thread);
        assert!(ThreadMembership::find_by_thread_and_user(conn, thread, DAVID)?.is_none());
        Ok(())
    }).await.unwrap();
    assert_eq!(storage_keys(&app), files, "no copied or generated files survive rollback");
}

fn forward_row_counts(conn: &campfire_db::Connection) -> campfire_db::Result<Vec<i64>> {
    ["messages", "action_text_rich_texts", "active_storage_blobs", "active_storage_attachments", "active_storage_variant_records", "thread_memberships"]
        .iter().map(|table| Ok(conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))?)).collect()
}

#[tokio::test]
async fn generated_media_rolls_back_on_later_target_and_deferred_enqueue_failures() {
    for trigger in [
        "CREATE TRIGGER ws8bm_reject_later_variant BEFORE INSERT ON active_storage_variant_records WHEN (SELECT COUNT(*) FROM messages WHERE forwarded_from_message_id = (SELECT id FROM messages WHERE client_message_id = 'variant-source')) > 1 BEGIN SELECT RAISE(ABORT,'later variant failed'); END;",
        "CREATE TRIGGER ws8bm_reject_media_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'media enqueue failed'); END;"
    ] {
        let app = fixture().await;
        let id = app.db().write(|tx| Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: DAVID, body: Some("Media snapshot".into()),
            client_message_id: Some("variant-source".into()), attachment_blob_id: Some(1), ..Default::default()}).map(|m| m.id)).await.unwrap();
        let before = app.db().read(forward_row_counts).await.unwrap();
        let files = storage_keys(&app);
        app.db().write(move |tx| {tx.conn().execute_batch(trigger)?;Ok(())}).await.unwrap();
        let response = app.david().write(Req::new(Method::POST, &format!("/messages/{id}/forwards.json"))
            .header("content-type", "application/json").body(json!({"destinations": [{"room_id": QUIET_CORNER}, {"room_id": ALL_TALK}]}).to_string())).await;
        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR, "{}", response.text());
        assert_eq!(app.db().read(forward_row_counts).await.unwrap(), before);
        assert_eq!(storage_keys(&app), files);
    }
}

pub(crate) fn success_oracle() -> Value {serde_json::from_str(include_str!("../../../../vectors/messaging/forward-success.json")).unwrap()}
pub(crate) async fn install_success_fixture(app: &TestApp) {
    app.db().write(|tx| {
        let source = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: DAVID, markdown_source: Some("**Forward snapshot**".into()), client_message_id: Some("success-source".into()), drive_file_ids: vec!["abcdefghij".into()], ..Default::default()})?;
        let legacy = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: DAVID, body: Some("<div>Legacy &amp; <strong>safe</strong></div>".into()), client_message_id: Some("success-legacy".into()), ..Default::default()})?;
        let file = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: DAVID, body: Some("File snapshot".into()), client_message_id: Some("success-file".into()), attachment_blob_id: Some(13), ..Default::default()})?;
        let thread = ChannelThread::create(tx, NewChannelThread {room_id: ALL_TALK, creator_id: JASON, name: Some("Success closed".into()), ..Default::default()})?;
        let child = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: DAVID, thread_id: Some(thread.id), markdown_source: Some("Nested snapshot".into()), client_message_id: Some("success-child".into()), ..Default::default()})?;
        tx.conn().execute("DELETE FROM thread_memberships WHERE thread_id = ? AND user_id = ?", (thread.id, DAVID))?;
        tx.conn().execute("UPDATE channel_threads SET closed_at = ?, last_activity_at = ? WHERE id = ?", (tx.now(), tx.now().since(jiff::SignedDuration::from_hours(-2)), thread.id))?;
        for (key,id) in [("source_id",source.id),("legacy_id",legacy.id),("file_id",file.id),("thread_id",thread.id),("child_id",child.id)] {assert_eq!(success_oracle()[key],id);}
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn positive_forward_responses_and_snapshot_rows_match_rails_without_masks() {
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    install_success_fixture(&app).await;
    for row in success_oracle()["rows"].as_array().unwrap() {
        let ids = row["client_ids"].as_array().unwrap().iter().map(|id| id.as_str().unwrap()).collect::<Vec<_>>().join(",");
        let input = row["input"].to_string();
        let mut request = Req::new(Method::POST, row["path"].as_str().unwrap()).header("x-ws8bm-forward-client-ids", &ids).header("content-type", "application/json").body(input);
        if row["name"] == "nested_html" {request = request.header("accept", "text/html");}
        let response = app.david().write(request).await;
        let name = row["name"].as_str().unwrap();
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{name}: {}", response.text());
        assert_eq!(response.header("cache-control"), row["cache_control"].as_str(), "{name}");
        assert_eq!(response.location(), row["location"].as_str(), "{name}");
        assert_eq!(response.content_type(), row["content_type"].as_str(), "{name}");
        if response.text() != row["body"].as_str().unwrap() {rails_mismatch(&response.text(), row["body"].as_str().unwrap(), name);}
        let expected = row["messages"].clone();
        app.db().read(move |conn| {
            for row in expected.as_array().unwrap() {
                let m = Message::find(conn, row["id"].as_i64().unwrap())?;
                assert_eq!(json!({"id": m.id, "room_id": m.room_id, "thread_id": m.thread_id, "body": m.body_html(conn)?.unwrap_or_default(),
                    "forwarded_from_message_id": m.forwarded_from_message_id, "forwarded_markdown": m.forwarded_markdown, "forward_note": m.forward_note, "drive_ids": m.drive_file_ids(conn)?}), *row);
            }
            Ok(())
        }).await.unwrap();
    }
}

#[tokio::test]
async fn forward_picker_excludes_boards_names_directs_and_keeps_query_count_constant() {
    let app=fixture().await;
    let source=oracle()["source_id"].as_i64().unwrap();
    let path=format!("/messages/{source}/forwards/destinations.json");
    let mut viewer=app.david();viewer.authenticity_token().await;
    let capture=|app:&TestApp|app.db().capture_read_queries();
    let log=capture(&app);
    let response=viewer.get(&path).await;
    app.db().stop_capturing_read_queries();
    assert_eq!(response.status,StatusCode::OK);
    assert_eq!(response.header("cache-control"),Some("no-store"));
    let rows=response.json()["destinations"].as_array().unwrap().clone();
    let board=oracle()["board_id"].as_i64().unwrap();
    assert!(rows.iter().all(|row|row["room_id"]!=board),"board never offered");
    assert_eq!(rows.iter().find(|row|row["room_id"]==DIRECT_DAVID_JASON).unwrap()["name"],"Jason");
    let small=log.lock().unwrap().len();assert!(small>0);
    app.db().write(|tx| {
        for i in 0..6 {
            let room=Room::create_for(tx,RoomType::Closed,Some(&format!("Query destination {i}")),DAVID,&[DAVID,JASON])?;
            ChannelThread::create(tx,NewChannelThread {room_id:room.id,creator_id:DAVID,name:Some(format!("Destination thread {i}")),..Default::default()})?;
        }
        Ok(())
    }).await.unwrap();
    let log=capture(&app);let response=viewer.get(&path).await;app.db().stop_capturing_read_queries();
    assert_eq!(response.status,StatusCode::OK);
    assert_eq!(response.json()["destinations"].as_array().unwrap().len(),rows.len()+6);
    let large=log.lock().unwrap().len();assert_eq!(small,large,"forward picker reader queries grow: {small} to {large}");
}
