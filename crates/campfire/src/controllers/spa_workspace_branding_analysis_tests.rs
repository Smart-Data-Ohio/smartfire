//! Branding analysis must never borrow an attachment's media slot.

use std::future::Future;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use campfire_storage::{Blob, Json, Variation, branding};

use super::*;

async fn with_media_observation<F: Future>(work: F) -> (F::Output, bool) {
    tokio::pin!(work);
    let mut full = true;
    loop {
        let (available, capacity) = campfire_web::active_storage::media_permits();
        full &= available == capacity;
        tokio::select! {
            result = &mut work => {
                let (available, capacity) = campfire_web::active_storage::media_permits();
                return (result, full && available == capacity);
            }
            () = tokio::time::sleep(Duration::from_millis(1)) => {}
        }
    }
}

async fn blob(a: &TestApp, id: i64) -> Blob {
    a.db()
        .read(move |conn| Ok(Blob::find(conn, id).unwrap().unwrap()))
        .await
        .unwrap()
}

async fn attach_legacy(a: &TestApp, id: i64, name: &'static str) {
    a.db()
        .write(move |tx| {
            use crate::controllers::presenters::attachments::{self, Record};
            let account = campfire_db::Account::first(tx.conn())?.unwrap();
            let blob = Blob::find(tx.conn(), id).unwrap().unwrap();
            attachments::attach_existing(tx, Record::account(account.id), name, blob)
        })
        .await
        .unwrap();
}

async fn observe_analysis_jobs(a: &TestApp) {
    a.db()
        .write(|tx| {
            tx.conn().execute_batch(
                "CREATE TABLE branding_analysis_jobs (arguments TEXT NOT NULL);
                 CREATE TRIGGER observe_branding_analysis AFTER INSERT ON background_jobs
                 WHEN NEW.job_class = 'ActiveStorage::AnalyzeJob'
                 BEGIN INSERT INTO branding_analysis_jobs VALUES (NEW.arguments); END;",
            )?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn analysis_jobs(a: &TestApp) -> i64 {
    a.db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT COUNT(*) FROM branding_analysis_jobs", [], |row| {
                    row.get(0)
                })?,
            )
        })
        .await
        .unwrap()
}

fn assert_metadata(blob: &Blob, width: i64, height: i64, animated: bool) {
    assert!(blob.is_identified());
    assert!(blob.is_analyzed());
    assert_eq!(blob.metadata.get("width"), Some(&Json::Int(width)));
    assert_eq!(blob.metadata.get("height"), Some(&Json::Int(height)));
    assert_eq!(
        blob.metadata.get(branding::ANIMATED_KEY),
        Some(&Json::Bool(animated))
    );
}

#[tokio::test]
async fn spa_workspace_branding_saves_with_running_jobs_keep_analyzer_metadata() {
    let _branding = BRANDING_TESTS.lock().await;
    let Some(mut a) = TestApp::boot_seed_with_env(
        "default",
        crate::controllers::presenters::test_support::seed_clock(),
        &[("SPA_ENABLED", "1")],
    )
    .await
    else {
        return;
    };
    observe_analysis_jobs(&a).await;
    let mut admin = a.sign_in(DAVID).await;
    let ((before, after, enqueued), media_full) = with_media_observation(async {
        let mut before = Vec::new();
        let mut current = Vec::new();
        for kind in ["logo", "banner"] {
            // Replacements enqueue real purge jobs while the ordinary runner stays live.
            for animated in [false, true] {
                let (bytes, filename, content_type) = if animated {
                    (animated_gif(), "animated.gif", "image/gif")
                } else {
                    (png(7, 3, false), "static.png", "image/png")
                };
                let (signed, id) = upload_bytes(&a, &bytes, filename, content_type).await;
                let _: api::Workspace = spa(
                    &mut admin,
                    Method::PUT,
                    &format!("/api/v1/admin/workspace/{kind}"),
                    json!({"signedId": signed}),
                )
                .await;
                before.push((blob(&a, id).await, animated));
                if animated {
                    current.push(id);
                }
            }
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let remaining = a
                    .db()
                    .write(|tx| {
                        Ok(tx.conn().query_row(
                            "SELECT COUNT(*) FROM background_jobs WHERE job_class IN
                         ('ActiveStorage::AnalyzeJob', 'ActiveStorage::PurgeJob')",
                            [],
                            |row| row.get::<_, i64>(0),
                        )?)
                    })
                    .await
                    .unwrap();
                if remaining == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        let mut after = Vec::new();
        for id in current {
            after.push(blob(&a, id).await);
        }
        (before, after, analysis_jobs(&a).await)
    })
    .await;
    drop(admin);
    a.booted.jobs.stop(Duration::from_secs(1)).await;
    assert_eq!(
        enqueued, 0,
        "validated branding must enqueue no analysis jobs"
    );
    assert!(
        media_full,
        "saving and draining branding jobs must leave media permits full"
    );
    for (blob, animated) in before {
        let (width, height) = if animated { (1, 1) } else { (7, 3) };
        assert_metadata(&blob, width, height, animated);
    }
    for blob in after {
        assert_metadata(&blob, 1, 1, true);
    }
}

#[tokio::test]
async fn spa_workspace_branding_rendered_variants_keep_dimensions_without_analysis() {
    let _branding = BRANDING_TESTS.lock().await;
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    for kind in ["logo", "banner"] {
        let (signed, id) = upload_bytes(&a, &png(7, 3, false), "static.png", "image/png").await;
        let workspace: api::Workspace = spa(
            &mut admin,
            Method::PUT,
            &format!("/api/v1/admin/workspace/{kind}"),
            json!({"signedId": signed}),
        )
        .await;
        a.db()
            .write(|tx| {
                tx.conn().execute("DELETE FROM background_jobs", [])?;
                Ok(())
            })
            .await
            .unwrap();
        let url = if kind == "logo" {
            workspace.logo_url
        } else {
            workspace.banner_url.unwrap()
        };
        assert_eq!(admin.get(&url).await.status, StatusCode::OK);
        let (image, jobs) = a.db().read(move |conn| {
            let original = Blob::find(conn, id).unwrap().unwrap();
            let (width, height) = if kind == "logo" { (512, 512) } else { (1920, 1080) };
            let image = a_variant(conn, &original, Variation::resize_to_limit(width, height, Some("png")));
            let jobs = conn.query_row(
                "SELECT COUNT(*) FROM background_jobs WHERE job_class = 'ActiveStorage::AnalyzeJob'",
                [], |row| row.get::<_, i64>(0),
            )?;
            Ok((image, jobs))
        }).await.unwrap();
        assert!(image.is_analyzed());
        assert_eq!(image.metadata.get("width"), Some(&Json::Int(7)));
        assert_eq!(image.metadata.get("height"), Some(&Json::Int(3)));
        assert_eq!(jobs, 0);
    }
}

fn a_variant(conn: &rusqlite::Connection, original: &Blob, variation: Variation) -> Blob {
    let record_id =
        campfire_storage::blob::find_variant_record(conn, original.id, &variation.digest())
            .unwrap()
            .unwrap();
    Blob::attached(conn, "ActiveStorage::VariantRecord", record_id, "image")
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn spa_workspace_branding_full_pool_render_deadlines_keep_existing_fallbacks() {
    let _branding = BRANDING_TESTS.lock().await;
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let stock = admin.get("/account/logo").await.body;
    let (_, id) = upload_bytes(&a, &png(7, 3, false), "legacy.png", "image/png").await;
    attach_legacy(&a, id, "logo").await;
    attach_legacy(&a, id, "banner").await;
    let source = blob(&a, id).await;
    let queued =
        branding::test_hooks::stall(&source.key, campfire_storage::vips::StallPhase::Header);
    let (_, blocked_id) = upload_bytes(&a, &animated_gif(), "blocked.gif", "image/gif").await;
    let blocked_blob = blob(&a, blocked_id).await;
    let blocked = branding::test_hooks::block_reads(&blocked_blob.key);
    let first = campfire_web::active_storage::process_branding_with_deadline(
        branding::processing_timeout(&blocked_blob),
        {
            let storage = a.booted.app.storage.clone();
            move |cancel| {
                Ok(branding::prepare(
                    &storage,
                    blocked_blob,
                    branding::Kind::Logo,
                    &cancel,
                ))
            }
        },
    )
    .await;
    let ((responses, elapsed), media_full) = with_media_observation(async {
        let mut responses = Vec::new();
        let started = Instant::now();
        for path in [
            "/account/logo".to_owned(),
            format!("/account/logo?blob={id}"),
            format!("/account/banner?blob={id}"),
        ] {
            responses.push(tokio::time::timeout(Duration::from_secs(2), admin.get(&path)).await);
        }
        (responses, started.elapsed())
    })
    .await;
    let reached = blocked.reached.load(Ordering::Relaxed);
    let queued_reached = queued.reached.load(Ordering::Relaxed);
    let slots = campfire_web::active_storage::branding_permits();
    drop(queued);
    drop(blocked);
    branding_slots_are_free().await;
    assert!(first.is_err());
    assert!(reached);
    assert!(!queued_reached);
    assert_eq!(slots, (0, 1));
    assert!(media_full);
    assert!(elapsed >= Duration::from_millis(1200) && elapsed < Duration::from_secs(6));
    for (index, response) in responses.into_iter().enumerate() {
        let response =
            response.expect("render deadline includes waiting for the occupied branding slot");
        if index < 2 {
            assert_eq!(response.status, StatusCode::OK);
            assert_eq!(response.body, stock);
        } else {
            assert_eq!(response.status, StatusCode::NOT_FOUND);
        }
    }
}

#[tokio::test]
async fn spa_workspace_branding_legacy_analysis_uses_its_pool_and_deadline() {
    let _branding = BRANDING_TESTS.lock().await;
    let Some(a) = app().await else { return };
    let (_, id) = upload_bytes(&a, &png(7, 3, false), "legacy.png", "image/png").await;
    attach_legacy(&a, id, "logo").await;
    let source = blob(&a, id).await;
    let blocked = branding::test_hooks::block_reads(&source.key);
    let ((first, second), media_full) = with_media_observation(async {
        let first = tokio::time::timeout(
            Duration::from_secs(2),
            campfire_web::active_storage::analyze(&a.booted.app, id),
        )
        .await;
        let second = tokio::time::timeout(
            Duration::from_secs(2),
            campfire_web::active_storage::analyze(&a.booted.app, id),
        )
        .await;
        (first, second)
    })
    .await;
    let reached = blocked.reached.load(Ordering::Relaxed);
    let slots = campfire_web::active_storage::branding_permits();
    drop(blocked);
    branding_slots_are_free().await;
    assert!(first.unwrap().is_err());
    assert!(second.unwrap().is_err());
    assert!(reached);
    assert_eq!(slots, (0, 1));
    assert!(media_full);
    assert!(!blob(&a, id).await.is_analyzed());
    let analyzed = campfire_web::active_storage::analyze(&a.booted.app, id)
        .await
        .unwrap()
        .unwrap();
    assert!(analyzed.is_analyzed());
    assert_eq!(analyzed.metadata.get("width"), Some(&Json::Int(7)));
    assert_eq!(analyzed.metadata.get("height"), Some(&Json::Int(3)));
}

#[tokio::test]
async fn spa_workspace_branding_legacy_analysis_caps_bytes_before_open_or_copy() {
    let _branding = BRANDING_TESTS.lock().await;
    let Some(a) = app().await else { return };
    let (_, id) = upload_bytes(&a, &png(7, 3, false), "legacy.png", "image/png").await;
    attach_legacy(&a, id, "banner").await;
    let source = blob(&a, id).await;
    let path = a.booted.app.storage.service.path_for(&source.key);
    let small_size = std::fs::metadata(&path).unwrap().len();
    for (recorded, disk_size) in [
        (branding::MAX_BYTES as i64 + 1, small_size),
        (small_size as i64, branding::MAX_BYTES + 1),
    ] {
        a.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE active_storage_blobs SET byte_size = ? WHERE id = ?",
                    [recorded, id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(disk_size)
            .unwrap();
        let stalled =
            branding::test_hooks::stall(&source.key, campfire_storage::vips::StallPhase::Header);
        let (result, full) =
            with_media_observation(campfire_web::active_storage::analyze(&a.booted.app, id)).await;
        branding_slots_are_free().await;
        assert!(full);
        assert!(!stalled.reached.load(Ordering::Relaxed));
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("10 MB byte budget")
        );
        assert!(!blob(&a, id).await.is_analyzed());
    }
}
