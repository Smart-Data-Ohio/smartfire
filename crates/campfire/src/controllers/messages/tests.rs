//! Request-level tests for the message, boost and bot API controllers, against the `default`
//! parity seed.

use axum::http::{Method, StatusCode};
use campfire_db::{Boost, Message};

use crate::controllers::presenters::test_support::*;

const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 4, 0, 0, 0, 3, 8, 2, 0, 0, 0, 59, 150, 57, 145, 0, 0,
    0, 16, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 0, 71, 12, 56, 57, 0, 245, 49, 11, 245, 53, 123, 251, 130, 0, 0, 0, 0,
    73, 69, 78, 68, 174, 66, 96, 130,
];

const TURBO_STREAM_ACCEPT: &str = "text/vnd.turbo-stream.html, text/html, application/xhtml+xml";

fn ws11_oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors/agents_bot_contract.json"))).unwrap()
}

// WS11: security regressions through the real router, database and verifier.
#[tokio::test]
async fn ws11_reply_token_only_creates_messages() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    let token = rails_compat::verifiers::bot_reply::token_for(
        &app.booted.app.secrets, BENDER, ALL_TALK, app.booted.app.clock.now(),
    );
    let base = format!("/rooms/{ALL_TALK}/{token}/messages");
    let mut bot = app.anonymous();
    let created = bot.send(Req::new(Method::POST, &base).body("Reply token message")).await;
    assert_eq!(created.status, StatusCode::CREATED);
    let id: i64 = created.location().unwrap().rsplit('/').next().unwrap().parse().unwrap();
    let oracle = ws11_oracle();
    for (index, (method, path)) in [
        (Method::GET, base.clone()),
        (Method::PUT, format!("{base}/{id}")),
        (Method::DELETE, format!("{base}/{id}")),
        (Method::POST, format!("{base}/{id}/boosts")),
        (Method::DELETE, format!("{base}/{id}/boosts/0")),
    ].into_iter().enumerate() {
        let response = bot.send(Req::new(method.clone(), &path).body("Forbidden")).await;
        let expected = StatusCode::from_u16(oracle["reply_denials"][index]["status"].as_u64().unwrap() as u16).unwrap();
        assert_eq!(response.status, expected, "{method} {path}");
    }
    let saved = app.db().read(move |conn| Message::find(conn, id)?.body_html(conn)).await.unwrap();
    assert!(saved.unwrap().contains("Reply token message"));
}

#[tokio::test]
async fn ws11_bot_cannot_manage_system_notes() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    let note = app.db().write(|tx| Message::create(tx, campfire_db::NewMessage {
        room_id: ALL_TALK, creator_id: BENDER, body: Some("System note".into()),
        system_note: true, ..Default::default()
    })).await.unwrap();
    let mut bot = app.anonymous();
    let path = format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages/{}", note.id);
    for method in [Method::PUT, Method::DELETE] {
        assert_eq!(bot.send(Req::new(method, &path).body("Changed")).await.status, StatusCode::FORBIDDEN);
    }
    assert!(app.db().read(move |conn| Message::find(conn, note.id)).await.is_ok());
}

#[tokio::test]
async fn ws11_bot_pagination_counts_only_root_messages() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    let count = messages_in(&app, ALL_TALK).await.len();
    let reply = app.db().write(|tx| {
        let mut thread = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread {
            room_id: ALL_TALK, creator_id: DAVID, name: Some("Bot pagination".into()), ..Default::default()
        })?;
        thread.post_message(tx, DAVID, campfire_db::NewMessage { body: Some("Thread reply".into()), ..Default::default() })
    }).await.unwrap();
    let mut bot = app.anonymous();
    let base = format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages");
    let page = bot.get(&base).await;
    assert_eq!(page.status, StatusCode::OK);
    assert_eq!(page.header("x-total-count"), Some(count.to_string().as_str()));
    assert!(page.json().as_array().unwrap().iter().all(|m| m["id"] != reply.id));
    assert_eq!(bot.get(&format!("{base}?before={}", reply.id)).await.status, StatusCode::NOT_FOUND);
}

async fn messages_in(app: &TestApp, room_id: i64) -> Vec<Message> {
    let mut messages = app.db().read(move |conn| Message::for_room(conn, room_id)).await.unwrap();
    messages.sort_by_key(|m| (m.created_at, m.id));
    messages
}

#[tokio::test]
async fn index_pages_with_conditional_gets() {
    let Some(app) = TestApp::boot().await else { return };
    let messages = messages_in(&app, ALL_TALK).await;
    let mut david = app.david();

    let reply = david.get(&format!("/rooms/{ALL_TALK}/messages?before={}", messages[50].id)).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert!(reply.body.is_empty());
    let after_last = david.get(&format!("/rooms/{ALL_TALK}/messages?after={}", messages.last().unwrap().id)).await;
    assert_eq!(after_last.status, StatusCode::NO_CONTENT);
    assert_eq!(david.get(&format!("/rooms/{ALL_TALK}/messages?before=0")).await.status, StatusCode::NOT_FOUND);
    assert_eq!(david.get(&format!("/rooms/{DIRECT_KEVIN_BENDER}/messages")).await.status, StatusCode::NOT_FOUND);
    assert_eq!(david.get("/messages").await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_commits_the_message_without_a_turbo_response() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    let reply = david
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                .header("accept", TURBO_STREAM_ACCEPT)
                .form(&[("message[body]", "<p>Hello <strong>there</strong></p>"), ("message[client_message_id]", "abc-123")]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    assert!(reply.text().is_empty());

    let message = messages_in(&app, ALL_TALK).await.pop().unwrap();
    assert_eq!((message.client_message_id.as_str(), message.creator_id), ("abc-123", DAVID));
}

#[tokio::test]
async fn create_in_a_room_you_left_renders_room_not_found() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    let reply = david
        .write(Req::new(Method::POST, &format!("/rooms/{DIRECT_KEVIN_BENDER}/messages")).form(&[("message[body]", "hi")]))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body.is_empty());

    let missing = david.write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages")).form(&[("body", "hi")])).await;
    assert_eq!(missing.status, StatusCode::BAD_REQUEST);
    let html = david.write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages")).form(&[("message[body]", "hi")])).await;
    assert_eq!(html.status, StatusCode::CREATED);
    assert!(html.text().is_empty());
}

#[tokio::test]
async fn uploads_attach_and_process_the_file() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    let reply = david
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                .header("accept", "*/*")
                .multipart(&[("message[client_message_id]", "upload-1")], ("message[attachment]", "red.png", "image/png", PNG)),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    assert!(reply.text().is_empty());

    let message = messages_in(&app, ALL_TALK).await.pop().unwrap();
    assert_eq!(message.client_message_id, "upload-1");
    let (_, blob) = app.db().read(move |conn| message.attachment(conn)).await.unwrap().unwrap();
    assert_eq!(blob.filename, "red.png");
    let variants: i64 = app
        .db()
        .read(move |conn| Ok(conn.query_row("SELECT count(*) FROM active_storage_variant_records WHERE blob_id = ?", [blob.id], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(variants, 1, "the :thumb variant is processed");
}

#[tokio::test]
async fn show_edit_update_and_destroy() {
    let Some(app) = TestApp::boot().await else { return };
    let message = messages_in(&app, ALL_TALK).await.into_iter().rev().find(|m| m.creator_id == DAVID).unwrap();
    let mut david = app.david();
    let path = format!("/rooms/{ALL_TALK}/messages/{}", message.id);

    assert_eq!(david.get(&path).await.status, StatusCode::FOUND);
    let framed = david.send(Req::new(Method::GET, &path).header("turbo-frame", "message_x")).await;
    assert_eq!(framed.status, StatusCode::FOUND);
    assert!(framed.body.is_empty());
    assert_eq!(david.get(&format!("{path}/edit")).await.status, StatusCode::FOUND);

    let updated = david.write(Req::new(Method::PATCH, &path).form(&[("message[body]", "<p>Edited</p>")])).await;
    assert_eq!(updated.status, StatusCode::FOUND, "{}", updated.text());
    assert_eq!(updated.location(), Some(format!("http://campfire.test{path}").as_str()));
    let body = app.db().read(move |conn| Message::find(conn, message.id)?.body_html(conn)).await.unwrap();
    assert_eq!(body.as_deref(), Some("<p>Edited</p>"));

    let json = david.write(Req::new(Method::PATCH, &format!("{path}.json")).form(&[("message[body]", "x")])).await;
    assert_eq!(json.status, StatusCode::OK, "{}", json.text());
    assert_eq!(json.json()["id"], message.id);
    assert_eq!(json.json()["body"]["plain_text"], "x");
    assert_eq!(json.json()["creator"]["id"], DAVID);

    let destroyed = david.write(Req::new(Method::DELETE, &path).header("accept", TURBO_STREAM_ACCEPT)).await;
    assert_eq!(destroyed.status, StatusCode::NO_CONTENT);
    assert!(destroyed.text().is_empty());
    assert!(app.db().read(move |conn| Message::find_by_id(conn, message.id)).await.unwrap().is_none());
    assert_eq!(david.get(&format!("/api/v1/messages/{}", message.id)).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn boosts_are_listed_created_and_removed() {
    let Some(app) = TestApp::boot().await else { return };
    let message = messages_in(&app, ALL_TALK).await.pop().unwrap();
    let mut david = app.david();
    let path = format!("/messages/{}/boosts", message.id);
    assert_eq!(david.get(&path).await.status, StatusCode::FOUND);
    assert_eq!(david.get(&format!("{path}/new")).await.status, StatusCode::FOUND);
    assert_eq!(david.get(&format!("{path}/1")).await.status, StatusCode::NOT_FOUND);

    let created = david.write(Req::new(Method::POST, &path).form(&[("boost[content]", "🔥")])).await;
    assert_eq!(created.location(), Some(format!("http://campfire.test{path}").as_str()));
    let boost = app.db().read(move |conn| Boost::for_message(conn, message.id)).await.unwrap().pop().unwrap();
    assert_eq!((boost.content.as_str(), boost.booster_id), ("🔥", DAVID));

    let destroyed = david.write(Req::new(Method::DELETE, &format!("{path}/{}", boost.id))).await;
    assert_eq!(destroyed.status, StatusCode::NO_CONTENT);
    let again = david.write(Req::new(Method::DELETE, &format!("{path}/{}", boost.id))).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
    assert_eq!(david.get(&format!("/messages/{}/boosts", i64::MAX)).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_bot_api() {
    let Some(app) = TestApp::boot().await else { return };
    let mut bot = app.anonymous();
    let base = format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages");

    let index = bot.get(&base).await;
    assert_eq!(index.status, StatusCode::OK, "{}", index.text());
    assert_eq!(index.content_type(), Some("application/json; charset=utf-8"));
    assert_eq!(index.header("x-total-count"), Some("131"));
    let page = index.json();
    assert_eq!(page.as_array().unwrap().len(), 40);
    let first_id = page[0]["id"].as_i64().unwrap();
    assert_eq!(index.header("link"), Some(format!("<http://campfire.test{base}?before={first_id}>; rel=\"next\"").as_str()));
    let keys: Vec<&str> = page[0].as_object().unwrap().keys().map(String::as_str).collect();
    // Smartfire's MessagePayloadHelper adds the client ID, updated timestamp and Drive files.
    // bot_http_tests compares these fields and their values to the pinned Rails responses.
    assert_eq!(keys, ["id", "client_message_id", "created_at", "updated_at", "body", "creator", "room", "drive_attachments", "url"]);

    let created = bot.send(Req::new(Method::POST, &base).body("Beep boop")).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let message = messages_in(&app, ALL_TALK).await.pop().unwrap();
    assert_eq!(created.location(), Some(format!("http://campfire.test/messages/{}", message.id).as_str()));
    assert_eq!(message.creator_id, BENDER);

    assert_eq!(bot.send(Req::new(Method::POST, &base).body("  \n")).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    let upload = bot.send(Req::new(Method::POST, &base).multipart(&[], ("attachment", "red.png", "image/png", PNG))).await;
    assert_eq!(upload.status, StatusCode::CREATED);

    let updated = bot.send(Req::new(Method::PUT, &format!("{base}/{}", message.id)).body("Beep edited")).await;
    assert_eq!(updated.status, StatusCode::OK, "{}", updated.text());
    assert_eq!(updated.json()["body"]["plain_text"], "Beep edited");

    // An attachment replaces the message's attachment (and keeps the body), like Rails'
    // `update!(attachment:)`; a second one replaces the first; "" removes it.
    let attached = |app: &TestApp, id: i64| {
        let db = app.db().clone();
        async move { db.read(move |conn| Message::find(conn, id)?.attachment(conn)).await.unwrap().map(|(_, blob)| blob) }
    };
    let put_file = |name: &'static str| {
        Req::new(Method::PUT, &format!("{base}/{}", message.id)).multipart(&[], ("attachment", name, "image/png", PNG))
    };
    let with_file = bot.send(put_file("red.png")).await;
    assert_eq!(with_file.status, StatusCode::OK, "{}", with_file.text());
    assert_eq!(with_file.json()["body"]["plain_text"], "Beep edited");
    let first = attached(&app, message.id).await.expect("attached");
    assert_eq!(first.filename, "red.png");
    assert_eq!(bot.send(put_file("again.png")).await.status, StatusCode::OK);
    let second = attached(&app, message.id).await.expect("replaced");
    assert_eq!(second.filename, "again.png");
    assert_ne!(first.id, second.id);
    let removed = bot.send(Req::new(Method::PUT, &format!("{base}/{}", message.id)).form(&[("attachment", "")])).await;
    assert_eq!(removed.status, StatusCode::OK, "{}", removed.text());
    assert!(attached(&app, message.id).await.is_none());

    let boost = bot.send(Req::new(Method::POST, &format!("{base}/{}/boosts", message.id)).body("🤖")).await;
    assert_eq!(boost.status, StatusCode::CREATED, "{}", boost.text());
    assert_eq!(boost.json()["content"], "🤖");
    let boost_id = boost.json()["id"].as_i64().unwrap();
    let removed = bot.send(Req::new(Method::DELETE, &format!("{base}/{}/boosts/{boost_id}", message.id))).await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    let missing = bot.send(Req::new(Method::DELETE, &format!("{base}/{}/boosts/{boost_id}", message.id))).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);

    let destroyed = bot.send(Req::new(Method::DELETE, &format!("{base}/{}", message.id))).await;
    assert_eq!(destroyed.status, StatusCode::NO_CONTENT);

    // Rooms the bot isn't in, other people's messages, and bad keys.
    assert_eq!(bot.get(&format!("/rooms/{DIRECT_DAVID_JASON}/{BENDER_KEY}/messages")).await.status, StatusCode::NOT_FOUND);
    let not_mine = messages_in(&app, ALL_TALK).await.into_iter().find(|m| m.creator_id == DAVID).unwrap();
    let forbidden = bot.send(Req::new(Method::DELETE, &format!("{base}/{}", not_mine.id))).await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);
    let bad_key = bot.get(&format!("/rooms/{ALL_TALK}/1-nope/messages")).await;
    assert_eq!(bad_key.status, StatusCode::FOUND);
    // A bot key doesn't open the rest of the app.
    let denied = bot.get(&format!("/rooms/{ALL_TALK}/messages?bot_key={BENDER_KEY}")).await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn bot_unchanged_body_edit_refreshes_stale_pull_requests_once() {
    let app = TestApp::boot_with_test_clock(std::sync::Arc::new(campfire_kit::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    )))
    .await
    .expect("build the default parity seed")
    .without_job_runner()
    .await;
    let base = format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages");
    let body = "<p>https://github.com/rails/rails/pull/3141</p>";
    let mut bot = app.anonymous();
    let created = bot.send(Req::new(Method::POST, &base).body(body)).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let id: i64 = created.location().unwrap().rsplit('/').next().unwrap().parse().unwrap();
    let (pr, saved_body) = app.db().write(move |tx| {
        let message = Message::find(tx.conn(), id)?;
        let prs = crate::integrations::github::pull_requests::PullRequest::for_message(tx.conn(), id)?;
        assert_eq!(prs.len(), 1);
        let pr = prs[0].id;
        tx.conn().execute("DELETE FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'", [])?;
        tx.conn().execute("UPDATE github_pull_requests SET private=0,title='Stale cached PR',fetched_at='2000-01-01 00:00:00',fetch_requested_at=NULL WHERE id=?", [pr])?;
        Ok((pr, message.body_html(tx.conn())?.unwrap()))
    }).await.unwrap();

    for _ in 0..2 {
        let response = bot.send(Req::new(Method::PUT, &format!("{base}/{id}")).body(body)).await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        let (edited_at, jobs, requested_at) = app.db().read(move |conn| {
            let message = Message::find(conn, id)?;
            let jobs = conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob' AND json_extract(arguments,'$.pull_request_id')=?", [pr], |row| row.get::<_, i64>(0))?;
            let requested_at = crate::integrations::github::pull_requests::PullRequest::find(conn, pr)?.fetch_requested_at;
            Ok((message.edited_at, jobs, requested_at))
        }).await.unwrap();
        assert!(edited_at.is_none(), "identical body must skip reference synchronization");
        assert_eq!(jobs, 1, "bot edits must schedule a stale card refresh exactly once");
        assert!(requested_at.is_some());
    }
    assert_eq!(app.db().read(move |conn| Message::find(conn, id)?.body_html(conn)).await.unwrap().unwrap(), saved_body);
}

#[tokio::test]
async fn ws11_agent_credentials_are_authenticated_then_denied_on_human_endpoints() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    let oracle = ws11_oracle();
    let secret = "ws11-test-credential";
    let digest = campfire_db::user::digest_bot_token(secret);
    assert_eq!(digest, oracle["credential_digest"]);
    let credential = app.db().write(move |tx| {
        let agent: i64 = tx.conn().query_row("SELECT id FROM agents WHERE user_id=?",[BENDER],|r|r.get(0))?;
        tx.conn().execute("INSERT INTO agent_credentials(agent_id,created_by_id,name,token_digest,token_last_four,created_at,updated_at) VALUES(?,?,?,?,?,?,?)",
            rusqlite::params![agent,DAVID,"WS11",digest,&digest[..4],tx.now(),tx.now()])?;
        Ok(tx.conn().last_insert_rowid())
    }).await.unwrap();
    let header = format!("Bearer {secret}");
    let mut client = app.anonymous();
    for path in [format!("/rooms/{ALL_TALK}"), format!("/rooms/{ALL_TALK}/BOT_KEY/messages")] {
        let response = client.send(Req::new(Method::GET,&path).header("authorization",&header)).await;
        assert_eq!(response.status,StatusCode::FORBIDDEN,"{path}");
    }
    let stamps = app.db().read(move |conn| Ok(conn.query_row("SELECT last_used_at,last_used_ip FROM agent_credentials WHERE id=?",[credential],|r|Ok((r.get::<_,Option<String>>(0)?,r.get::<_,Option<String>>(1)?)))?)).await.unwrap();
    assert!(stamps.0.is_some());
    let second = client.send(Req::new(Method::GET,&format!("/rooms/{ALL_TALK}")).header("authorization",&header).header("x-forwarded-for","203.0.113.99")).await;
    assert_eq!(second.status,StatusCode::FORBIDDEN);
    let repeated = app.db().read(move |conn| Ok(conn.query_row("SELECT last_used_at,last_used_ip FROM agent_credentials WHERE id=?",[credential],|r|Ok((r.get::<_,Option<String>>(0)?,r.get::<_,Option<String>>(1)?)))?)).await.unwrap();
    assert_eq!(stamps,repeated,"a use inside one minute must not rewrite the activity stamp");
    for column in ["revoked_at", "expires_at"] {
        app.db().write(move |tx| {
            tx.conn().execute(&format!("UPDATE agent_credentials SET revoked_at=NULL, expires_at=NULL, {column}=? WHERE id=?"),rusqlite::params![tx.now().ago(jiff::SignedDuration::from_secs(1)),credential])?;
            Ok(())
        }).await.unwrap();
        assert_eq!(client.send(Req::new(Method::GET,&format!("/rooms/{ALL_TALK}")).header("authorization",&header)).await.status,StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn ws11_bot_grants_are_checked_on_every_request_after_membership() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    let mut client = app.anonymous();
    let path = format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages");
    assert_eq!(client.get(&path).await.status,StatusCode::OK,"legacy capabilities before any grant");
    let grant = app.db().write(|tx| {
        let agent: i64 = tx.conn().query_row("SELECT id FROM agents WHERE user_id=?",[BENDER],|r|r.get(0))?;
        tx.conn().execute("INSERT INTO agent_grants(agent_id,granted_by_id,capability,created_at,updated_at,revoked_at) VALUES(?,?,'read_messages',?,?,?)",
            rusqlite::params![agent,DAVID,tx.now(),tx.now(),tx.now()])?;
        Ok(tx.conn().last_insert_rowid())
    }).await.unwrap();
    let oracle = ws11_oracle();
    let denied = client.get(&path).await;
    assert_eq!(denied.status,StatusCode::FORBIDDEN);
    assert_eq!(denied.json(),oracle["grants"][1]["body"]);
    assert_eq!(client.get(&format!("/rooms/{DIRECT_DAVID_JASON}/{BENDER_KEY}/messages")).await.status,StatusCode::NOT_FOUND);
    app.db().write(move |tx| {tx.conn().execute("UPDATE agent_grants SET revoked_at=NULL WHERE id=?",[grant])?;Ok(())}).await.unwrap();
    assert_eq!(client.get(&path).await.status,StatusCode::OK,"workspace-wide grant");
    let create = client.send(Req::new(Method::POST,&path).body("Missing posting grant")).await;
    assert_eq!(create.status,StatusCode::FORBIDDEN);
    assert_eq!(create.json(),oracle["grants"][3]["body"]);
    app.db().write(move |tx| {tx.conn().execute("UPDATE agent_grants SET room_id=? WHERE id=?",[DIRECT_KEVIN_BENDER,grant])?;Ok(())}).await.unwrap();
    assert_eq!(client.get(&path).await.status,StatusCode::FORBIDDEN,"grant for another room");
    app.db().write(move |tx| {tx.conn().execute("UPDATE agent_grants SET room_id=? WHERE id=?",[ALL_TALK,grant])?;Ok(())}).await.unwrap();
    assert_eq!(client.get(&path).await.status,StatusCode::OK,"room-scoped grant");
    app.db().write(move |tx| {tx.conn().execute("UPDATE agent_grants SET revoked_at=? WHERE id=?",rusqlite::params![tx.now(),grant])?;Ok(())}).await.unwrap();
    assert_eq!(client.get(&path).await.status,StatusCode::FORBIDDEN,"revocation takes effect next request");
}

#[tokio::test]
async fn ws11_reply_token_unknown_message_is_forbidden() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    let token = rails_compat::verifiers::bot_reply::token_for(&app.booted.app.secrets,BENDER,ALL_TALK,app.booted.app.clock.now());
    let mut client = app.anonymous();
    let oracle=ws11_oracle();
    for (index,method) in [Method::PUT,Method::DELETE].into_iter().enumerate() {
        let response=client.send(Req::new(method.clone(),&format!("/rooms/{ALL_TALK}/{token}/messages/0")).body("Missing")).await;
        assert_eq!(response.status.as_u16(),oracle["missing_reply_messages"][index]["status"].as_u64().unwrap() as u16,"{method}");
    }
}

#[tokio::test]
async fn ws11_agent_backed_bots_never_receive_legacy_webhook_jobs() {
    let app = TestApp::boot().await.expect("build the default parity seed").without_job_runner().await;
    app.db().write(|tx| {
        tx.conn().execute("UPDATE rooms SET type='Rooms::Direct' WHERE id=?", [ALL_TALK])?;
        let message = Message::create(tx, campfire_db::NewMessage { room_id: ALL_TALK, creator_id: DAVID, body: Some("DM".into()), ..Default::default() })?;
        let room = campfire_db::Room::find(tx.conn(), ALL_TALK)?;
        super::deliver_webhooks_to_bots(tx, &room, &message)
    }).await.unwrap();
    let count: i64 = app.db().read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Bot::WebhookJob' AND json_extract(arguments, '$.bot_id')=?", [BENDER], |row| row.get(0))?)).await.unwrap();
    assert_eq!(count, 0, "agent-backed bot bypassed agent delivery");
}

#[tokio::test]
async fn ws11_webhook_payloads_match_rails_bytes() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    let vectors: serde_json::Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors/agents_webhook_contract.json"))).unwrap();
    let rich_text = app.db().env().rich_text.clone();
    let actual = app.db().read(move |conn| {
        let webhook = campfire_db::Webhook::find_by_user(conn, BENDER)?.unwrap();
        let message = Message::find(conn, 136976342)?;
        let agent_id: i64 = conn.query_row("SELECT id FROM agents WHERE user_id=?", [BENDER], |row| row.get(0))?;
        Ok((webhook.payload(conn, &*rich_text, &message, &campfire_routes::room(ALL_TALK), &campfire_routes::room_at_message(ALL_TALK, message.id))?,
            webhook.payload_for_agent(conn, &*rich_text, &message, agent_id, 123, &serde_json::Value::Null)?))
    }).await.unwrap();
    assert_eq!(actual.0, vectors["payloads"]["agent_backed_legacy_delivery"]);
    assert_eq!(actual.1, vectors["payloads"]["agent_delivery"]);
    let secrets = app.booted.app.secrets.clone();
    let now = vectors["now"].as_str().unwrap().parse().unwrap();
    let rich_text = app.db().env().rich_text.clone();
    let legacy = app.db().write(move |tx| {
        tx.conn().execute("DELETE FROM agents WHERE user_id=?", [BENDER])?;
        let webhook = campfire_db::Webhook::find_by_user(tx.conn(), BENDER)?.unwrap();
        let message = Message::find(tx.conn(), 136976342)?;
        let token = rails_compat::verifiers::bot_reply::token_for(&secrets, BENDER, ALL_TALK, now);
        webhook.payload(tx.conn(), &*rich_text, &message, &campfire_routes::room_bot_messages(ALL_TALK, &token), &campfire_routes::room_at_message(ALL_TALK, message.id))
    }).await.unwrap();
    assert_eq!(legacy, vectors["payloads"]["legacy_delivery"]);
    assert!(!legacy.contains(BENDER_KEY));
}

#[tokio::test]
async fn ws11_webhook_secrets_reload_encrypt_and_rotate() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    let vectors: serde_json::Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors/agents_webhook_contract.json"))).unwrap();
    let encryption = app.booted.app.ar_encryption.clone();
    let raw = vectors["secret"]["ciphertext"].as_str().unwrap().to_owned();
    let expected = vectors["secret"]["plaintext"].as_str().unwrap().to_owned();
    assert_eq!(encryption.decrypt(&raw).unwrap(), expected);
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE webhooks SET signing_secret=? WHERE user_id=?", rusqlite::params![raw, BENDER])?;
        Ok(())
    }).await.unwrap();
    let (mut winner, mut stale) = app.db().read(|conn| {
        let webhook = campfire_db::Webhook::find_by_user(conn, BENDER)?.unwrap();
        Ok((webhook.clone(), webhook))
    }).await.unwrap();
    let encryption = app.booted.app.ar_encryption.clone();
    let export = app.db().write(move |tx| {
        assert_eq!(winner.signing_secret(&encryption)?.as_deref(), Some(expected.as_str()));
        let secret = winner.reset_signing_secret(tx, &encryption)?;
        assert_ne!(secret, expected);
        // Rails returns an already-loaded present secret without reloading it.
        assert_eq!(stale.ensure_signing_secret(tx, &encryption)?, expected);
        tx.conn().execute("UPDATE webhooks SET signing_secret=NULL WHERE user_id=?", [BENDER])?;
        winner = campfire_db::Webhook::find_by_user(tx.conn(), BENDER)?.unwrap();
        stale = winner.clone();
        let secret = winner.ensure_signing_secret(tx, &encryption)?;
        assert_eq!(stale.ensure_signing_secret(tx, &encryption)?, secret);
        let first_ciphertext = stale.encrypted_signing_secret.clone();
        assert_eq!(stale.ensure_signing_secret(tx, &encryption)?, secret);
        assert_eq!(stale.encrypted_signing_secret, first_ciphertext);
        assert_eq!(secret.len(), 64);
        assert!(secret.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
        let ciphertext = stale.encrypted_signing_secret.unwrap();
        assert!(!ciphertext.contains(&secret));
        assert_eq!(encryption.decrypt_bytes(&ciphertext).unwrap().encoding, "US-ASCII");
        assert!(rails_compat::ar_encryption::ArEncryption::from_key(&[7; 32]).decrypt(&ciphertext).is_err());
        let agent_id: i64 = tx.conn().query_row("SELECT id FROM agents WHERE user_id=?", [BENDER], |row| row.get(0))?;
        let agent_secret = campfire_db::models::agent_access::ensure_webhook_signing_secret(tx, &encryption, agent_id)?;
        assert_eq!(campfire_db::models::agent_access::ensure_webhook_signing_secret(tx, &encryption, agent_id)?, agent_secret);
        let agent_ciphertext: String = tx.conn().query_row("SELECT webhook_signing_secret FROM agents WHERE id=?", [agent_id], |row| row.get(0))?;
        assert_eq!(encryption.decrypt_bytes(&agent_ciphertext).unwrap().encoding, "US-ASCII");
        Ok(serde_json::json!({"webhook":{"plaintext":secret,"ciphertext":ciphertext},"agent":{"plaintext":agent_secret,"ciphertext":agent_ciphertext}}))
    }).await.unwrap();
    if let Ok(path) = std::env::var("WS11_SECRET_EXPORT") {
        std::fs::write(path, serde_json::to_string_pretty(&export).unwrap()).unwrap();
    }
}

#[tokio::test]
async fn ws11_bot_key_message_budget_overflow_notifies_once() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    app.db().write(|tx| {
        tx.conn().execute("UPDATE agents SET daily_message_cap=1 WHERE user_id=?", [BENDER])?;
        Message::create(tx, campfire_db::NewMessage { room_id: ALL_TALK, creator_id: BENDER, body: Some("Spent".into()), ..Default::default() })?;
        Ok(())
    }).await.unwrap();
    let mut bot = app.anonymous();
    for attempt in 0..2 {
        let response = bot.send(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages")).body("Over budget")).await;
        assert_eq!(response.status, StatusCode::TOO_MANY_REQUESTS);
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["error"], "Daily message budget exceeded (1/day)");
        assert_eq!(body["cap"], "messages");
        assert_eq!(body["limit"], 1);
        assert!(body["retry_after"].as_i64().unwrap() > 0);
        assert!(response.headers.get("retry-after").is_none());
        if attempt == 0 {
            app.db().write(|tx| {
                tx.conn().execute("UPDATE activity_items SET read_at=? WHERE source_type='AgentBudgetNotice'", [tx.now()])?;
                Ok(())
            }).await.unwrap();
        }
    }
    app.db().read(|conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM agent_budget_notices WHERE cap='messages'", [], |r| r.get::<_, i64>(0))?, 1);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentBudgetNotice' AND event_type='agent_budget_exceeded' AND user_id=?", [DAVID], |r| r.get::<_, i64>(0))?, 1);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentBudgetNotice' AND read_at IS NOT NULL", [], |r| r.get::<_, i64>(0))?, 1, "repeated overflow made the notice unread again");
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn ws11_bot_key_idempotency_replay_precedes_budget() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    let message = app.db().write(|tx| {
        tx.conn().execute("UPDATE agents SET daily_message_cap=1 WHERE user_id=?", [BENDER])?;
        Message::create(tx, campfire_db::NewMessage { room_id: ALL_TALK, creator_id: BENDER, client_message_id: Some("ws11-retry".into()), body: Some("Original".into()), ..Default::default() })
    }).await.unwrap();
    let count: i64 = app.db().read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?)).await.unwrap();
    let mut bot = app.anonymous();
    let body = r#"{"message":{"client_message_id":"ws11-retry","body":"Replacement"}}"#;
    let response = bot.send(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages")).header("content-type", "application/json").body(body)).await;
    assert_eq!(response.status, StatusCode::CREATED);
    assert_eq!(response.location().unwrap().rsplit('/').next().unwrap(), message.id.to_string());
    app.db().read(move |conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?, count);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM agent_budget_notices", [], |r| r.get::<_, i64>(0))?, 0);
        assert_eq!(Message::find(conn, message.id)?.body_html(conn)?.as_deref(), Some("Original"));
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn ws11_legacy_bot_keeps_raw_body_and_ignores_client_message_id() {
    let app = TestApp::boot().await.expect("build the default parity seed");
    app.db().write(|tx| { tx.conn().execute("DELETE FROM agents WHERE user_id=?", [BENDER])?; Ok(()) }).await.unwrap();
    let mut bot = app.anonymous();
    let mut ids = Vec::new();
    for _ in 0..2 {
        let response = bot.send(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages")).header("content-type", "application/json").body(r#"{"message":{"client_message_id":"ws11-legacy","body":"JSON is raw text"}}"#)).await;
        assert_eq!(response.status, StatusCode::CREATED);
        ids.push(response.location().unwrap().rsplit('/').next().unwrap().parse::<i64>().unwrap());
    }
    assert_ne!(ids[0], ids[1]);
    app.db().read(move |conn| {
        for id in ids {
            let message = Message::find(conn, id)?;
            assert_ne!(message.client_message_id, "ws11-legacy");
            assert_eq!(message.markdown_source, None);
            assert!(message.body_html(conn)?.unwrap().contains("JSON is raw text"));
        }
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn ws11_replay_and_budget_precede_attachment_validation() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors/agents_posting_budget_contract.json"))).unwrap();
    let app = TestApp::boot().await.expect("build the default parity seed");
    let original = app.db().write(|tx| {
        tx.conn().execute("UPDATE agents SET daily_message_cap=1 WHERE user_id=?", [BENDER])?;
        Message::create(tx, campfire_db::NewMessage { room_id: ALL_TALK, creator_id: BENDER, client_message_id: Some("ws11-bad-attachment".into()), body: Some("Original".into()), ..Default::default() })
    }).await.unwrap();
    let mut bot = app.anonymous();
    let path = format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages");
    let replay = bot.send(Req::new(Method::POST, &path).header("content-type", "application/json").body(r#"{"attachment":123,"message":{"client_message_id":"ws11-bad-attachment"}}"#)).await;
    assert_eq!(replay.status.as_u16(), vectors["malformed_replay"]["status"].as_u64().unwrap() as u16);
    assert_eq!(replay.location().unwrap().rsplit('/').next().unwrap(), original.id.to_string());
    let overflow = bot.send(Req::new(Method::POST, &path).header("content-type", "application/json").body(r#"{"attachment":123,"message":{"client_message_id":"ws11-new-bad-attachment"}}"#)).await;
    assert_eq!(overflow.status.as_u16(), vectors["malformed_overflow"]["status"].as_u64().unwrap() as u16);
}

#[tokio::test]
async fn ws11_agent_delivery_enqueue_failure_rolls_the_http_message_back() {
    let app=TestApp::boot().await.expect("default seed");
    app.db().write(|tx| {
        tx.conn().execute("UPDATE rooms SET type='Rooms::Direct' WHERE id=?", [ALL_TALK])?;
        tx.conn().execute_batch("CREATE TRIGGER reject_agent_delivery BEFORE INSERT ON background_jobs WHEN NEW.job_class='Agent::DeliveryJob' BEGIN SELECT RAISE(ABORT,'WS11 queue rejected'); END;")?;Ok(())
    }).await.unwrap();
    let before=messages_in(&app,ALL_TALK).await.len();
    let mut david=app.david();
    let response=david.write(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/messages")).header("accept",TURBO_STREAM_ACCEPT).form(&[("message[body]","Atomic delivery")])).await;
    assert_eq!(response.status,StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(messages_in(&app,ALL_TALK).await.len(),before);
    app.db().read(|c| {
        let n:i64=c.query_row("SELECT COUNT(*) FROM agent_events WHERE event_type='direct_message'",[],|r|r.get(0))?;
        assert_eq!(n,0);Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn ws11_bot_raw_edit_clears_existing_markdown_source() {
    let app=TestApp::boot().await.expect("default seed");
    let message=app.db().write(|tx|Message::create(tx,campfire_db::NewMessage {room_id:ALL_TALK,creator_id:BENDER,markdown_source:Some("**Old**".into()),..Default::default()})).await.unwrap();
    let mut client=app.anonymous();
    let response=client.send(Req::new(Method::PUT,&format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages/{}",message.id)).body("Fresh raw body")).await;
    assert_eq!(response.status,StatusCode::OK);
    let id=message.id;
    let actual=app.db().read(move |c|Message::find(c,id)).await.unwrap();
    assert!(actual.markdown_source.is_none(),"Bot raw edits explicitly assign markdown_source: nil");

}

#[tokio::test]
async fn ws11_bot_boost_content_matches_rails_shortcodes_and_strip() {
    let vectors:serde_json::Value=serde_json::from_str(include_str!("../../../../../vectors/agents_bot_gaps_contract.json")).unwrap();
    let app=TestApp::boot().await.expect("default seed");
    let mut client=app.anonymous();
    for case in vectors["boosts"].as_array().unwrap() {
        let response=client.send(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages/136976342/boosts")).body(case["input"].as_str().unwrap())).await;
        assert_eq!(response.status.as_u16(),case["status"].as_u64().unwrap() as u16);
        if response.status==StatusCode::CREATED {
            let content=app.db().read(|c|Ok(c.query_row("SELECT content FROM boosts WHERE message_id=136976342 ORDER BY id DESC LIMIT 1",[],|r|r.get::<_,String>(0))?)).await.unwrap();
            assert_eq!(content,case["stored"].as_str().unwrap());
        }
    }
}

#[tokio::test]
async fn ws11_bot_client_id_coercion_matches_rails_lookup_and_storage() {
    let vectors:serde_json::Value=serde_json::from_str(include_str!("../../../../../vectors/agents_bot_gaps_contract.json")).unwrap();
    let app=TestApp::boot().await.expect("default seed");let mut client=app.anonymous();
    for case in vectors["client_ids"].as_array().unwrap() {
        let mut locations=vec![];
        for status in case["statuses"].as_array().unwrap() {
            let raw=serde_json::json!({"message":{"client_message_id":case["input"]}}).to_string();
            let response=client.send(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages")).header("content-type","application/json").body(raw)).await;
            assert_eq!(response.status.as_u16(),status.as_u64().unwrap() as u16,"input={}",case["input"]);
            locations.push(response.location().map(str::to_string));
        }
        if let Some(expected)=case["replayed"].as_bool() {assert_eq!(locations[0]==locations[1],expected,"input={}",case["input"]);}
        if let Some(id)=locations[1].as_deref().and_then(|s|s.rsplit('/').next()).and_then(|s|s.parse::<i64>().ok()) {
            let actual=app.db().read(move |c|Message::find(c,id)).await.unwrap();
            assert_eq!(actual.client_message_id,case["stored"].as_str().unwrap());
        }
    }
}

/// Registers a workspace (custom, non-built-in) icon so `Icons.find` resolves it.
async fn custom_icon_message(app: &TestApp, body: &str) -> i64 {
    let body = body.to_string();
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "INSERT INTO workspace_icons (name,title,creator_id,created_at,updated_at) VALUES ('acme_brand','Acme',?,?,?)",
                (DAVID, tx.now(), tx.now()),
            )?;
            let message = Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some(body),
                    client_message_id: Some("custom-icon-emoji".into()),
                    ..Default::default()
                },
            )?;
            Ok(message.id)
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn boost_of_only_a_custom_icon_shortcode_is_emoji_only_like_rails() {
    let app = TestApp::boot().await.unwrap();
    let id = custom_icon_message(&app, "Boost target").await;
    app.db()
        .write(move |tx| {
            Boost::create(tx, id, DAVID, ":acme_brand:")?;
            Boost::create(tx, id, JASON, ":acme_unknown:")?;
            Ok(())
        })
        .await
        .unwrap();
    let runtime = app.booted.app.clone();
    let boosts = app
        .db()
        .read(move |conn| {
            let p = crate::controllers::presenters::Presenter::new(conn, &runtime, None);
            Ok(p.boosts(&Message::find(conn, id)?)?.into_iter().map(|b| (b.content, b.all_emoji)).collect::<Vec<_>>())
        })
        .await
        .unwrap();
    assert!(boosts.contains(&(":acme_brand:".to_string(), true)), "{boosts:?}");
    assert!(boosts.contains(&(":acme_unknown:".to_string(), false)), "{boosts:?}");
}
