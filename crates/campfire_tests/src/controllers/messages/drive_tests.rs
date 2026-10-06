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
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{}/{}: {}", row["mode"], row["name"], response.text());
        assert_eq!(response.location(), row["location"].as_str());
        assert_eq!(response.content_type(), row["content_type"].as_str());
        if response.text() != row["body"].as_str().unwrap() { rails_mismatch(&response.text(), row["body"].as_str().unwrap(), &format!("Drive {}/{}", row["mode"], row["name"])); }
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
        if row["extra"].is_object() && row["extra"]["show"].is_string() {
            use askama::Template;
            use crate::controllers::presenters::{Presenter, page};
            let runtime = app.booted.app.clone();
            let id = row["saved"]["id"].as_i64().unwrap();
            let (show, edit) = app.db().read(move |conn| {
                let p = Presenter::new(conn, &runtime, None);
                let message = Message::find(conn, id)?;
                let view = p.message(&message)?;
                let account = campfire_db::Account::first(conn)?;
                page::render_detached_at(&runtime, account.as_ref(), "http://campfire.test", |ctx| {
                    Ok((campfire_views::messages::Show {ctx, message:&view}.render()?,
                        campfire_views::messages::Edit {ctx,edit:&campfire_views::messages::EditView {
                            editable_body_html:p.editable_markdown_source(&message).unwrap(), message:view.clone()}}.render()?))
                }).map_err(|e: askama::Error| campfire_db::Error::Other(e.to_string()))
            }).await.unwrap();
            assert_eq!(show, row["extra"]["show"].as_str().unwrap());
            assert_eq!(edit, row["extra"]["edit"].as_str().unwrap());
            let path = format!("/rooms/{ALL_TALK}/messages/{id}");
            let mut browser = app.david();
            assert!(browser.get(&path).await.text().contains(&show));
            let crypto = rails_compat::ar_encryption::ArEncryption::new(&app.booted.app.secrets);
            let token = crypto.encrypt("drive-fixture-token");
            app.db().write(move |tx| {
                tx.conn().execute("INSERT INTO google_accounts (user_id,email,access_token,scopes,created_at,updated_at) VALUES (?,'drive-fixture@example.test',?,'https://www.googleapis.com/auth/drive.file',?,?) ON CONFLICT(user_id) DO UPDATE SET scopes=excluded.scopes,access_token=excluded.access_token,disconnected_reason=NULL", (DAVID,token,tx.now(),tx.now()))?;
                Ok(())
            }).await.unwrap();
            assert!(browser.get(&path).await.text().contains(&show));
            let edit = browser.get(&format!("{path}/edit")).await;
            assert_eq!(edit.status, 200);
            assert_eq!(edit.text().matches("class=\"drive-attachment-chip\"").count(), 2);
            assert!(edit.text().split("<input ").skip(1).filter_map(|s| s.split('>').next())
                .any(|tag| tag.contains("name=\"message[drive_file_ids][]\"") && tag.contains("value=\"\"")));
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
#[tokio::test]
async fn persisted_drive_attachments_reach_the_message_http_response() {
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_profile_html.json"
    ))
    .unwrap();
    let row = oracle["drive"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| !r["urls"].as_array().unwrap().is_empty())
        .unwrap();
    let ids = vec![
        "1AbcDefGhIjKlMnOpQrSt".into(),
        "2BcDefGhIjKlMnOpQrStU".into(),
    ];
    let message = app
        .db()
        .write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("attached".into()),
                    client_message_id: Some("review239-drive".into()),
                    drive_file_ids: ids,
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let response = app
        .david()
        .get(&format!("/rooms/{ALL_TALK}/messages/{}", message.id))
        .await;
    assert_eq!(response.status, 200);
    let html = response.text();
    let expected = row["html"].as_str().unwrap().replace(
        "drive_attachments_message_0013",
        "drive_attachments_message_review239-drive",
    );
    assert!(
        html.contains(&expected),
        "persisted attachments missing from HTTP show: {html}"
    );
    assert_eq!(html.matches("class=\"drive-attachments\"").count(), 1);
    assert_eq!(html.matches("class=\"drive-attachment\"").count(), 2);
}
