//! PR192 R4: commit ownership, missing-file lookups and complete media relationships.
use super::agent_review_r3_tests::{closed_thread, fresh_source, post};
use super::presenters::test_support::{Req, TestApp};
use campfire_kit::Method;
use campfire_storage::{Blob, Variation};
use serde_json::{Value, json};

async fn media(app: &TestApp, source: i64) -> (Blob, Blob) {
    let pending = app.db().read(move |c| Ok(c.query_row("SELECT EXISTS(SELECT 1 FROM background_jobs WHERE job_class='Message::AttachmentProcessingJob' AND json_extract(arguments,'$.blob_id')=?)", [source], |r| r.get::<_,bool>(0))?)).await.unwrap();
    if pending { super::messages::attachment_processing_tests::perform_queued(app, 1).await.unwrap(); }
    app.db()
        .read(move |conn| {
            let preview = Blob::attached(conn, "ActiveStorage::Blob", source, "preview_image")
                .unwrap()
                .unwrap();
            let variant = conn.query_row(
                "SELECT id FROM active_storage_variant_records WHERE blob_id=? ORDER BY id LIMIT 1",
                [preview.id],
                |r| r.get::<_, i64>(0),
            )?;
            let image = Blob::attached(conn, "ActiveStorage::VariantRecord", variant, "image")
                .unwrap()
                .unwrap();
            Ok((preview, image))
        })
        .await
        .unwrap()
}
fn assert_file(app: &TestApp, blob: &Blob) {
    let storage = &app.booted.app.storage;
    let path = storage.service.path_for(&blob.key);
    assert!(
        storage.service.exist(&blob.key),
        "committed blob {} has lost its file",
        blob.id
    );
    assert_eq!(
        std::fs::metadata(&path).unwrap().len() as i64,
        blob.byte_size
    );
    assert_eq!(
        Some(campfire_storage::key::checksum_file(&path).unwrap()),
        blob.checksum
    );
}
#[tokio::test]
async fn pr192_r4_earlier_after_commit_error_preserves_committed_video_files_and_jobs() {
    use campfire_db::callbacks::Phase;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let app = closed_thread().await;
    let source = fresh_source(&app, 9).await;
    let earlier = Arc::new(AtomicBool::new(false));
    let later = Arc::new(AtomicBool::new(false));
    let observed = earlier.clone();
    let skipped = later.clone();
    app.booted.app.jobs.model_callbacks.install(Phase::MessageActivity, move |tx, _| {
        assert_eq!(tx.conn().query_row("SELECT count(*) FROM active_storage_attachments WHERE record_type='ActiveStorage::Blob' AND record_id=? AND name='preview_image'", [source], |r| r.get::<_, i64>(0))?, 0, "inject before media retention is registered");
        let observed = observed.clone();
        tx.after_commit(move |_| {
            observed.store(true, Ordering::SeqCst);
            Err(campfire_db::Error::Other("PR192 injected earlier after-commit failure".into()))
        });
        let skipped = skipped.clone();
        tx.after_commit(move |_| { skipped.store(true, Ordering::SeqCst); Ok(()) });
        Ok(())
    });
    let response = app
        .anonymous()
        .send(post(&app, source, "pr192-r4-after-commit"))
        .await;
    assert_eq!(response.status.as_u16(), 500);
    assert!(earlier.load(Ordering::SeqCst));
    assert!(
        !later.load(Ordering::SeqCst),
        "ordinary model callbacks retain Rails' failure ordering"
    );
    app.db().write(move |tx| {
        let message = tx.conn().query_row("SELECT id FROM messages WHERE client_message_id='pr192-r4-after-commit'", [], |r| r.get::<_,i64>(0))?;
        campfire_db::models::message_attachment_processing::recover(tx, message, source)
    }).await.unwrap();
    let (preview, image) = media(&app, source).await;
    let (preview_id, image_id) = (preview.id, image.id);
    app.db().read(move |conn| {
        let message = conn.query_row("SELECT id FROM messages WHERE client_message_id='pr192-r4-after-commit'", [], |r| r.get::<_, i64>(0))?;
        let closed: Option<String> = conn.query_row("SELECT closed_at FROM channel_threads WHERE id=1900700030", [], |r| r.get(0))?;
        assert_eq!(closed, None);
        let jobs = conn.prepare("SELECT job_class,arguments FROM background_jobs ORDER BY job_class")?.query_map([], |r| Ok(json!({"class":r.get::<_, String>(0)?, "args":serde_json::from_str::<Value>(&r.get::<_, String>(1)?).unwrap()})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        assert_eq!(jobs, vec![json!({"class":"ActiveStorage::AnalyzeJob","args":{"blob_id":source}}), json!({"class":"ActiveStorage::AnalyzeJob","args":{"blob_id":preview_id}}), json!({"class":"ActiveStorage::AnalyzeJob","args":{"blob_id":image_id}}), json!({"class":"ChannelThread::PushMessageJob","args":{"thread_id":1900700030,"message_id":message}})]);
        Ok(())
    }).await.unwrap();
    assert_file(&app, &preview);
    assert_file(&app, &image);
    let blob = app
        .db()
        .read(move |c| Ok(Blob::find(c, source).unwrap().unwrap()))
        .await
        .unwrap();
    let reused = crate::active_storage::processed_representation(
        &app.booted.app,
        blob,
        Variation::format_only("webp"),
    )
    .await
    .unwrap();
    assert_eq!(reused.id, image.id);
    assert_file(&app, &reused);
    println!(
        "PR192_R4_AFTER_COMMIT status=500 message/thread/media/four_jobs=committed preview/variant_files=retained later_model_callback=skipped"
    );
}
#[tokio::test]
async fn pr192_r4_missing_variant_retains_processed_metadata_and_file_checks() {
    missing_file(false).await;
}
#[tokio::test]
async fn pr192_r4_missing_preview_retains_processed_metadata_and_file_checks() {
    missing_file(true).await;
}
async fn missing_file(preview_missing: bool) {
    let app = closed_thread().await;
    let source = fresh_source(&app, 9).await;
    let response = app
        .anonymous()
        .send(post(&app, source, "pr192-r4-missing-file"))
        .await;
    assert_eq!(response.status.as_u16(), 201, "{}", response.text());
    let storage = &app.booted.app.storage;
    let blob = app
        .db()
        .read(move |c| Ok(Blob::find(c, source).unwrap().unwrap()))
        .await
        .unwrap();
    // Rails' signed JSON key turns the format symbol into a string, with a distinct
    // Marshal digest. Remove the exact variant the HTTP URL requests.
    let variation = Variation::decode(
        &*storage.verifier,
        &Variation::format_only("webp").key(&*storage.verifier),
        app.booted.app.clock.now(),
    )
    .unwrap();
    let image = crate::active_storage::processed_representation(
        &app.booted.app,
        blob.clone(),
        variation.clone(),
    )
    .await
    .unwrap();
    let (preview, _) = media(&app, source).await;
    let missing = if preview_missing { &preview } else { &image };
    storage.service.delete(&missing.key).unwrap();
    let before = super::agent_review_tests::snapshot(&app).await;
    let files = super::agent_review_tests::stored_files(&app);
    let representation = campfire_storage::paths::representation_redirect_path(
        &*storage.verifier,
        &blob,
        &variation,
    );
    let checked_storage = storage.clone();
    let checked_source = blob.clone();
    let checked_preview = preview.clone();
    let checked_variation = variation.clone();
    let missing_file_is_rejected = app
        .db()
        .read(move |conn| {
            Ok(if preview_missing {
                checked_storage
                    .existing_preview_file(conn, &checked_source)
                    .is_err()
            } else {
                checked_storage
                    .existing_variant_file(conn, &checked_preview, &checked_variation)
                    .is_err()
            })
        })
        .await
        .unwrap();
    assert!(
        missing_file_is_rejected,
        "file-aware processing helpers must still reject missing files"
    );
    let result = crate::active_storage::processed_representation(&app.booted.app, blob, variation)
        .await
        .unwrap();
    assert_eq!(
        result.id, image.id,
        "Rails processed? reuses the record; serving handles a missing final file"
    );
    let reply = app
        .anonymous()
        .send(Req::new(Method::GET, &representation))
        .await;
    assert_eq!(reply.status.as_u16(), 302);
    assert!(reply.header("location").is_some());
    assert_eq!(super::agent_review_tests::snapshot(&app).await, before);
    assert_eq!(super::agent_review_tests::stored_files(&app), files);
    println!(
        "PR192_R4_MISSING_FILE preview_missing={preview_missing} processed_metadata=reused file_helper=clean_error redirect=302 rows/files/jobs=unchanged"
    );
}
#[tokio::test]
async fn pr192_r4_video_oracle_preserves_attachment_targets_and_exact_row_counts() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_review192r3_attachment.json"
    ))
    .unwrap();
    let expected = vector["cases"][0]["state"]["attachments"]
        .as_array()
        .unwrap();
    for row in expected {
        assert!(
            row["record_id"].is_i64(),
            "oracle omits attachment target: {row}"
        );
        assert!(row["record_type"].is_string());
    }
    let app = closed_thread().await;
    let source = fresh_source(&app, 9).await;
    let count = |conn: &rusqlite::Connection| -> campfire_db::Result<[i64; 3]> {
        Ok([
            conn.query_row("SELECT count(*) FROM active_storage_blobs", [], |r| {
                r.get(0)
            })?,
            conn.query_row("SELECT count(*) FROM active_storage_attachments", [], |r| {
                r.get(0)
            })?,
            conn.query_row(
                "SELECT count(*) FROM active_storage_variant_records",
                [],
                |r| r.get(0),
            )?,
        ])
    };
    let before = app.db().read(count).await.unwrap();
    let response = app
        .anonymous()
        .send(post(&app, source, "pr192-r3-fresh-video"))
        .await;
    assert_eq!(response.status.as_u16(), 201, "{}", response.text());
    let message = response.json()["id"].as_i64().unwrap();
    let (preview, image) = media(&app, source).await;
    assert_file(&app, &preview);
    assert_file(&app, &image);
    let (preview_id, image_id) = (preview.id, image.id);
    let expected = expected.clone();
    app.db().read(move |conn| {
        let after = count(conn)?;
        assert_eq!(after, [before[0] + 2, before[1] + 3, before[2] + 1]);
        let variant = conn.query_row("SELECT id FROM active_storage_variant_records WHERE blob_id=?", [preview_id], |r| r.get::<_, i64>(0))?;
        for (name, record_type, record, blob) in [("attachment", "Message", message, source), ("preview_image", "ActiveStorage::Blob", source, preview_id), ("image", "ActiveStorage::VariantRecord", variant, image_id)] {
            assert_eq!(conn.query_row("SELECT count(*) FROM active_storage_attachments WHERE name=? AND record_type=? AND record_id=? AND blob_id=?", rusqlite::params![name, record_type, record, blob], |r| r.get::<_, i64>(0))?, 1);
        }
        for row in expected {
            let sql = format!("SELECT * FROM active_storage_attachments WHERE blob_id={} AND name='{}' AND record_type='{}'", row["blob_id"], row["name"].as_str().unwrap(), row["record_type"].as_str().unwrap());
            let actual = super::agent_review_r2_tests::row_state(conn, &sql, &row)?;
            assert_eq!(actual, row);
            let mut wrong_target = row.clone();
            wrong_target["record_id"] = json!(row["record_id"].as_i64().unwrap() + 1);
            assert_ne!(actual, wrong_target, "a changed attachment target must fail the oracle");
        }
        Ok(())
    }).await.unwrap();
    println!(
        "PR192_R4_RELATIONS blobs=+2 attachments=+3 variants=+1 three_exact_foreign_keys=true target_injection=detected"
    );
}
#[tokio::test]
async fn pr192_r4_late_media_failures_preserve_posts_but_precommit_job_failure_rolls_back() {
    for trigger in [
        "CREATE TRIGGER pr192_r4_fault BEFORE INSERT ON active_storage_attachments WHEN NEW.name='preview_image' BEGIN SELECT RAISE(ABORT,'preview attachment failure'); END",
        "CREATE TRIGGER pr192_r4_fault AFTER INSERT ON active_storage_attachments WHEN NEW.name='image' AND NEW.record_type='ActiveStorage::VariantRecord' BEGIN SELECT RAISE(ABORT,'variant attachment failure'); END",
        "CREATE TRIGGER pr192_r4_fault BEFORE INSERT ON background_jobs WHEN NEW.job_class='ChannelThread::PushMessageJob' BEGIN SELECT RAISE(ABORT,'thread push failure'); END",
    ] {
        let app = closed_thread().await;
        let source = fresh_source(&app, 9).await;
        app.db()
            .write(move |tx| {
                tx.conn().execute_batch(trigger)?;
                Ok(())
            })
            .await
            .unwrap();
        let rows = super::agent_review_tests::snapshot(&app).await;
        let files = super::agent_review_tests::stored_files(&app);
        let reply = app.anonymous().send(post(&app, source, "pr192-r4-precommit")).await;
        if trigger.contains("PushMessageJob") {
            assert_eq!(reply.status.as_u16(), 500);
            assert_eq!(super::agent_review_tests::snapshot(&app).await, rows);
            assert_eq!(super::agent_review_tests::stored_files(&app), files);
        } else {
            assert_eq!(reply.status.as_u16(), 201);
            assert!(super::messages::attachment_processing_tests::perform_queued(&app, 1).await.is_err());
            let retained = super::agent_review_tests::stored_files(&app);
            assert_eq!(retained.len(), files.len() + usize::from(trigger.contains("variant attachment")));
            let storage = app.booted.app.storage.clone();
            app.db().read(move |c| {
                assert!(campfire_db::Message::find_duplicate(c, 486777696, 394959859, "pr192-r4-precommit")?.is_some());
                assert!(campfire_db::ChannelThread::find(c, 1900700030)?.closed_at.is_none());
                assert_eq!(c.query_row("SELECT count(*) FROM active_storage_variant_records WHERE blob_id=? OR blob_id IN (SELECT blob_id FROM active_storage_attachments WHERE record_type='ActiveStorage::Blob' AND record_id=? AND name='preview_image')", [source, source], |r| r.get::<_,i64>(0))?, 0);
                let blob = Blob::find(c, source).unwrap().unwrap();
                assert!(storage.service.exist(&blob.key));
                Ok(())
            }).await.unwrap();
        }
    }
    println!(
        "PR192_R4_BOUNDARIES deferred_media_errors=committed_posts precommit_push_error=rollback"
    );
}

#[tokio::test]
async fn pr192_r4_approved_jpeg_metadata_reuse_does_not_serve_missing_files() {
    let app = closed_thread().await;
    let source = fresh_source(&app, 1).await;
    for (client, status) in [("pr192-r4-jpeg-first", 201), ("pr192-r4-jpeg-reuse", 422)] {
        let reply = app.anonymous().send(post(&app, source, client)).await;
        assert_eq!(reply.status.as_u16(), status, "{}", reply.text());
    }
    let blob = app
        .db()
        .read(move |c| Ok(Blob::find(c, source).unwrap().unwrap()))
        .await
        .unwrap();
    let variation = Variation::resize_to_limit(1200, 800, None);
    let proxy = campfire_storage::paths::representation_proxy_path(
        &*app.booted.app.storage.verifier,
        &blob,
        &variation,
    );
    let result =
        crate::active_storage::processed_representation(&app.booted.app, blob, variation).await;
    let image = result.unwrap();
    assert!(app.booted.app.storage.service.exist(&image.key));
    app.booted.app.storage.service.delete(&image.key).unwrap();
    let response = app.anonymous().send(Req::new(Method::GET, &proxy)).await;
    assert_eq!(response.status.as_u16(), 404);
    assert!(response.body.is_empty());
    let counts = app
        .db()
        .read(move |c| {
            Ok((
                c.query_row(
                    "SELECT count(*) FROM messages WHERE client_message_id LIKE 'pr192-r4-jpeg-%'",
                    [],
                    |r| r.get::<_, i64>(0),
                )?,
                c.query_row(
                    "SELECT count(*) FROM active_storage_variant_records WHERE blob_id=?",
                    [source],
                    |r| r.get::<_, i64>(0),
                )?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(counts, (1, 1));
    println!(
        "PR192_R4_JPEG reuse_statuses=201/422 messages=1 variants=1 missing_file_serving=empty_404"
    );
}
