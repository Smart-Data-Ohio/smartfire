//! #226: exercise real rendering, durable scheduling and the registered media worker.
use crate::controllers::presenters::test_support::*;
use campfire_db::{Message, NewMessage, Timestamp};
use campfire_jobs::{JobQueue, QueueConfig, RunnerConfig};
use campfire_kit::FrozenClock;
use campfire_storage::Blob;
use serde_json::json;
use std::{sync::Arc, time::Duration};

const CLASS: &str = "Message::AttachmentProcessingJob";

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/message_attachment_processing.json"
    ))
    .unwrap()
}

async fn render(app: &TestApp, id: i64) -> serde_json::Value {
    let response = app.david().send(Req::new(axum::http::Method::GET, &format!("/api/v1/messages/{id}")).header("accept", "application/json")).await;
    assert_eq!(response.status, axum::http::StatusCode::OK, "{}", response.text());
    response.json()["message"].clone()
}



pub(crate) async fn perform_queued(app: &TestApp, attempts: u32) -> campfire_jobs::JobResult {
    let (id, arguments, now) = app.db().read(|c| Ok(c.query_row("SELECT id,arguments,created_at FROM background_jobs WHERE job_class=? AND status='ready' ORDER BY id DESC LIMIT 1", [CLASS], |r| Ok((r.get::<_, i64>(0)?, r.get::<_,String>(1)?, r.get::<_,Timestamp>(2)?)))?)).await.unwrap();
    let result = crate::jobs::attachment_processing::perform(
        app.booted.app.clone(),
        serde_json::from_str(&arguments).unwrap(),
        campfire_jobs::Execution {
            id,
            executions: attempts,
            enqueued_at: now,
            scheduled_at: now,
        },
    )
    .await;
    if result.is_ok() {
        app.db()
            .write(move |tx| {
                tx.conn()
                    .execute("DELETE FROM background_jobs WHERE id=?", [id])?;
                Ok(())
            })
            .await
            .unwrap();
    }
    result
}

pub(crate) async fn setup(corrupt: bool) -> (TestApp, Arc<FrozenClock>, i64, i64) {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_clock(clock.clone())
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let storage = app.booted.app.storage.clone();
    let (id, blob_id) = app
        .db()
        .write(move |tx| {
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            let source = Blob::find(tx.conn(), 9).unwrap().unwrap();
            let staged = if corrupt {
                storage
                    .stage_bytes(
                        b"corrupt MOV",
                        source.filename,
                        source.content_type.as_deref(),
                    )
                    .unwrap()
            } else {
                storage
                    .stage_file(
                        &storage.service.path_for(&source.key),
                        source.filename,
                        source.content_type.as_deref(),
                    )
                    .unwrap()
            };
            let blob = staged.insert(tx.conn(), tx.now().jiff()).unwrap();
            crate::active_storage::keep_after_commit(tx, staged);
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    client_message_id: Some("attachment-processing-1".into()),
                    attachment_blob_id: Some(blob.id),
                    ..Default::default()
                },
            )?;
            Ok((message.id, blob.id))
        })
        .await
        .unwrap();
    (app, clock, id, blob_id)
}

async fn view(app:&TestApp,id:i64) { let message=render(app,id).await; assert_eq!(message["id"],id); }


async fn state(app: &TestApp, blob: i64) -> (Option<String>, Option<Timestamp>, i64) {
    app.db().read(move |c| {
        let (token, expires) = c.query_row("SELECT message_processing_token,message_processing_expires_at FROM active_storage_blobs WHERE id=?", [blob], |r| Ok((r.get(0)?, r.get(1)?)))?;
        let count = c.query_row("SELECT count(*) FROM background_jobs WHERE job_class=? AND COALESCE(json_extract(arguments,'$.blob_id'),json_extract(arguments,'$._campfire_retry_metadata_v1.arguments.blob_id'))=?", rusqlite::params![CLASS, blob], |r| r.get(0))?;
        Ok((token, expires, count))
    }).await.unwrap()
}

/// Aborts the cable server if an assertion fails before the test reaches an explicit abort.
/// Dropping the `JoinHandle` only detaches `axum::serve`, so the accept loop keeps the
/// process alive and the failure looks like a hang.
struct AbortServer(tokio::task::JoinHandle<()>);

impl Drop for AbortServer {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(crate) async fn subscribe(
    app: &TestApp,
) -> (
    crate::channels::tests::support::Client,
    tokio::task::JoinHandle<()>,
) {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let listener = crate::integrations::test_support::ws15e_listener().await;
    let address = listener.local_addr().unwrap();
    let router = app.booted.app.cable.router::<()>("/cable");
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{address}").parse().unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    request
        .headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    let mut client = crate::channels::tests::support::Client {
        socket: tokio_tungstenite::connect_async(request).await.unwrap().0,
    };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let identifier = crate::channels::tests::support::identifier(json!({"channel":"RoomChannel","room_id":ALL_TALK}));
    client.confirm(&identifier).await;
    (client, serving)
}

pub(crate) async fn json_subscribe(app: &TestApp) -> (crate::controllers::spa::api_tests::Sync, tokio::task::JoinHandle<()>) {
    campfire_api::install(&app.booted.app);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = app.booted.app.cable.sync_router::<()>(campfire_api::SYNC_PATH);
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let browser = app.sign_in(DAVID).await;
    let mut sync = crate::controllers::spa::api_tests::Sync::connect(addr, &browser.cookie_header(), &[format!("room:{ALL_TALK}")]).await;
    sync.welcome().await;
    (sync, serving)
}

#[tokio::test]
async fn attachment_processing_concurrent_views_claim_only_one_job() {
    let (app, _, id, blob) = setup(false).await;
    futures_util::future::join_all((0..8).map(|_| view(&app, id))).await;
    let (token, expires, count) = state(&app, blob).await;
    assert_eq!(count, 1);
    assert!(token.unwrap().ends_with(":0"));
    assert_eq!(
        expires.unwrap().jiff(),
        SEED_NOW
            .parse::<jiff::Timestamp>()
            .unwrap()
            .checked_add(jiff::SignedDuration::from_secs(900))
            .unwrap()
    );
}

#[tokio::test]
async fn attachment_processing_expired_lease_recovers_but_active_does_not() {
    let (app, clock, id, blob) = setup(false).await;
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE active_storage_blobs SET message_processing_token='lost:0',message_processing_expires_at=? WHERE id=?", rusqlite::params![tx.now().since(jiff::SignedDuration::from_secs(900)), blob])?;
        Ok(())
    }).await.unwrap();
    view(&app, id).await;
    assert_eq!(state(&app, blob).await.2, 0);
    clock.advance(jiff::SignedDuration::from_secs(900));
    view(&app, id).await;
    let (token, _, count) = state(&app, blob).await;
    assert_eq!(count, 1);
    assert_ne!(token.as_deref(), Some("lost:0"));
}

async fn seed_job(app: &TestApp, id: i64, blob: i64) {
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE active_storage_blobs SET message_processing_token='worker:0',message_processing_expires_at=? WHERE id=?", rusqlite::params![tx.now().since(jiff::SignedDuration::from_secs(900)), blob])?;
        tx.conn().execute("INSERT INTO background_jobs(queue_name,job_class,arguments,payload_version,status,attempts,run_at,created_at,updated_at) VALUES('default',?, ?,1,'ready',0,?,?,?)", rusqlite::params![CLASS, json!({"message_id":id,"blob_id":blob,"token":"worker:0"}).to_string(), tx.now(),tx.now(),tx.now()])?;
        Ok(())
    }).await.unwrap();
}

fn start(app: &TestApp) -> campfire_jobs::Runner {
    let registry = crate::jobs::registry();
    let mut config = RunnerConfig::new(
        ["default", "push", "webhooks", "slack_import"]
            .map(|name| QueueConfig::new(name, 1))
            .to_vec(),
    );
    config.poll = Duration::from_millis(10);
    config.recovery = Duration::from_millis(10);
    let queue = JobQueue::new(&registry, &config).unwrap();
    campfire_jobs::start(
        app.db().clone(),
        queue,
        registry,
        app.booted.app.clone(),
        config,
    )
}

#[tokio::test]
async fn attachment_processing_completion_render_failure_retries_like_rails() {
    use rusqlite::OptionalExtension;
    let (app, clock, id, blob) = setup(false).await;
    let (mut client, server) = json_subscribe(&app).await;
    seed_job(&app, id, blob).await;
    // Media and touch can succeed, but the detached completion presenter cannot render.
    app.db().write(|tx| {
        tx.conn().execute_batch("ALTER TABLE boosts RENAME TO unavailable_boosts")?;
        Ok(())
    }).await.unwrap();
    app.publications().take();
    let runner = start(&app);
    let retry = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let row = app.db().read(|c| Ok(c.query_row(
                "SELECT status,attempts,run_at FROM background_jobs WHERE job_class=?",
                [CLASS], |r| Ok((r.get::<_,String>(0)?, r.get::<_,i64>(1)?, r.get::<_,Timestamp>(2)?)),
            ).optional()?)).await.unwrap();
            if row.as_ref().is_none_or(|r| r.0 == "ready" && r.1 > 0) { break row; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    runner.shutdown(Duration::from_secs(1)).await;
    let storage = app.booted.app.storage.clone();
    let preview_attached = app.db().read(move |c| {
        let source = Blob::find(c, blob).unwrap().unwrap();
        Ok(storage.existing_preview_image(c, &source).unwrap().is_some())
    }).await.unwrap();
    let (token, expires, count) = state(&app, blob).await;
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/message_attachment_processing_failures.json"
    )).unwrap();
    assert!(app.publications().take().is_empty());
    assert_eq!(json!({
        "preview_attached": preview_attached,
        "retry_jobs": count,
        "retry_executions": retry.as_ref().map(|r| vec![r.1]).unwrap_or_default(),
        "lease": {"message_processing_token":token,"message_processing_expires_at":expires.map(|t| t.jiff().to_string())},
    }), expected["completion_failure"], "a committed preview does not acknowledge a failed completion broadcast");

    app.db().write(|tx| {
        tx.conn().execute_batch("ALTER TABLE unavailable_boosts RENAME TO boosts")?;
        Ok(())
    }).await.unwrap();
    clock.set(retry.unwrap().2.jiff());
    let runner = start(&app);
    tokio::time::timeout(Duration::from_secs(15), async {
        while state(&app, blob).await.2 != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    runner.shutdown(Duration::from_secs(1)).await;
    client.until(|event| matches!(&event.payload, campfire_api_types::SyncPayload::MessageUpdated(message) if message.id == id && message.attachment.as_ref().is_some_and(|file| file.thumbnail_url.is_some())), |_| false).await;
    assert!(render(&app,id).await["attachment"]["thumbnailUrl"].is_string());
    server.abort();
}

#[tokio::test]
async fn attachment_processing_duplicate_composer_response_keeps_recovery_requests() {
    let (app, _, _, blob) = setup(true).await;
    // Duplicate submission skips broadcast_create: the response itself must drain intents,
    // including intents recorded by a warm PR-card collection cache key.
    for _ in 0..2 {
        let response = app.david().write(Req::new(axum::http::Method::POST,
            &format!("/rooms/{ALL_TALK}/messages"))
            .header("accept", "text/vnd.turbo-stream.html")
            .header("content-type", "application/json")
            .body(json!({"message":{"client_message_id":"attachment-processing-1"}}).to_string())
        ).await;
        assert_eq!(response.status.as_u16(), 201, "{}", response.text());
        assert_eq!(state(&app, blob).await.2, 1);
    }
}

#[tokio::test]
async fn attachment_processing_quiet_stream_final_keeps_recovery_requests() {
    let (app, _, id, blob) = setup(true).await;
    app.db().write(move |tx| {
        tx.emit_after_commit(campfire_db::Event::broadcast(
            &campfire_db::models::user::lifecycle::QuietStreamFinal { message_id: id }
        ));
        Ok(())
    }).await.unwrap();
    assert_eq!(state(&app, blob).await.2, 1);
}

fn event_oracle() -> serde_json::Value {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/message_attachment_processing_events.json"
    )).unwrap();
    for row in value.as_object_mut().unwrap().values_mut() { if let Some(row) = row.as_object_mut() { row.remove("video_frames"); row.remove("poster_present"); } }
    value
}

async fn event_snapshot(app: &TestApp, blob: i64) -> serde_json::Value {
    let (token, expires, count) = state(app, blob).await;
    let preview: bool = app.db().read(move |c| Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM active_storage_attachments WHERE record_type='ActiveStorage::Blob' AND name='preview_image' AND record_id=?)",
        [blob], |r| r.get(0),
    )?)).await.unwrap();
    json!({
        "processing_jobs":count,"preview_attached":preview,
        "token_suffix":token.and_then(|t| t.rsplit(':').next().map(str::to_owned)),
        "expires_in":expires.map(|t| t.jiff().duration_since(SEED_NOW.parse().unwrap()).as_secs()),
    })
}

async fn agent_step_video() -> (TestApp, i64, i64) {
    let (app, _, id, blob) = setup(true).await;
    crate::controllers::agent_http_tests::initialize(&app).await;
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE messages SET creator_id=(SELECT user_id FROM agents WHERE id=773018776),client_message_id='agent-step-recovery' WHERE id=?", [id])?;
        Ok(())
    }).await.unwrap();
    assert_eq!(state(&app, blob).await, (None, None, 0));
    (app, id, blob)
}

#[tokio::test]
async fn attachment_processing_step_create_retains_recovery_like_fresh_rails() {
    let (app, id, blob) = agent_step_video().await;
    let (_client, server) = subscribe(&app).await;
    app.publications().take();
    let reply = app.anonymous().send(Req::new(axum::http::Method::POST, "/agents/steps")
        .header("authorization", &format!("Bearer {}", crate::controllers::agent_http_tests::SECRET))
        .header("accept", "application/json").header("content-type", "application/json")
        .body(json!({"message_id":id,"name":"Inspect video"}).to_string())).await;
    let mut actual = event_snapshot(&app, blob).await;
    actual["http_status"] = json!(reply.status.as_u16());
    server.abort();
    assert_eq!(actual, event_oracle()["step_create"], "{}", reply.text());
}

#[tokio::test]
async fn attachment_processing_step_update_retains_recovery_like_fresh_rails() {
    let (app, id, blob) = agent_step_video().await;
    let step = app.db().write(move |tx| {
        tx.conn().execute("INSERT INTO agent_steps(agent_id,message_id,name,status,position,created_at,updated_at) VALUES(773018776,?,'Inspect video','running',0,?,?)", rusqlite::params![id,tx.now(),tx.now()])?;
        Ok(tx.conn().last_insert_rowid())
    }).await.unwrap();
    let (_client, server) = subscribe(&app).await;
    app.publications().take();
    let reply = app.anonymous().send(Req::new(axum::http::Method::PATCH, &format!("/agents/steps/{step}"))
        .header("authorization", &format!("Bearer {}", crate::controllers::agent_http_tests::SECRET))
        .header("accept", "application/json").header("content-type", "application/json")
        .body(json!({"status":"done"}).to_string())).await;
    let mut actual = event_snapshot(&app, blob).await;
    actual["http_status"] = json!(reply.status.as_u16());
    server.abort();
    assert_eq!(actual, event_oracle()["step_update"], "{}", reply.text());
}

#[tokio::test]
async fn attachment_processing_full_render_event_sweep_matches_fresh_rails() {
    use campfire_db::{Event, broadcasts::Broadcast};
    let mut actuals = serde_json::Map::new();
    let mut expectations = serde_json::Map::new();
    for kind in ["notifier", "digest", "system_note", "stage_note", "thread_indicator"] {
        let (app, _, id, blob) = setup(true).await;
        if kind == "stage_note" {
            app.db().write(move |tx| {
                tx.conn().execute("UPDATE rooms SET type='Rooms::Stage' WHERE id=?", [ALL_TALK])?;
                tx.conn().execute("UPDATE messages SET system_note=1 WHERE id=?", [id])?;
                Ok(())
            }).await.unwrap();
        }
        let (_client, server) = subscribe(&app).await;
        app.publications().take();
        app.db().write(move |tx| {
            let event = match kind {
                "notifier" => Event::broadcast(&crate::integrations::github::notifier::MessageCreated {
                    room_id:ALL_TALK, message_id:id, thread_id:None,
                }),
                "digest" | "system_note" => {
                    if kind == "system_note" {
                        tx.conn().execute("UPDATE messages SET system_note=1 WHERE id=?", [id])?;
                    }
                    Event::broadcast(&campfire_db::models::board_automations::DigestNotes {message_ids:vec![id]})
                },
                "stage_note" => Event::broadcast(&campfire_db::models::huddle_effects::StageEndedNote {message_id:id}),
                "thread_indicator" => {
                    Event::broadcast(&Broadcast::ThreadIndicator { message_id: id, reply_count: 1 })
                },
                _ => unreachable!(),
            };
            tx.emit_after_commit(event);
            Ok(())
        }).await.unwrap();
        let expected = match kind {
            "notifier" | "digest" => "full_append",
            "stage_note" => "system_note",
            _ => kind,
        };
        let actual = event_snapshot(&app, blob).await;
        server.abort();
        actuals.insert(kind.into(), actual);
        expectations.insert(kind.into(), event_oracle()[expected].clone());
    }
    assert_eq!(actuals, expectations, "full render events must retain recovery; component-only events must not invent it");
}

#[tokio::test]
async fn attachment_processing_terminal_failure_never_restarts_from_views() {
    let (app, clock, id, blob) = setup(true).await;
    seed_job(&app, id, blob).await;
    let runner = start(&app);
    let waits = tokio::time::timeout(Duration::from_secs(15), async {
        let mut waits = Vec::new();
        loop {
            let job = app
                .db()
                .read(|c| {
                    Ok(c.query_row(
                        "SELECT status,run_at FROM background_jobs WHERE job_class=?",
                        [CLASS],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, Timestamp>(1)?)),
                    )?)
                })
                .await
                .unwrap();
            if job.0 == "failed" {
                break;
            }
            if job.0 == "ready" && job.1.jiff() > campfire_kit::Clock::now(&*clock) {
                waits.push(
                    (job.1.jiff().as_microsecond()
                        - campfire_kit::Clock::now(&*clock).as_microsecond())
                        as f64
                        / 1_000_000.0,
                );
            }
            clock.set(job.1.jiff());
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        waits
    })
    .await
    .unwrap();
    assert_eq!(waits.len(), 2);
    for (wait, base) in waits.into_iter().zip([3.0, 18.0]) {
        assert!((base..=(base + (base - 2.0) * 0.15)).contains(&wait));
    }
    runner.shutdown(Duration::from_secs(1)).await;
    assert_eq!(state(&app, blob).await.0.as_deref(), Some("failed"));
    let message=render(&app,id).await;
    assert!(message["attachment"]["thumbnailUrl"].is_null());
    for _ in 0..3 {
        view(&app, id).await;
    }
    assert_eq!(state(&app, blob).await.2, 1);
    let attempts = app
        .db()
        .read(|c| {
            Ok(c.query_row(
                "SELECT attempts FROM background_jobs WHERE job_class=?",
                [CLASS],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(attempts, 3);
}

#[tokio::test]
async fn attachment_processing_restart_performs_the_persisted_job() {
    let (app, _, id, blob) = setup(false).await;
    seed_job(&app, id, blob).await;
    app.db().write(|tx| {
        tx.conn().execute("UPDATE background_jobs SET status='running',attempts=1,claimed_by='stopped-process',lease_expires_at=? WHERE job_class=?", rusqlite::params![tx.now().since(jiff::SignedDuration::from_secs(-1)), CLASS])?;
        Ok(())
    }).await.unwrap();
    let runner = start(&app);
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let done = app.db().read(|c| Ok(c.query_row("SELECT NOT EXISTS(SELECT 1 FROM background_jobs WHERE job_class=? AND status!='failed')", [CLASS], |r| r.get::<_,bool>(0))?)).await.unwrap();
            if done { break; }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await.unwrap();
    runner.shutdown(Duration::from_secs(1)).await;
    let storage = app.booted.app.storage.clone();
    let present = app
        .db()
        .read(move |c| {
            let source = Blob::find(c, blob).unwrap().unwrap();
            let preview = storage.existing_preview_image(c, &source).unwrap();
            Ok(preview.is_some_and(|p| {
                storage
                    .existing_variant(c, &p, &campfire_storage::Variation::format_only("webp"))
                    .unwrap()
                    .is_some_and(|v| storage.service.exist(&v.key))
            }))
        })
        .await
        .unwrap();
    assert!(
        present,
        "restart must generate and retain JPEG and WebP files"
    );
    assert_eq!(state(&app, blob).await.0, None);
}

#[tokio::test]
async fn attachment_processing_rows_html_and_broadcast_bytes_match_fresh_rails() {
    let (app, _, id, blob_id) = setup(false).await;
    let (mut client, server) = json_subscribe(&app).await;
    let server = AbortServer(server);
    let expected = oracle()["success"].clone();
    assert!(render(&app,id).await["attachment"]["thumbnailUrl"].is_null());
    let (token, expires, count) = state(&app, blob_id).await;
    assert_eq!(count, 1);
    assert_eq!(
        token.as_deref().unwrap().rsplit_once(':').unwrap().1,
        expected["pending"]["token_suffix"]
    );
    assert_eq!(
        expires
            .unwrap()
            .jiff()
            .strftime("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
        expected["pending"]["message_processing_expires_at"]
    );
    app.publications().take();
    perform_queued(&app, 1).await.unwrap();
    client.until(|event| matches!(&event.payload, campfire_api_types::SyncPayload::MessageUpdated(message) if message.id == id && message.attachment.as_ref().is_some_and(|file| file.thumbnail_url.is_some())), |_| false).await;
    assert!(render(&app,id).await["attachment"]["thumbnailUrl"].is_string());
    let storage = app.booted.app.storage.clone();
    let expected_message = expected["message"].clone();
    let expected_blob = expected["blob"].clone();
    let actual = app.db().read(move |conn| {
        let row = crate::controllers::agent_review_r2_tests::row_state;
        let message = row(conn, &format!("SELECT * FROM messages WHERE id={id}"), &expected_message)?;
        let blob = row(conn, &format!("SELECT * FROM active_storage_blobs WHERE id={blob_id}"), &expected_blob)?;
        let source = Blob::find(conn, blob_id).unwrap().unwrap();
        let preview = storage.existing_preview_image(conn, &source).unwrap().unwrap();
        let webp = storage.existing_variant(conn, &preview, &campfire_storage::Variation::format_only("webp")).unwrap().unwrap();
        let files = [&source, &preview, &webp].map(|b| {
            use sha2::Digest;
            let bytes = storage.service.download(&b.key).unwrap();
            json!({"content_type":b.content_type,"bytes":bytes.len(),"sha256":format!("{:x}", sha2::Sha256::digest(&bytes))})
        });
        let preview_metadata = serde_json::from_str::<serde_json::Value>(&preview.metadata.encode()).unwrap();
        let analysis = conn.prepare("SELECT json_extract(arguments,'$.blob_id') FROM background_jobs WHERE job_class='ActiveStorage::AnalyzeJob' ORDER BY id")?.query_map([], |r| r.get::<_,i64>(0))?.collect::<Result<Vec<_>,_>>()?;
        Ok((preview_metadata, json!(files), message, blob, json!(analysis)))
    }).await.unwrap();
    drop(server);
    assert_eq!(actual.0, expected["preview_metadata"]);
    assert_eq!(actual.2, expected["message"]);
    assert_eq!(actual.3, expected["blob"]);
    assert_eq!(actual.4, expected["image_analysis_jobs"]);
    assert_eq!(actual.1, expected["files"]);
}

#[tokio::test]
async fn attachment_processing_refused_enqueues_back_off_durably_then_recover() {
    let (app, clock, id, blob) = setup(false).await;
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_processing BEFORE INSERT ON background_jobs WHEN NEW.job_class='Message::AttachmentProcessingJob' BEGIN SELECT RAISE(ABORT,'queue refused'); END")?;
        Ok(())
    }).await.unwrap();
    for (index, delay) in oracle()["enqueue_cooldowns"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let before = campfire_kit::Clock::now(&*clock);
        for _ in 0..3 {
            view(&app, id).await;
        }
        let (token, expires, count) = state(&app, blob).await;
        assert_eq!(count, 0);
        assert_eq!(token.as_deref(), delay["message_processing_token"].as_str());
        assert_eq!(
            expires.unwrap().jiff().duration_since(before).as_secs(),
            delay["delay"].as_f64().unwrap() as i64
        );
        assert_eq!(
            token.unwrap(),
            format!("enqueue_failed:{}", (index + 1).min(5))
        );
        clock.set(expires.unwrap().jiff());
    }
    app.db()
        .write(|tx| {
            tx.conn().execute_batch("DROP TRIGGER reject_processing")?;
            Ok(())
        })
        .await
        .unwrap();
    view(&app, id).await;
    assert_eq!(state(&app, blob).await.2, 1);
    perform_queued(&app, 1).await.unwrap();
    assert_eq!(state(&app, blob).await.0, None);
}

#[tokio::test]
async fn bot_caption_edit_recovers_video_preview_after_enqueue_failure() {
    let (app, clock, id, blob) = setup(false).await;
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE messages SET creator_id=? WHERE id=?", (BENDER, id))?;
        tx.conn().execute_batch("CREATE TRIGGER reject_bot_preview BEFORE INSERT ON background_jobs WHEN NEW.job_class='Message::AttachmentProcessingJob' BEGIN SELECT RAISE(ABORT,'queue refused'); END")?;
        Message::find(tx.conn(), id)?.replace_attachment(tx, Some(blob))
    }).await.unwrap();
    let (token, expires, count) = state(&app, blob).await;
    assert_eq!(token.as_deref(), Some("enqueue_failed:1"));
    assert_eq!(count, 0);
    clock.set(expires.unwrap().jiff());
    app.db().write(|tx| {
        tx.conn().execute_batch("DROP TRIGGER reject_bot_preview")?;
        Ok(())
    }).await.unwrap();

    let path = format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages/{id}");
    let mut bot = app.anonymous();
    for caption in ["Updated caption", "Another caption"] {
        let response = bot.send(Req::new(axum::http::Method::PUT, &path).body(caption)).await;
        assert_eq!(response.status.as_u16(), 200, "{}", response.text());
        assert_eq!(response.json()["body"]["plain_text"], caption);
        let (token, expires, count) = state(&app, blob).await;
        assert_eq!(count, 1, "bot caption edits must recover the pending video preview exactly once");
        assert!(token.unwrap().ends_with(":1"));
        assert!(expires.is_some());
    }
    app.db().read(move |conn| {
        let message = Message::find(conn, id)?;
        assert_eq!(message.attachment(conn)?.unwrap().1.id, blob);
        assert!(message.body_html(conn)?.unwrap().contains("Another caption"));
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn attachment_processing_replacements_cover_room_thread_multipart_and_direct() {
    for thread in [false, true] {
        for multipart in [false, true] {
            let (app, _, _, blob) = setup(false).await;
            let message = app
                .db()
                .write(move |tx| {
                    let attributes = NewMessage {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        attachment_blob_id: Some(1),
                        ..Default::default()
                    };
                    if thread {
                        let mut thread = campfire_db::ChannelThread::create(
                            tx,
                            campfire_db::NewChannelThread {
                                room_id: ALL_TALK,
                                creator_id: DAVID,
                                ..Default::default()
                            },
                        )?;
                        thread.post_message(tx, DAVID, attributes)
                    } else {
                        Message::create(tx, attributes)
                    }
                })
                .await
                .unwrap();
            let path = message.thread_id.map_or_else(
                || format!("/rooms/{ALL_TALK}/messages/{}", message.id),
                |t| format!("/rooms/{ALL_TALK}/threads/{t}/messages/{}", message.id),
            );
            let mut req =
                Req::new(axum::http::Method::PATCH, &path).header("accept", "application/json");
            let bytes = app
                .db()
                .read({
                    let storage = app.booted.app.storage.clone();
                    move |c| {
                        Ok(storage
                            .service
                            .download(&Blob::find(c, blob).unwrap().unwrap().key)
                            .unwrap())
                    }
                })
                .await
                .unwrap();
            if multipart {
                req = req.multipart(
                    &[],
                    (
                        "message[attachment]",
                        "alpha-centuri.mov",
                        "video/quicktime",
                        &bytes,
                    ),
                );
            } else {
                let signed = campfire_storage::paths::signed_blob_id(
                    &*app.booted.app.storage.verifier,
                    blob,
                    None,
                );
                req = req
                    .header("content-type", "application/json")
                    .body(json!({"message":{"attachment":signed}}).to_string());
            }
            let response = app.david().write(req).await;
            assert_eq!(response.status.as_u16(), 200, "{}", response.text());
            let replacement = app
                .db()
                .read({
                    let id = message.id;
                    move |c| Ok(Message::find(c, id)?.attachment(c)?.unwrap().1.id)
                })
                .await
                .unwrap();
            assert_eq!(state(&app, replacement).await.2, 1);
            perform_queued(&app, 1).await.unwrap();
            assert!(render(&app,message.id).await["attachment"]["thumbnailUrl"].is_string());
        }
    }
}

#[tokio::test]
async fn attachment_processing_edited_scheduler_refreshes_every_other_current_owner() {
    let (app, clock, id, blob) = setup(false).await;
    let (mut client, server) = json_subscribe(&app).await;
    let other = app
        .db()
        .write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    attachment_blob_id: Some(blob),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    seed_job(&app, id, blob).await;
    // The scheduling message changes while its durable job still names the old blob.
    app.db()
        .write(move |tx| Message::find(tx.conn(), id)?.replace_attachment(tx, Some(1)))
        .await
        .unwrap();
    // Keep just the old-video worker; the replacement image has its own independent job.
    app.db().write(move |tx| { tx.conn().execute("DELETE FROM background_jobs WHERE job_class=? AND json_extract(arguments,'$.blob_id')!=?", rusqlite::params![CLASS, blob])?; Ok(()) }).await.unwrap();
    clock.advance(jiff::SignedDuration::from_secs(20));
    app.publications().take();
    perform_queued(&app, 1).await.unwrap();
    let other_id = other.id;
    client.until(|event| matches!(&event.payload, campfire_api_types::SyncPayload::MessageUpdated(message) if message.id == other_id && message.attachment.as_ref().is_some_and(|file| file.thumbnail_url.is_some())), |event| matches!(&event.payload, campfire_api_types::SyncPayload::MessageUpdated(message) if message.id == id)).await;
    let touched = app
        .db()
        .read(move |c| Message::find(c, other.id))
        .await
        .unwrap();
    assert_eq!(touched.updated_at.jiff(), campfire_kit::Clock::now(&*clock));
    assert_eq!(render(&app,id).await["attachment"]["contentType"], "image/jpeg");
    server.abort();
}

#[tokio::test]
async fn attachment_processing_claims_renew_and_stale_owners_cannot_clear_new_leases() {
    let (app, clock, _, blob) = setup(false).await;
    use campfire_db::models::message_attachment_processing as processing;
    let claims = app
        .db()
        .write(move |tx| {
            Ok([
                processing::claim(tx, blob, "first:0")?,
                processing::claim(tx, blob, "second:0")?,
                processing::claim(tx, blob, "first:0")?,
            ])
        })
        .await
        .unwrap();
    let golden = oracle()["claim"].clone();
    assert_eq!(
        claims,
        [
            golden["claims"][0].as_bool().unwrap(),
            golden["claims"][1].as_bool().unwrap(),
            golden["renewal"].as_bool().unwrap()
        ]
    );
    clock.advance(jiff::SignedDuration::from_secs(900));
    app.db()
        .write(move |tx| {
            assert!(processing::claim(tx, blob, "second:0")?);
            processing::release(tx, blob, "first:0")?;
            processing::fail(tx, blob, "first:0")?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(state(&app, blob).await.0.as_deref(), Some("second:0"));
    app.db()
        .write(move |tx| processing::release(tx, blob, "second:0"))
        .await
        .unwrap();
    assert_eq!(state(&app, blob).await, (None, None, 0));
}

#[tokio::test]
async fn attachment_processing_inline_failure_recovers_when_a_detached_broadcast_renders() {
    let (app, _, id, blob) = setup(true).await;
    let source = app
        .db()
        .read(move |c| Ok(Blob::find(c, blob).unwrap().unwrap()))
        .await
        .unwrap();
    super::process_attachment(&app.booted.app, source)
        .await
        .unwrap();
    assert_eq!(state(&app, blob).await.0, None);
    app.db()
        .write(move |tx| {
            tx.emit_after_commit(campfire_db::Event::broadcast(
                &campfire_db::broadcasts::Broadcast::MessageCreated { message_id: id },
            ));
            Ok(())
        })
        .await
        .unwrap();
    let (token, _, count) = state(&app, blob).await;
    assert_eq!(count, 1);
    assert!(token.unwrap().ends_with(":0"));
}

#[tokio::test]
async fn attachment_processing_reassigning_the_same_blob_schedules_the_save_callback() {
    let (app, _, id, blob) = setup(false).await;
    app.db()
        .write(move |tx| Message::find(tx.conn(), id)?.replace_attachment(tx, Some(blob)))
        .await
        .unwrap();
    assert_eq!(
        state(&app, blob).await.2,
        1,
        "Rails after_save also processes an unchanged attachment assignment"
    );
}

#[tokio::test]
async fn attachment_processing_agent_root_processes_before_its_create_broadcast() {
    use crate::controllers::agent_http_tests::{AGENT, SECRET};
    for corrupt in [false, true] {
        let (app, _, _, blob) = setup(corrupt).await;
        crate::controllers::agent_http_tests::initialize(&app).await;
        app.db().write(|tx| {
            tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES(?,'post_messages',127326141,?,?)", rusqlite::params![AGENT,tx.now(),tx.now()])?;
            Ok(())
        }).await.unwrap();
        let (mut client, server) = json_subscribe(&app).await;
        app.publications().take();
        let signed = campfire_storage::paths::signed_blob_id(&*app.booted.app.storage.verifier, blob, None);
        let response = app.anonymous().send(
            Req::new(axum::http::Method::POST, &format!("/rooms/{ALL_TALK}/agents/messages"))
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .header("authorization", &format!("Bearer {SECRET}"))
                .body(json!({"message":{"attachment":signed,"client_message_id":"root-processing-order"}}).to_string())
        ).await;
        assert_eq!(response.status.as_u16(), 201, "{}", response.text());
        let storage = app.booted.app.storage.clone();
        app.db().read(move |conn| {
            let source = Blob::find(conn, blob).unwrap().unwrap();
            assert!(source.is_analyzed(), "Rails analyzes before broadcasting the root post");
            assert_eq!(storage.existing_preview_image(conn, &source).unwrap().is_some(), !corrupt);
            Ok(())
        }).await.unwrap();
        let event = client.until(|event| matches!(&event.payload, campfire_api_types::SyncPayload::MessageCreated(message) if message.client_message_id == "root-processing-order"), |_| false).await;
        let campfire_api_types::SyncPayload::MessageCreated(message) = event.payload else { unreachable!() };
        assert_eq!(message.attachment.unwrap().thumbnail_url.is_some(), !corrupt);
        assert_eq!(state(&app, blob).await.2, if corrupt { 1 } else { 0 });
        server.abort();
    }
}
