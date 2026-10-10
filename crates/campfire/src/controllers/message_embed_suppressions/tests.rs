use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Message, NewMessage};

async fn app() -> TestApp {
    let mut app = TestApp::boot().await.expect("build parity seed");
    app.booted.jobs.stop(std::time::Duration::from_secs(1)).await;
    app
}

async fn message(app: &TestApp) -> Message {
    app.db()
        .write(|tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    body: Some("<p>read https://example.com/page</p>".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn ws15e_suppression_requires_membership_author_and_eligible_conversation() {
    let app = app().await;
    let message = message(&app).await;
    let path = format!("/rooms/{ALL_TALK}/messages/{}/embed_suppression.json", message.id);
    assert_eq!(app.anonymous().send(Req::new(Method::POST, &path)).await.status, StatusCode::FOUND);
    assert_eq!(
        app.sign_in(JASON).await.write(Req::new(Method::POST, &path)).await.status,
        StatusCode::FORBIDDEN
    );
    let wrong = format!("/rooms/{DIRECT_KEVIN_BENDER}/messages/{}/embed_suppression.json", message.id);
    let others=app.db().write(|tx| Message::create(tx,NewMessage { room_id:ALL_TALK,creator_id:JASON,body:Some("other author's message".into()),..Default::default() })).await.unwrap();
    assert_eq!(app.david().write(Req::new(Method::POST,&format!("/messages/{}/embed_suppression.json",others.id))).await.status,StatusCode::FORBIDDEN,"administrator has no author override");
    assert_eq!(
        app.david().write(Req::new(Method::POST, &wrong)).await.status,
        StatusCode::NOT_FOUND
    );
    let note = app
        .db()
        .write(|tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    system_note: true,
                    body: Some("note".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let path = format!("/messages/{}/embed_suppression.json", note.id);
    assert_eq!(app.david().write(Req::new(Method::POST, &path)).await.status, StatusCode::FORBIDDEN);
    assert!(
        !app.db()
            .read(move |conn| Message::find(conn, message.id))
            .await
            .unwrap()
            .embeds_suppressed
    );
}

#[tokio::test]
async fn ws15e_http_link_jobs_and_references_roll_back_with_rejected_enqueue() {
    let app = app().await;
    app.db().write(|tx| Ok(tx.conn().execute_batch("CREATE TRIGGER ws15e_reject_link_jobs BEFORE INSERT ON background_jobs WHEN NEW.job_class='LinkEmbed::FetchJob' BEGIN SELECT RAISE(ABORT,'link jobs rejected'); END")?)).await.unwrap();
    let req = Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
        .header("accept", "text/vnd.turbo-stream.html")
        .form(&[
            ("message[markdown_source]", "see https://example.com/new"),
            ("message[client_message_id]", "ws15e-atomic"),
        ]);
    let response = app.david().write(req).await;
    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR, "{}", response.text());
    app.db()
        .read(|conn| {
            for sql in [
                "SELECT count(*) FROM messages WHERE client_message_id='ws15e-atomic'",
                "SELECT count(*) FROM link_embeds WHERE normalized_url='https://example.com/new'",
                "SELECT count(*) FROM background_jobs WHERE job_class='LinkEmbed::FetchJob'",
            ] {
                assert_eq!(conn.query_row(sql, [], |row| row.get::<_, i64>(0))?, 0);
            }
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn ws15e_suppression_retains_references_is_idempotent_and_clears_both_targets() {
    let app = app().await;
    let message = app
        .db()
        .write(|tx| {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    client_message_id: Some("ws15e-suppressed".into()),
                    body: Some("<p>https://example.com/a</p>".into()),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE messages SET markdown_source='https://example.com/a' WHERE id=?",
                [message.id],
            )?;
            let message = Message::find(tx.conn(), message.id)?;
            crate::integrations::link_embed::sync_message(tx, &message, false)?;
            Ok(message)
        })
        .await
        .unwrap();
    let path = format!("/messages/{}/embed_suppression", message.id);
    let mut browser = app.david();
    let response = browser.write(Req::new(Method::POST, &format!("{path}.json"))).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.text(), "{\"embeds_suppressed\":true}");
    assert!(response.headers["cache-control"].to_str().unwrap().contains("no-store"));
    let id = message.id;
    let first = app.db().read(move |conn| Message::find(conn, id)).await.unwrap();
    assert!(first.embeds_suppressed);
    let response = browser
        .write(Req::new(Method::POST, &format!("{path}.json")))
        .await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.text(), "{\"embeds_suppressed\":true}");
    let second = app
        .db()
        .read(move |conn| {
            let message = Message::find(conn, id)?;
            assert_eq!(crate::integrations::link_embed::Reference::for_message(conn, &message)?.len(), 1);
            Ok(message)
        })
        .await
        .unwrap();
    assert_eq!(first.updated_at, second.updated_at, "repeat suppression doesn't touch");
    let response = browser
        .write(Req::new(Method::POST, &path).header("accept", "text/vnd.turbo-stream.html"))
        .await;
    assert_eq!(response.status, StatusCode::NOT_ACCEPTABLE);
    assert!(response.body.is_empty());
    let response = browser.get(&format!("/api/v1/rooms/{ALL_TALK}/messages")).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(
        app.db()
            .read(campfire_jobs::inspect::all)
            .await
            .unwrap()
            .iter()
            .all(|job| job.class != "LinkEmbed::FetchJob"),
        "suppressed render never requests a fetch"
    );
    let response = browser.write(Req::new(Method::POST, &path)).await;
    assert_eq!(response.status, StatusCode::FOUND);
    assert!(
        response.headers["location"]
            .to_str()
            .unwrap()
            .ends_with(&campfire_routes::room_at_message(ALL_TALK, id))
    );
}

#[tokio::test]
async fn ws15e_suppression_checks_locked_threads_and_root_scoping() {
    let app = app().await;
    let message=app.db().write(|tx| {
        let thread:i64=tx.conn().query_row("INSERT INTO channel_threads (room_id,creator_id,name,last_activity_at,created_at,updated_at,locked_at) VALUES (?1,?2,'locked',?3,?3,?3,?3) RETURNING id",rusqlite::params![ALL_TALK,DAVID,tx.now()],|row| row.get(0))?;
        let message=Message::create(tx,NewMessage { room_id:ALL_TALK,creator_id:DAVID,body:Some("<p>thread</p>".into()),..Default::default() })?;
        tx.conn().execute("UPDATE messages SET thread_id=?1 WHERE id=?2",rusqlite::params![thread,message.id])?;
        Message::find(tx.conn(),message.id)
    }).await.unwrap();
    let mut browser = app.david();
    let root = format!("/rooms/{ALL_TALK}/messages/{}/embed_suppression.json", message.id);
    assert_eq!(browser.write(Req::new(Method::POST, &root)).await.status, StatusCode::NOT_FOUND);
    let thread = format!(
        "/rooms/{ALL_TALK}/threads/{}/messages/{}/embed_suppression.json",
        message.thread_id.unwrap(),
        message.id
    );
    assert_eq!(browser.write(Req::new(Method::POST, &thread)).await.status, StatusCode::FORBIDDEN);
    assert_eq!(
        browser
            .write(Req::new(Method::POST, &format!("/messages/{}/embed_suppression.json", message.id)))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let thread_id=message.thread_id.unwrap();
    app.db().write(move |tx| { tx.conn().execute("UPDATE channel_threads SET locked_at=NULL WHERE id=?",[thread_id])?; Ok(()) }).await.unwrap();
    assert_eq!(browser.write(Req::new(Method::POST,&thread)).await.status,StatusCode::OK,"unlocked author can suppress through the thread route");
}
