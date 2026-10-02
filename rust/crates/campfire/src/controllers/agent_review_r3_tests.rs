//! PR192 R3: real fresh/reused media, transactional faults and JSON-only query preloads.
use super::agent_http_tests::{SECRET, setup};
use super::agent_review_r2_tests::row_state;
use super::presenters::test_support::{Req, TestApp};
use campfire_kit::Method;
use campfire_storage::{Blob, Variation};
use serde_json::{Value, json};

const THREAD: i64 = 1900700030;

async fn closed_thread() -> TestApp {
    let app = setup().await.without_job_runner().await;
    app.db().write(|tx| {
        tx.conn().execute_batch("UPDATE agents SET owner_id=127326141,daily_message_cap=NULL WHERE id=773018776; INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,closed_at,last_activity_at,created_at,updated_at) VALUES(1900700030,'Review',486777696,394959859,394959859,'in_progress','2026-03-01 16:00:00','2026-03-01 16:00:00','2026-03-01 16:00:00','2026-03-01 16:00:00'); DELETE FROM background_jobs;")?;
        Ok(())
    }).await.unwrap();
    app
}
async fn fresh_source(app: &TestApp, seed_id: i64) -> i64 {
    let storage = app.booted.app.storage.clone();
    app.db()
        .write(move |tx| {
            let source = Blob::find(tx.conn(), seed_id).unwrap().unwrap();
            let staged = storage
                .stage_file(
                    &storage.service.path_for(&source.key),
                    source.filename,
                    source.content_type.as_deref(),
                )
                .unwrap();
            let blob = staged.insert(tx.conn(), tx.now().jiff()).unwrap();
            crate::active_storage::keep_after_commit(tx, staged);
            Ok(blob.id)
        })
        .await
        .unwrap()
}
fn post(app: &TestApp, source: i64, client_id: &str) -> Req {
    let signed =
        campfire_storage::paths::signed_blob_id(&*app.booted.app.storage.verifier, source, None);
    Req::new(Method::POST, "/rooms/486777696/agents/messages")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("authorization", &format!("Bearer {SECRET}"))
        .body(json!({"thread_id":THREAD,"message":{"attachment":signed,"client_message_id":client_id}}).to_string())
}
async fn variant_files(app: &TestApp, source: i64) -> Vec<(i64, bool, u64)> {
    let storage = app.booted.app.storage.clone();
    app.db()
        .read(move |conn| {
            let preview = storage
                .existing_preview_image(conn, &Blob::find(conn, source).unwrap().unwrap())
                .unwrap();
            let target = preview.map_or(source, |p| p.id);
            let mut images = Vec::new();
            let records = conn
                .prepare(
                    "SELECT id FROM active_storage_variant_records WHERE blob_id=? ORDER BY id",
                )?
                .query_map([target], |r| r.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            for record in records {
                let blob = Blob::attached(conn, "ActiveStorage::VariantRecord", record, "image")
                    .unwrap()
                    .unwrap();
                let path = storage.service.path_for(&blob.key);
                images.push((
                    blob.id,
                    storage.service.exist(&blob.key),
                    std::fs::metadata(path).map_or(0, |m| m.len()),
                ));
            }
            Ok(images)
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn pr192_r3_fresh_video_retains_preview_and_variant_files() {
    let app = closed_thread().await;
    let source = fresh_source(&app, 9).await;
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_review192r3_attachment.json"
    ))
    .unwrap();
    let case = &vector["cases"][0];
    assert_eq!(case["rails"]["status"], 500);
    assert_eq!(
        case["exception"]["class"],
        "ActiveStorage::FileNotFoundError"
    );
    assert_eq!(case["exception"]["open_transactions"], 1);
    assert!(
        case["rails_state"]["row_deltas"]
            .as_object()
            .unwrap()
            .values()
            .all(|v| v == 0)
    );
    assert_eq!(case["rails_state"]["jobs"], json!([]));
    let reply = app
        .anonymous()
        .send(post(&app, source, "pr192-r3-fresh-video"))
        .await;
    assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
    let files = variant_files(&app, source).await;
    println!("PR192_R3_VIDEO status=201 variant_files={files:?}");
    assert_eq!(files.len(), 1);
    assert!(
        files.iter().all(|(_, exists, size)| *exists && *size > 0),
        "successful video must retain every recorded variant file: {files:?}"
    );
    assert_eq!(reply.text(), case["approved"]["response"].as_str().unwrap());
    for (name, value) in case["approved"]["headers"].as_object().unwrap() {
        assert_eq!(reply.header(name), value.as_str());
    }
    let state = case["state"].clone();
    let storage = app.booted.app.storage.clone();
    let actual = app.db().read(move |conn| {
        let message = row_state(conn, "SELECT * FROM messages WHERE client_message_id='pr192-r3-fresh-video'", &state["message"])?;
        let thread = row_state(conn, "SELECT * FROM channel_threads WHERE id=1900700030", &state["thread"])?;
        let blobs = state["blobs"].as_array().unwrap().iter().map(|expected| {
            let id = expected["attributes"]["id"].as_i64().unwrap();
            let attributes = row_state(conn, &format!("SELECT * FROM active_storage_blobs WHERE id={id}"), &expected["attributes"])?;
            let blob = Blob::find(conn, id).unwrap().unwrap();
            let exists = storage.service.exist(&blob.key);
            Ok(json!({"attributes":attributes,"file_exists":exists,"file_size":std::fs::metadata(storage.service.path_for(&blob.key)).unwrap().len()}))
        }).collect::<campfire_db::Result<Vec<_>>>()?;
        let attachments = state["attachments"].as_array().unwrap().iter().map(|expected| {
            let sql = format!("SELECT * FROM active_storage_attachments WHERE blob_id={} AND name='{}' AND record_type='{}'", expected["blob_id"], expected["name"].as_str().unwrap(), expected["record_type"].as_str().unwrap());
            row_state(conn, &sql, expected)
        }).collect::<campfire_db::Result<Vec<_>>>()?;
        let jobs = conn.prepare("SELECT job_class,arguments FROM background_jobs ORDER BY job_class,id")?.query_map([], |r| Ok(json!({"class":r.get::<_,String>(0)?,"args":serde_json::from_str::<Value>(&r.get::<_,String>(1)?).unwrap()})))?.collect::<Result<Vec<_>,_>>()?;
        let metadata: String = conn.query_row("SELECT metadata FROM active_storage_blobs WHERE id=?", [source], |r| r.get(0))?;
        Ok(json!({"message":message,"thread":thread,"blobs":blobs,"attachments":attachments,"jobs":jobs,"source_metadata":serde_json::from_str::<Value>(&metadata).unwrap(),"variant_count":conn.query_row("SELECT count(*) FROM active_storage_variant_records WHERE blob_id=16",[],|r|r.get::<_,i64>(0))?}))
    }).await.unwrap();
    assert_eq!(
        actual, case["state"],
        "approved video state differs from its committed-media Rails oracle"
    );
}
#[tokio::test]
async fn pr192_r3_reused_video_variant_keeps_the_same_usable_file() {
    let app = closed_thread().await;
    let source = fresh_source(&app, 9).await;
    for client in ["pr192-video-first", "pr192-video-second"] {
        let reply = app.anonymous().send(post(&app, source, client)).await;
        assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
    }
    let files = variant_files(&app, source).await;
    assert_eq!(
        files.len(),
        1,
        "reposting must reuse the original variant record"
    );
    assert!(
        files[0].1 && files[0].2 > 0,
        "reused record cannot block thumbnail generation: {files:?}"
    );
    let blob = app
        .db()
        .read(move |conn| Ok(Blob::find(conn, source).unwrap().unwrap()))
        .await
        .unwrap();
    let image = crate::active_storage::processed_representation(
        &app.booted.app,
        blob,
        Variation::format_only("webp"),
    )
    .await
    .unwrap();
    assert_eq!(image.id, files[0].0);
    assert!(app.booted.app.storage.service.exist(&image.key));
}
#[tokio::test]
async fn pr192_r3_reused_jpeg_variant_retains_its_existing_file() {
    let app = closed_thread().await;
    let source = fresh_source(&app, 1).await;
    let blob = app
        .db()
        .read(move |conn| Ok(Blob::find(conn, source).unwrap().unwrap()))
        .await
        .unwrap();
    let image = crate::active_storage::processed_representation(
        &app.booted.app,
        blob,
        Variation::resize_to_limit(1200, 800, None),
    )
    .await
    .unwrap();
    let reply = app
        .anonymous()
        .send(post(&app, source, "pr192-reused-jpeg"))
        .await;
    assert_eq!(reply.status.as_u16(), 201, "{}", reply.text());
    let files = variant_files(&app, source).await;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, image.id);
    assert!(files[0].1 && files[0].2 > 0);
}
#[tokio::test]
async fn pr192_r3_video_insert_failure_rolls_back_all_rows_and_files() {
    let app = closed_thread().await;
    let source = fresh_source(&app, 9).await;
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER pr192_video_failure BEFORE INSERT ON active_storage_variant_records BEGIN SELECT RAISE(ABORT,'injected video variant failure'); END")?;
        Ok(())
    }).await.unwrap();
    let rows = super::agent_review_tests::snapshot(&app).await;
    let files = super::agent_review_tests::stored_files(&app);
    let reply = app
        .anonymous()
        .send(post(&app, source, "pr192-video-failure"))
        .await;
    assert_eq!(reply.status.as_u16(), 500);
    assert_eq!(super::agent_review_tests::snapshot(&app).await, rows);
    assert_eq!(super::agent_review_tests::stored_files(&app), files);
    assert!(variant_files(&app, source).await.is_empty());
}
#[tokio::test]
async fn pr192_r3_mcp_payload_omits_html_cache_timestamps_at_both_sizes() {
    let app = super::agent_review_tests::readers().await;
    let mut cache_counts = Vec::new();
    for size in [5, 50] {
        let request = || {
            Req::new(Method::POST, "/agents/mcp")
            .header("accept", "application/json").header("content-type", "application/json")
            .header("authorization", &format!("Bearer {SECRET}"))
            .body(json!({"jsonrpc":"2.0","id":192,"method":"tools/call","params":{"name":"read_messages","arguments":{"room_id":486777696,"limit":size}}}).to_string())
        };
        let warm = app.anonymous().send(request()).await;
        let log = app.db().capture_read_queries();
        let reply = app.anonymous().send(request()).await;
        app.db().stop_capturing_read_queries();
        assert_eq!(reply.status.as_u16(), 200);
        assert_eq!(reply.body, warm.body);
        assert_eq!(
            reply.json()["result"]["structuredContent"]["messages"]
                .as_array()
                .unwrap()
                .len(),
            size
        );
        let queries = log.lock().unwrap();
        let cache = queries
            .iter()
            .filter(|q| q.contains("MAX(updated_at)") && q.contains("github_pull_request_threads"))
            .count();
        use sha2::{Digest, Sha256};
        let total = queries
            .iter()
            .filter(|q| q.trim_start().to_ascii_uppercase().starts_with("SELECT"))
            .count();
        println!(
            "PR192_R3_CACHE size={size} SELECTs={total} HTML_timestamp_SELECTs={cache} body_sha256={:x}",
            Sha256::digest(&reply.body)
        );
        cache_counts.push(cache);
    }
    assert_eq!(
        cache_counts,
        [0, 0],
        "JSON payloads must not read HTML-only cache stamps"
    );
}

#[tokio::test]
async fn pr192_r3_video_analysis_enqueue_failure_rolls_back_all_rows_and_files() {
    let app = closed_thread().await;
    let source = fresh_source(&app, 9).await;
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER pr192_video_queue_failure BEFORE INSERT ON background_jobs WHEN NEW.job_class='ActiveStorage::AnalyzeJob' BEGIN SELECT RAISE(ABORT,'injected video analysis queue failure'); END")?;
        Ok(())
    }).await.unwrap();
    let rows = super::agent_review_tests::snapshot(&app).await;
    let files = super::agent_review_tests::stored_files(&app);
    let reply = app
        .anonymous()
        .send(post(&app, source, "pr192-video-queue-failure"))
        .await;
    assert_eq!(
        reply.status.as_u16(),
        500,
        "successful video must enqueue its variant analysis atomically"
    );
    assert_eq!(super::agent_review_tests::snapshot(&app).await, rows);
    assert_eq!(super::agent_review_tests::stored_files(&app), files);
    assert!(variant_files(&app, source).await.is_empty());
}
