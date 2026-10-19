use super::agent_http_tests::{AGENT, SECRET, initialize};
use super::presenters::test_support::{ALL_TALK, BENDER, DAVID, Req, TestApp};
use axum::{
    body::Body,
    http::{Request, Response},
};
use campfire_db::{Message, NewMessage, Room};
use http_body_util::BodyExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tower::ServiceExt;

async fn setup() -> TestApp {
    setup_with(&[]).await
}

async fn setup_with(extra: &[(&str, &str)]) -> TestApp {
    let mut settings = vec![
        ("AGENT_EVENT_STREAM_RECHECK_MS", "20"),
        ("AGENT_EVENT_STREAM_KEEPALIVE_MS", "40"),
        ("AGENT_EVENT_STREAM_LIFETIME_MS", "2000"),
    ];
    settings.extend_from_slice(extra);
    let app = TestApp::boot_frozen_with_env(&settings)
        .await
        .unwrap()
        .without_job_runner()
        .await;
    initialize(&app).await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM agent_events WHERE agent_id=?", [AGENT])?;
            Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[BENDER])?;
            Ok(())
        })
        .await
        .unwrap();
    app
}

async fn mention(app: &TestApp) -> i64 {
    app.db().write(|tx| {
        let message = Message::create(tx, NewMessage {
            room_id: ALL_TALK, creator_id: DAVID,
            markdown_source: Some("Hey @[Bender Bot]".into()), ..Default::default()
        })?;
        Ok(tx.conn().query_row("SELECT id FROM agent_events WHERE agent_id=? AND message_id=? AND event_type='mention'", rusqlite::params![AGENT, message.id], |r| r.get(0))?)
    }).await.unwrap()
}

async fn open(app: &TestApp, query: &str, last: Option<i64>, secret: &str) -> Response<Body> {
    let mut request = Request::builder()
        .uri(format!("/agents/events/stream{query}"))
        .header("host", "campfire.test")
        .header("accept", "text/event-stream")
        .header("authorization", format!("Bearer {secret}"));
    if let Some(id) = last {
        request = request.header("last-event-id", id.to_string());
    }
    app.booted
        .router
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn stream(app: &TestApp, query: &str, last: Option<i64>) -> Body {
    let response = open(app, query, last, SECRET).await;
    assert_eq!(
        response.status(),
        200,
        "SSE endpoint must accept the polling credential"
    );
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    assert_eq!(response.headers()["cache-control"], "no-cache");
    assert_eq!(response.headers()["x-accel-buffering"], "no");
    response.into_body()
}

async fn frame(body: &mut Body) -> Option<String> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let frame = body.frame().await?.unwrap();
            if let Ok(bytes) = frame.into_data() {
                return Some(String::from_utf8(bytes.to_vec()).unwrap());
            }
        }
    })
    .await
    .expect("stream must produce a frame or close")
}

async fn event(body: &mut Body) -> Value {
    loop {
        let text = frame(body).await.expect("event before stream closes");
        if let Some(data) = text.lines().find_map(|line| line.strip_prefix("data: ")) {
            let data: Value = serde_json::from_str(data).unwrap();
            assert!(text.contains(&format!("id: {}\n", data["id"])));
            assert!(text.contains(&format!(
                "event: {}\n",
                data["event_type"].as_str().unwrap()
            )));
            return data;
        }
    }
}

#[tokio::test]
async fn agent_sse_new_mention_arrives_live_with_polling_payload_and_ack() {
    let app = setup().await;
    let mut body = stream(&app, "", None).await;
    assert!(frame(&mut body).await.unwrap().contains("retry: "));
    let id = mention(&app).await;
    let polling = app
        .anonymous()
        .send(
            Req::new(campfire_kit::Method::GET, "/agents/events")
                .header("authorization", &format!("Bearer {SECRET}")),
        )
        .await;
    let data = event(&mut body).await;
    assert_eq!(data, polling.json()[0]);
    assert_eq!(data["id"], id);
    let outcome = app
        .db()
        .read(move |c| {
            Ok(
                c.query_row("SELECT outcome FROM agent_events WHERE id=?", [id], |r| {
                    r.get::<_, String>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(
        outcome, "pending",
        "stream reads leave delivery jobs in charge of delivery state"
    );
    let ack = app
        .anonymous()
        .send(
            Req::new(
                campfire_kit::Method::POST,
                &format!("/agents/events/{id}/ack"),
            )
            .header("authorization", &format!("Bearer {SECRET}")),
        )
        .await;
    assert_eq!(ack.status.as_u16(), 200);
    assert_eq!(ack.json()["outcome"], "acknowledged");
}

#[tokio::test]
async fn agent_sse_last_event_id_wins_and_only_later_rows_replay() {
    let app = setup().await;
    let first = mention(&app).await;
    let second = mention(&app).await;
    let third = mention(&app).await;
    let mut body = stream(&app, "?since=0&limit=1", Some(first)).await;
    assert_eq!(event(&mut body).await["id"], second);
    assert_eq!(event(&mut body).await["id"], third);
    assert!(frame(&mut body).await.unwrap().starts_with(": ping"));
    let mut resumed = stream(&app, &format!("?since={second}"), None).await;
    assert_eq!(event(&mut resumed).await["id"], third);
}

#[tokio::test]
async fn agent_sse_insert_during_paged_replay_has_no_gap_or_duplicate() {
    let app = setup().await;
    let first = mention(&app).await;
    let second = mention(&app).await;
    let mut body = stream(&app, "?limit=1", None).await;
    assert_eq!(event(&mut body).await["id"], first);
    let inserted = mention(&app).await;
    assert_eq!(event(&mut body).await["id"], second);
    assert_eq!(event(&mut body).await["id"], inserted);
    let live = mention(&app).await;
    assert_eq!(event(&mut body).await["id"], live);
    assert!(frame(&mut body).await.unwrap().starts_with(": ping"));
}

#[tokio::test]
async fn agent_sse_live_authorization_hides_unreadable_events() {
    let app = setup().await;
    let hidden = mention(&app).await;
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agent_events SET message_id=NULL WHERE id=?",
                [hidden],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut body = stream(&app, "?limit=1", None).await;
    let visible = mention(&app).await;
    assert_eq!(event(&mut body).await["id"], visible);
    let forbidden = mention(&app).await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                rusqlite::params![ALL_TALK, BENDER],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    while let Some(text) = frame(&mut body).await {
        assert!(!text.contains(&format!("id: {forbidden}\n")));
    }
}

#[tokio::test]
async fn agent_sse_cap_is_per_credential_and_disconnect_releases_slot() {
    let app = setup().await;
    let one = stream(&app, "", None).await;
    let two = stream(&app, "", None).await;
    let three = stream(&app, "", None).await;
    let excess = open(&app, "", None, SECRET).await;
    assert_eq!(excess.status(), 429);
    assert!(excess.headers().contains_key("retry-after"));
    let other_secret = "another-agent-stream-credential";
    app.db()
        .write(move |tx| {
            let digest = format!("{:x}", Sha256::digest(other_secret));
            campfire_db::AgentCredential::create(
                tx,
                campfire_db::NewCredential {
                    agent_id: AGENT,
                    created_by_id: DAVID,
                    name: "Second stream credential".into(),
                    token_last_four: digest[..4].into(),
                    token_digest: digest,
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let independent = open(&app, "", None, other_secret).await;
    assert_eq!(independent.status(), 200);
    drop(one);
    let replacement = stream(&app, "", None).await;
    drop((two, three, replacement));
}

#[tokio::test]
async fn agent_sse_deactivated_agent_closes_stream() {
    let app = setup().await;
    let mut body = stream(&app, "", None).await;
    frame(&mut body).await.unwrap();
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agents SET suspended_at=? WHERE id=?",
                rusqlite::params![tx.now(), AGENT],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(frame(&mut body).await, None);
}

#[tokio::test]
async fn agent_sse_revoked_credential_closes_stream() {
    let app = setup().await;
    let mut body = stream(&app, "", None).await;
    frame(&mut body).await.unwrap();
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE agent_credentials SET revoked_at=? WHERE agent_id=?",
                rusqlite::params![tx.now(), AGENT],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(frame(&mut body).await, None);
}

#[tokio::test]
async fn agent_sse_keepalive_and_clean_lifetime_close() {
    let app = setup().await;
    let mut body = stream(&app, "", None).await;
    assert!(frame(&mut body).await.unwrap().contains("retry: "));
    assert_eq!(frame(&mut body).await.as_deref(), Some(": ping\n\n"));
    while let Some(text) = frame(&mut body).await {
        assert_eq!(text, ": ping\n\n");
    }
}

#[tokio::test]
async fn agent_sse_front_does_not_buffer_or_compress_and_stream_ends_before_write_timeout() {
    use campfire_kit::front::{ConnInfo, FrontConfig, Handler};
    let app = setup_with(&[("HTTP_WRITE_TIMEOUT", "1")]).await;
    let config =
        FrontConfig::from_lookup(|name| (name == "HTTP_WRITE_TIMEOUT").then(|| "1".into()));
    let handler = std::sync::Arc::new(Handler::new(&config, app.booted.router.clone()));
    let request = Request::builder()
        .uri("/agents/events/stream")
        .header("host", "campfire.test")
        .header("accept", "text/event-stream")
        .header("accept-encoding", "gzip")
        .header("authorization", format!("Bearer {SECRET}"))
        .body(Body::empty())
        .unwrap();
    let started = tokio::time::Instant::now();
    let response = tokio::time::timeout(
        Duration::from_millis(500),
        handler.call(
            request,
            ConnInfo {
                remote: "127.0.0.1:1234".parse().unwrap(),
                tls: false,
            },
        ),
    )
    .await
    .expect("front must flush SSE headers without waiting for its compression buffer");
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-encoding"], "identity");
    assert!(!response.headers().contains_key("etag"));
    let mut body = response.into_body();
    assert!(frame(&mut body).await.unwrap().contains("retry: "));
    while frame(&mut body).await.is_some() {}
    assert!(started.elapsed() < Duration::from_secs(1));
}
