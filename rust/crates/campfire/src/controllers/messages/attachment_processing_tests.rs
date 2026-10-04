//! #226 failing-first regressions against 0373dfbd9's production code.
use crate::controllers::presenters::test_support::*;
use campfire_db::{Message, NewMessage, Timestamp};
use campfire_jobs::{JobQueue, QueueConfig, RunnerConfig};
use campfire_kit::FrozenClock;
use campfire_storage::Blob;
use serde_json::json;
use std::{sync::Arc, time::Duration};
const CLASS: &str = "Message::AttachmentProcessingJob";

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
async fn attachment_processing_expired_lease_recovers_but_active_and_terminal_do_not() {
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
async fn attachment_processing_terminal_failure_never_restarts_from_views() {
    let (app, clock, id, blob) = setup(true).await;
    seed_job(&app, id, blob).await;
    let runner = start(&app);
    tokio::time::timeout(Duration::from_secs(15), async {
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
            clock.set(job.1.jiff());
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    runner.shutdown(Duration::from_secs(1)).await;
    assert_eq!(state(&app, blob).await.0.as_deref(), Some("failed"));
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

