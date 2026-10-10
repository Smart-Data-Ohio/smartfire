use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, ThreadMembership, Tx};
use campfire_kit::clock::FrozenClock;
use serde_json::{Value, json};
use std::sync::Arc;
use crate::controllers::presenters::test_support::*;

async fn scope_fixture() -> (TestApp, i64) {
    let app = TestApp::boot().await.unwrap();
    let id = app.db().write(seed).await.unwrap();
    (app, id)
}

pub(crate) fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/thread-lifecycle.json")).unwrap() }

pub(crate) fn seed(tx: &mut Tx<'_>) -> campfire_db::Result<i64> {
        tx.conn().execute("UPDATE users SET role = 0 WHERE id = ?", [JASON])?;
        let parent = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID,
            markdown_source: Some("Lifecycle parent".into()), client_message_id: Some("lifecycle-parent".into()), ..Default::default() })?;
        let thread = ChannelThread::create(tx, NewChannelThread { room_id: ALL_TALK, creator_id: JASON,
            parent_message_id: Some(parent.id), name: Some("Lifecycle".into()), ..Default::default() })?;
        ThreadMembership::join(tx, thread.id, JASON)?;
        assert_eq!(parent.id, oracle()["parent_id"]);
        assert_eq!(thread.id, oracle()["thread_id"]);
        Ok(thread.id)
}

pub(crate) fn prepare(tx: &mut Tx<'_>, name: &str) -> campfire_db::Result<()> {
    let id = oracle()["thread_id"].as_i64().unwrap();
    let stale = tx.now().since(jiff::SignedDuration::from_hours(-2));
    if name == "stale_reopen" { tx.conn().execute("UPDATE channel_threads SET last_activity_at = ?, auto_archive_after_minutes = 60 WHERE id = ?", (stale, id))?; }
    if name == "stale_close" { tx.conn().execute("UPDATE channel_threads SET last_activity_at = ? WHERE id = ?", (stale, id))?; }
    Ok(())
}

pub(crate) fn request(row: &Value) -> Req {
    Req::new(Method::from_bytes(row["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(), row["path"].as_str().unwrap())
        .header("content-type", "application/json").body(row["input"].to_string())
}

#[tokio::test]
async fn lifecycle_actions_match_rails_responses_and_atomic_rows() {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.unwrap();
    let first = app.db().write(seed).await.unwrap();
    let mut david = app.david();
    let mut creator = app.sign_in(JASON).await;
    for row in oracle()["rows"].as_array().unwrap().iter().filter(|row| !row["method"].as_str().unwrap().eq_ignore_ascii_case("get")) {
        let name = row["name"].as_str().unwrap().to_owned();
        app.db().write(move |tx| prepare(tx, &name)).await.unwrap();
        clock.set(row["time"].as_str().unwrap().parse().unwrap());
        let browser = if row["viewer"] == 0 { &mut david } else { &mut creator };
        let response = browser.write(request(row)).await;
        let name = row["name"].as_str().unwrap();
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{name}: {}", response.text());
        if row["html"] != true {
            assert_eq!(response.content_type(), row["content_type"].as_str(), "{name}");
        }
        assert_eq!(response.header("cache-control"), row["cache_control"].as_str(), "{name}");
        assert_eq!(response.location(), row["location"].as_str(), "{name}");
        let expected = row["body"].as_str().unwrap();
        if row["html"] == true {
            assert!(response.text().is_empty(), "{name}");
        } else if response.text() != expected { rails_mismatch(&response.text(), expected, name); }
        let state = app.db().read(move |conn| {
            let mut records = ChannelThread::for_room(conn, ALL_TALK)?;
            records.sort_by_key(|thread| thread.id);
            let threads = records.iter().filter(|thread| thread.id >= first).map(|thread| {
                let mut members = ThreadMembership::for_thread(conn, thread.id)?.into_iter().map(|member| member.user_id).collect::<Vec<_>>();
                members.sort();
                let messages = Message::in_thread(conn, thread.id)?.iter().map(|message| Ok(json!({
                    "id": message.id, "client_message_id": message.client_message_id, "markdown_source": message.markdown_source, "body": message.body_html(conn)?,
                }))).collect::<campfire_db::Result<Vec<_>>>()?;
                Ok(json!({"id": thread.id, "name": thread.name, "creator_id": thread.creator_id, "parent_message_id": thread.parent_message_id,
                    "auto_archive_after_minutes": thread.auto_archive_after_minutes, "closed_at": thread.closed_at.map(|time| campfire_presentation::messages::support::json_time(time.jiff())),
                    "locked_at": thread.locked_at.map(|time| campfire_presentation::messages::support::json_time(time.jiff())),
                    "last_activity_at": campfire_presentation::messages::support::json_time(thread.last_activity_at.jiff()),
                    "updated_at": campfire_presentation::messages::support::json_time(thread.updated_at.jiff()), "tags": thread.tag_names(conn)?, "members": members, "messages": messages}))
            }).collect::<campfire_db::Result<Vec<_>>>()?;
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM channel_threads", [], |row| row.get(0))?;
            Ok(json!({"threads": threads, "thread_count": count}))
        }).await.unwrap();
        for key in ["threads", "thread_count"] { assert_eq!(state[key], row[key], "{name}/{key}"); }
    }
}

#[tokio::test]
async fn initial_post_enqueue_failure_rolls_back_thread_membership_and_message() {
    let app = TestApp::boot().await.unwrap();
    let before = app.db().read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM channel_threads", [], |row| row.get::<_, i64>(0))?)).await.unwrap();
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws8bm_reject_initial_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'WS8bm initial enqueue rejection'); END;")?;
        Ok(())
    }).await.unwrap();
    let response = app.david().write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/threads.json"))
        .header("content-type", "application/json").body(json!({"thread": {"name": "Rollback", "message": {"markdown_source": "Rejected", "client_message_id": "rollback-initial"}}}).to_string())).await;
    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    app.db().read(move |conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM channel_threads", [], |row| row.get::<_, i64>(0))?, before);
        assert!(Message::find_duplicate(conn, ALL_TALK, DAVID, "rollback-initial")?.is_none());
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn lifecycle_routes_scope_before_writes_and_reject_forged_or_bot_requests() {
    let (app, id) = scope_fixture().await;
    let mut david = app.david();
    for (method, path) in [(Method::POST, format!("/rooms/{QUIET_CORNER}/threads.json")),
        (Method::PATCH, format!("/rooms/{QUIET_CORNER}/threads/{id}.json")),
        (Method::DELETE, format!("/rooms/{QUIET_CORNER}/threads/{id}.json"))] {
        let response = david.write(Req::new(method, &path).form(&[("thread[parent_message_id]", "0")] )).await;
        assert_eq!(response.status, StatusCode::NOT_FOUND, "{path}");
    }
    let path = format!("/rooms/{ALL_TALK}/threads/{id}.json");
    assert_eq!(david.send(Req::new(Method::DELETE, &path).header("origin", "https://forged.test")).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.anonymous().write(Req::new(Method::DELETE, &format!("{path}?bot_key={BENDER_KEY}"))).await.status, StatusCode::FORBIDDEN);
    let mut kevin = app.sign_in(KEVIN).await;
    assert_eq!(kevin.write(Req::new(Method::DELETE, &path)).await.status, StatusCode::NOT_FOUND);
    app.db().write(|tx| { tx.conn().execute("UPDATE rooms SET deleted_at = ? WHERE id = ?", (tx.now(), ALL_TALK))?; Ok(()) }).await.unwrap();
    assert_eq!(david.write(Req::new(Method::DELETE, &path)).await.status, StatusCode::NOT_FOUND);
    assert!(app.db().read(move |conn| ChannelThread::find_by_id(conn, id)).await.unwrap().is_some());
}

#[tokio::test]
async fn thread_creator_cannot_moderate_or_delete_even_when_joined() {
    let (app, id) = scope_fixture().await;
    app.db().write(|tx| { tx.conn().execute("UPDATE users SET role = 0 WHERE id = ?", [JASON])?; Ok(()) }).await.unwrap();
    let mut creator = app.sign_in(JASON).await;
    let path = format!("/rooms/{ALL_TALK}/threads/{id}.json");
    assert_eq!(creator.write(Req::new(Method::PATCH, &path).form(&[("thread[status]", "locked")])).await.status, StatusCode::FORBIDDEN);
    assert_eq!(creator.write(Req::new(Method::DELETE, &path)).await.status, StatusCode::FORBIDDEN);
    let thread = app.db().read(move |conn| ChannelThread::find(conn, id)).await.unwrap();
    assert!(thread.locked_at.is_none());
}

#[tokio::test]
async fn initial_thread_upload_is_processed_and_downloadable() {
    let app = TestApp::boot().await.unwrap();
    let response = app.david().write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/threads.json"))
        .multipart(&[("thread[name]", "Uploaded thread"), ("message[client_message_id]", "thread-upload")],
            ("message[attachment]", "thread.txt", "text/plain", b"Thread upload\n"))).await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
    let (thread, blob) = app.db().read(|conn| {
        let message = Message::find_duplicate(conn, ALL_TALK, DAVID, "thread-upload")?.unwrap();
        Ok((message.thread_id.unwrap(), campfire_storage::Blob::attached(conn, "Message", message.id, "attachment").map_err(crate::controllers::presenters::storage_error)?.unwrap()))
    }).await.unwrap();
    assert!(blob.is_analyzed());
    assert_eq!(app.booted.app.storage.service.download(&blob.key).unwrap(), b"Thread upload\n");
    assert_eq!(response.json()["thread"]["id"], thread);
}

/// test/controllers/channel_threads_controller_test.rb:331, "work owner must be an eligible
/// parent-room member and a revoked owner stays visible as unavailable", on the Rails fixtures
/// with the declaration's setup: jz's "Design discussion" thread in Designers.
#[tokio::test]
async fn work_owner_must_be_an_eligible_parent_room_member_and_a_revoked_owner_stays_visible_as_unavailable() {
    const DESIGNERS: i64 = 654632876;
    const JZ: i64 = 773523953;
    let app = TestApp::boot_frozen().await.expect("default seed required").without_job_runner().await;
    let thread = app.db().write(|tx| {
        tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
        let tables = tx.conn().prepare("SELECT name FROM pragma_table_list WHERE schema='main' AND type='table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('schema_migrations','ar_internal_metadata')")?
            .query_map([], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for table in tables {
            tx.conn().execute(&format!("DELETE FROM \"{}\"", table.replace('"', "\"\"")), [])?;
        }
        campfire_db::fixtures::load(tx.conn(), &campfire_db::fixtures::reference_dir(), &campfire_db::fixtures::Options { now: tx.now(), bcrypt_cost: 4 })?;
        let thread = ChannelThread::create(tx, NewChannelThread {
            room_id: DESIGNERS, creator_id: JZ, name: Some("Design discussion".into()), ..Default::default()
        })?;
        ThreadMembership::join(tx, thread.id, JZ)?;
        Ok(thread.id)
    }).await.unwrap();
    let path = format!("/rooms/{DESIGNERS}/threads/{thread}.json");
    let patch = |input: Value| Req::new(Method::PATCH, &path).header("content-type", "application/json").body(json!({"thread": input}).to_string());
    let owner = || async { app.db().read(move |c| ChannelThread::find(c, thread)).await.unwrap().work_owner_id };
    let events = || async {
        app.db().read(|c| Ok(c.query_row("SELECT COUNT(*) FROM work_thread_events", [], |r| r.get::<_, i64>(0))?)).await.unwrap()
    };
    let mut jz = app.sign_in(JZ).await;

    let assigned = jz.write(patch(json!({"work_status": "planned", "work_owner_id": KEVIN}))).await;
    assert_eq!(assigned.status, StatusCode::OK, "{}", assigned.text());

    let before = events().await;
    let refused = jz.write(patch(json!({"work_owner_id": BENDER}))).await;
    assert_eq!(events().await, before, "a refused owner records no work event");
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", refused.text());
    assert!(refused.json()["error"].as_str().unwrap().contains("active agent member"), "{}", refused.text());
    assert_eq!(owner().await, Some(KEVIN));

    // users(:kevin).deactivate
    app.db().write(|tx| campfire_db::User::find(tx.conn(), KEVIN)?.deactivate(tx)).await.unwrap();

    let shown = jz.get(&path).await;
    assert_eq!(shown.status, StatusCode::OK, "{}", shown.text());
    // assert_not @thread.reload.work_owner_active?, as the thread JSON reports it.
    assert_eq!(shown.json()["thread"]["work_owner_active"], json!(false), "{}", shown.text());
    assert_eq!(shown.json()["thread"]["work_owner_id"], json!(KEVIN), "{}", shown.text());
    assert_eq!(shown.json()["thread"]["work_owner"]["active"], json!(false), "{}", shown.text());
    assert_eq!(shown.json()["thread"]["work_owner"]["name"], json!("Kevin"), "{}", shown.text());

    let cleared = jz.write(patch(json!({"work_owner_id": ""}))).await;
    assert_eq!(cleared.status, StatusCode::OK, "{}", cleared.text());
    assert_eq!(owner().await, None);
}
