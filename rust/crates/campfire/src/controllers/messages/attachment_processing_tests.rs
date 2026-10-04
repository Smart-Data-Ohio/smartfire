//! #226: exercise real rendering, durable scheduling and the registered media worker.
use crate::controllers::presenters::test_support::*;
use askama::Template;
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

async fn render(app: &TestApp, id: i64) -> String {
    let runtime = app.booted.app.clone();
    let (html, refreshes) = app
        .db()
        .read(move |conn| {
            let presenter = crate::controllers::presenters::Presenter::new(conn, &runtime, None);
            let view = presenter.message(&Message::find(conn, id)?)?;
            let account = campfire_db::Account::first(conn)?;
            let html = crate::controllers::presenters::page::render_detached_at(
                &runtime,
                account.as_ref(),
                "http://example.org",
                |ctx| {
                    campfire_views::messages::PresentationPartial {
                        ctx,
                        message: &view,
                    }
                    .render()
                    .unwrap()
                },
            );
            Ok((html, presenter.take_render_refreshes()))
        })
        .await
        .unwrap();
    crate::controllers::presenters::refresh_after_render(app.db(), refreshes).await;
    html
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

async fn setup(corrupt: bool) -> (TestApp, Arc<FrozenClock>, i64, i64) {
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

async fn view(app: &TestApp, id: i64) {
    let response = app
        .david()
        .get(&format!("/rooms/{ALL_TALK}/messages/{id}"))
        .await;
    assert_eq!(response.status.as_u16(), 200, "{}", response.text());
    assert!(
        !response.text().contains("poster=\""),
        "pending preview must omit its poster"
    );
}

async fn state(app: &TestApp, blob: i64) -> (Option<String>, Option<Timestamp>, i64) {
    app.db().read(move |c| {
        let (token, expires) = c.query_row("SELECT message_processing_token,message_processing_expires_at FROM active_storage_blobs WHERE id=?", [blob], |r| Ok((r.get(0)?, r.get(1)?)))?;
        let count = c.query_row("SELECT count(*) FROM background_jobs WHERE job_class=? AND COALESCE(json_extract(arguments,'$.blob_id'),json_extract(arguments,'$._campfire_retry_metadata_v1.arguments.blob_id'))=?", rusqlite::params![CLASS, blob], |r| r.get(0))?;
        Ok((token, expires, count))
    }).await.unwrap()
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
    let room = app
        .db()
        .read(|c| campfire_db::Room::find(c, ALL_TALK))
        .await
        .unwrap();
    let gid = crate::channels::room_gid(&room).to_param();
    let identifier = crate::channels::tests::support::identifier(
        json!({"channel":"RoomMessagesChannel","signed_stream_name":rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, &[&gid,"messages"])}),
    );
    client.confirm(&identifier).await;
    (client, serving)
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
    let (_client, server) = subscribe(&app).await;
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
    assert_eq!(app.publications().take().len(), 1, "the retry must publish completion");
    assert!(render(&app, id).await.contains("poster=\""));
    server.abort();
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
    let html = render(&app, id).await;
    assert!(html.contains("<video"));
    assert!(!html.contains("poster=") && !html.contains("spinner"));
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
    let (_client, server) = subscribe(&app).await;
    let expected = oracle()["success"].clone();
    assert_eq!(render(&app, id).await, expected["before"].as_str().unwrap());
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
    let frames = app.publications().take().into_iter().map(|(channel, bytes)| json!({"channel":channel,"payload":serde_json::from_str::<serde_json::Value>(&bytes).unwrap()})).collect::<Vec<_>>();
    assert_eq!(json!(frames), expected["frames"]);
    assert_eq!(render(&app, id).await, expected["after"].as_str().unwrap());
    let storage = app.booted.app.storage.clone();
    let actual = app.db().read(move |conn| {
        let row = crate::controllers::agent_review_r2_tests::row_state;
        let message = row(conn, &format!("SELECT * FROM messages WHERE id={id}"), &expected["message"])?;
        let blob = row(conn, &format!("SELECT * FROM active_storage_blobs WHERE id={blob_id}"), &expected["blob"])?;
        let source = Blob::find(conn, blob_id).unwrap().unwrap();
        let preview = storage.existing_preview_image(conn, &source).unwrap().unwrap();
        let webp = storage.existing_variant(conn, &preview, &campfire_storage::Variation::format_only("webp")).unwrap().unwrap();
        let files = [&source, &preview, &webp].map(|b| {
            use sha2::Digest;
            let bytes = storage.service.download(&b.key).unwrap();
            json!({"content_type":b.content_type,"bytes":bytes.len(),"sha256":format!("{:x}", sha2::Sha256::digest(&bytes))})
        });
        assert_eq!(serde_json::from_str::<serde_json::Value>(&preview.metadata.encode()).unwrap(), expected["preview_metadata"]);
        assert_eq!(json!(files), expected["files"]);
        assert_eq!(message, expected["message"]);
        assert_eq!(blob, expected["blob"]);
        let analysis = conn.prepare("SELECT json_extract(arguments,'$.blob_id') FROM background_jobs WHERE job_class='ActiveStorage::AnalyzeJob' ORDER BY id")?.query_map([], |r| r.get::<_,i64>(0))?.collect::<Result<Vec<_>,_>>()?;
        assert_eq!(json!(analysis), expected["image_analysis_jobs"]);
        Ok(())
    }).await;
    actual.unwrap();
    server.abort();
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
            assert!(render(&app, message.id).await.contains("poster=\""));
        }
    }
}

#[tokio::test]
async fn attachment_processing_edited_scheduler_refreshes_every_other_current_owner() {
    let (app, clock, id, blob) = setup(false).await;
    let (_client, server) = subscribe(&app).await;
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
    let frames = app.publications().take();
    assert_eq!(frames.len(), 1);
    let payload = serde_json::from_str::<String>(&frames[0].1).unwrap();
    assert!(payload.contains(&format!("presentation_message_{}", other.client_message_id)));
    assert!(!payload.contains("presentation_message_attachment-processing-1"));
    let touched = app
        .db()
        .read(move |c| Message::find(c, other.id))
        .await
        .unwrap();
    assert_eq!(touched.updated_at.jiff(), campfire_kit::Clock::now(&*clock));
    assert!(render(&app, id).await.contains("<img"));
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
            let message = Message::find(tx.conn(), id)?;
            let room = campfire_db::Room::find(tx.conn(), message.room_id)?;
            tx.emit_after_commit(campfire_db::Event::broadcast(
                &campfire_db::broadcasts::Broadcast::append(
                    campfire_db::broadcasts::conversation_messages(tx.conn(), &message)?,
                    campfire_db::broadcasts::room_dom_id(&room, Some("messages")),
                    campfire_db::broadcasts::Partial::Message { message_id: id },
                ),
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
async fn attachment_processing_cached_collection_recovers_a_lost_enqueue_without_rebuilding() {
    let (app, _, id, blob) = setup(false).await;
    let path = format!("/rooms/{ALL_TALK}/messages");
    let mut browser = app.david();
    assert_eq!(browser.get(&path).await.status.as_u16(), 200);
    async fn fragment(app: &TestApp, id: i64) -> Arc<String> {
        let runtime = app.booted.app.clone();
        app.db()
            .read(move |c| {
                let presenter = crate::controllers::presenters::Presenter::new(c, &runtime, None);
                let message = Message::find(c, id)?;
                let key = presenter.message_fragment_cache_key(&message, "http://campfire.test")?;
                Ok(campfire_views::fragment_cache::with(
                    &runtime.fragment_cache,
                    || campfire_views::fragment_cache::read(&key).unwrap(),
                ))
            })
            .await
            .unwrap()
    }
    let before = fragment(&app, id).await;
    app.db().write(move |tx| {
        tx.conn().execute("DELETE FROM background_jobs WHERE job_class=?", [CLASS])?;
        tx.conn().execute("UPDATE active_storage_blobs SET message_processing_token=NULL,message_processing_expires_at=NULL WHERE id=?", [blob])?;
        Ok(())
    }).await.unwrap();
    assert_eq!(browser.get(&path).await.status.as_u16(), 200);
    let after = fragment(&app, id).await;
    assert!(
        Arc::ptr_eq(&before, &after),
        "the HTML must actually come from the warm fragment cache"
    );
    assert_eq!(state(&app, blob).await.2, 1);
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
        let (_client, server) = subscribe(&app).await;
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
        let frames = app.publications().take();
        let create = frames.iter().find(|(_, bytes)| bytes.contains("action=\\\"append\\\"")).unwrap();
        assert_eq!(create.1.contains("poster=\\\""), !corrupt, "the first root broadcast must use the processed attachment");
        assert_eq!(state(&app, blob).await.2, if corrupt { 1 } else { 0 });
        server.abort();
    }
}
