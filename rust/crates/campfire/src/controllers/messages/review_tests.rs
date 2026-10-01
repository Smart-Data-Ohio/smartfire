use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, ThreadMembership};
use campfire_kit::clock::FrozenClock;
use serde_json::{Value, json};
use std::sync::Arc;

fn oracle_row(name: &str) -> Value {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/thread-review.json"
    ))
    .unwrap();
    oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap()
        .clone()
}

fn assert_response(response: &Reply, row: &Value) {
    assert_eq!(
        response.status.as_u16(),
        row["status"].as_u64().unwrap() as u16
    );
    assert_eq!(response.content_type(), row["content_type"].as_str());
    assert_eq!(
        response.header("cache-control"),
        row["cache_control"].as_str()
    );
    assert_eq!(response.location(), row["location"].as_str());
    if response.text() != row["body"].as_str().unwrap() {
        rails_mismatch(
            &response.text(),
            row["body"].as_str().unwrap(),
            row["name"].as_str().unwrap(),
        );
    }
}

async fn app() -> TestApp {
    TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap())))
        .await
        .unwrap()
}
async fn thread(app: &TestApp) -> i64 {
    app.db()
        .write(|tx| {
            ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: ALL_TALK,
                    creator_id: JASON,
                    name: Some("Review".into()),
                    ..Default::default()
                },
            )
            .map(|t| t.id)
        })
        .await
        .unwrap()
}
fn post(id: i64, body: Value) -> Req {
    Req::new(
        Method::POST,
        &format!("/rooms/{ALL_TALK}/threads/{id}/messages.json"),
    )
    .header("content-type", "application/json")
    .header("accept", "application/json")
    .body(body.to_string())
}
#[tokio::test]
async fn review_failed_thread_upload_rolls_back_like_rails() {
    let app = app().await;
    let id = thread(&app).await;
    app.db()
        .write(move |tx| ChannelThread::find(tx.conn(), id)?.close(tx))
        .await
        .unwrap();
    let blob = app
        .db()
        .read(|c| Ok(campfire_storage::Blob::find(c, 1).unwrap().unwrap()))
        .await
        .unwrap();
    app.booted.app.storage.service.delete(&blob.key).unwrap();
    let signed =
        campfire_storage::paths::signed_blob_id(&*app.booted.app.storage.verifier, 1, None);
    let rows = row_snapshot(&app).await;
    let files = file_snapshot(app.booted.app.storage.service.root());
    let reply = app
        .david()
        .write(post(
            id,
            json!({"message":{"client_message_id":"review-missing-file","attachment":signed}}),
        ))
        .await;
    let state = app
        .db()
        .read(move |c| {
            Ok((
                Message::in_thread(c, id)?.len(),
                ThreadMembership::find_by_thread_and_user(c, id, DAVID)?.is_some(),
                ChannelThread::find(c, id)?.closed_at.is_some(),
            ))
        })
        .await
        .unwrap();
    println!(
        "WS8bmr Rust missing-file: status {}; (messages, joined, closed) {:?}",
        reply.status, state
    );
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        state,
        (0, false, true),
        "Rails rolls back failed thread post, join, and reopen"
    );
    assert_eq!(row_snapshot(&app).await, rows);
    assert_eq!(file_snapshot(app.booted.app.storage.service.root()), files);
    assert_response(&reply, &oracle_row("missing_file")["responses"][0]);
}

// Request record tables; the running retention worker may claim/finish its own queue
// row during media processing. Durable request enqueue rollback has separate HTTP
// trigger regressions, so unrelated consumer state is not a request invariant.
async fn row_snapshot(app: &TestApp) -> Vec<(String, Vec<Vec<String>>)> {
    app.db()
        .read(|conn| {
            [
                "rooms",
                "memberships",
                "channel_threads",
                "thread_memberships",
                "messages",
                "action_text_rich_texts",
                "active_storage_attachments",
                "active_storage_blobs",
                "active_storage_variant_records",
            ]
            .into_iter()
            .map(|table| {
                let mut statement =
                    conn.prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))?;
                let columns = statement.column_count();
                let rows = statement
                    .query_map([], |row| {
                        (0..columns)
                            .map(|i| row.get_ref(i).map(|v| format!("{v:?}")))
                            .collect::<rusqlite::Result<Vec<_>>>()
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok((table.to_owned(), rows))
            })
            .collect()
        })
        .await
        .unwrap()
}

fn file_snapshot(
    root: &std::path::Path,
) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn visit(
        root: &std::path::Path,
        path: &std::path::Path,
        files: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), files);
            } else {
                files.insert(
                    entry.path().strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut files = Default::default();
    visit(root, root, &mut files);
    files
}

#[tokio::test]
async fn thread_media_failure_rolls_back_uploaded_original_variants_and_all_rows() {
    for initial in [false, true] {
        let app = app().await;
        let id = thread(&app).await;
        app.db().write(move |tx| {
            ChannelThread::find(tx.conn(), id)?.close(tx)?;
            tx.conn().execute_batch("CREATE TRIGGER ws8bm_reject_variant BEFORE INSERT ON active_storage_variant_records BEGIN SELECT RAISE(ABORT, 'thread variant persistence failed'); END;")?;
            Ok(())
        }).await.unwrap();
        let source = app
            .db()
            .read(|conn| Ok(campfire_storage::Blob::find(conn, 1).unwrap().unwrap()))
            .await
            .unwrap();
        let image = app
            .booted
            .app
            .storage
            .service
            .download(&source.key)
            .unwrap();
        let mut browser = app.david();
        let rows = row_snapshot(&app).await;
        let files = file_snapshot(app.booted.app.storage.service.root());
        let path = if initial {
            format!("/rooms/{ALL_TALK}/threads.json")
        } else {
            format!("/rooms/{ALL_TALK}/threads/{id}/messages.json")
        };
        let response = browser
            .write(Req::new(Method::POST, &path).multipart(
                &[
                    ("thread[name]", "Failed image"),
                    ("message[client_message_id]", "failed-variant"),
                ],
                ("message[attachment]", "upload.jpg", "image/jpeg", &image),
            ))
            .await;
        assert_eq!(
            response.status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "initial={initial}"
        );
        assert_eq!(
            row_snapshot(&app).await,
            rows,
            "initial={initial}: every database row must roll back"
        );
        assert_eq!(
            file_snapshot(app.booted.app.storage.service.root()),
            files,
            "initial={initial}: generated files must roll back, existing files must survive"
        );
    }
}
#[tokio::test]
async fn review_boolean_client_retry_matches_rails_one_row() {
    let app = app().await;
    let id = thread(&app).await;
    let mut browser = app.david();
    let body = json!({"message":{"client_message_id":true,"markdown_source":"retry me"}});
    let a = browser.write(post(id, body.clone())).await;
    let b = browser.write(post(id, body)).await;
    let ids = app
        .db()
        .read(move |c| {
            Ok(Message::in_thread(c, id)?
                .into_iter()
                .map(|m| (m.id, m.client_message_id))
                .collect::<Vec<_>>())
        })
        .await
        .unwrap();
    println!(
        "WS8bmr Rust boolean retry: statuses {} {}; rows {:?}",
        a.status, b.status, ids
    );
    assert_eq!(a.status, StatusCode::CREATED);
    assert_eq!(b.status, StatusCode::CREATED);
    assert_eq!(
        ids.len(),
        1,
        "Rails writes one message on a true client ID retry"
    );
}
#[tokio::test]
async fn review_signed_initial_thread_attachment_matches_rails() {
    let app = app().await;
    let signed =
        campfire_storage::paths::signed_blob_id(&*app.booted.app.storage.verifier, 13, None);
    let response=app.david().write(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/threads.json")).header("content-type","application/json").header("accept","application/json").body(json!({"thread":{"name":"Signed initial"},"message":{"client_message_id":"review-initial","attachment":signed}}).to_string())).await;
    println!("WS8bmr Rust signed initial: {}", response.status);
    assert_eq!(response.status, StatusCode::CREATED);
    let expected = oracle_row("signed_initial");
    assert_response(&response, &expected["responses"][0]);
    let state = app
        .db()
        .read(|conn| {
            let message =
                Message::find_duplicate(conn, ALL_TALK, DAVID, "review-initial")?.unwrap();
            let blob = campfire_storage::Blob::attached(conn, "Message", message.id, "attachment")
                .unwrap()
                .unwrap();
            assert!(blob.is_analyzed());
            assert_eq!(
                Message::in_thread(conn, message.thread_id.unwrap())?.len(),
                1
            );
            Ok(blob)
        })
        .await
        .unwrap();
    assert_eq!(state.id, expected["blob_id"]);
    assert!(
        !app.booted
            .app
            .storage
            .service
            .download(&state.key)
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn scalar_retry_paths_match_rails_bytes_and_rows() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/client-retries.json"
    ))
    .unwrap();
    let mut requests = 0;
    for row in oracle["rows"].as_array().unwrap() {
        let app = app().await;
        let (last_message, last_thread) = app
            .db()
            .read(|conn| {
                Ok((
                    conn.query_row("SELECT MAX(id) FROM messages", [], |r| r.get::<_, i64>(0))?,
                    conn.query_row("SELECT MAX(id) FROM channel_threads", [], |r| {
                        r.get::<_, i64>(0)
                    })?,
                ))
            })
            .await
            .unwrap();
        if row["kind"] == "reply" {
            thread(&app).await;
        }
        let mut browser = app.david();
        for expected in row["responses"].as_array().unwrap() {
            let response = browser
                .write(
                    Req::new(Method::POST, expected["path"].as_str().unwrap())
                        .header("content-type", "application/json")
                        .header("accept", "application/json")
                        .body(expected["input"].to_string()),
                )
                .await;
            let mut expected = expected.clone();
            expected["name"] = row["name"].clone();
            assert_response(&response, &expected);
            requests += 1;
        }
        let state = app
            .db()
            .read(move |conn| {
                let clients = conn
                    .prepare("SELECT client_message_id FROM messages WHERE id > ? ORDER BY id")?
                    .query_map([last_message], |r| r.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                let count = conn.query_row(
                    "SELECT COUNT(*) FROM channel_threads WHERE id > ?",
                    [last_thread],
                    |r| r.get::<_, i64>(0),
                )?;
                Ok(json!({"clients": clients, "thread_count": count}))
            })
            .await
            .unwrap();
        let name = row["name"].as_str().unwrap();
        assert_eq!(state["clients"], row["clients"], "{name}");
        assert_eq!(state["thread_count"], row["thread_count"], "{name}");
    }
    assert_eq!(requests, 48);
    println!(
        "WS8bm client retries: 24 scenarios; 48 Rails HTTP responses byte-identical; scalar IDs and saved rows checked"
    );
}

#[tokio::test]
async fn initial_jpeg_after_commit_matches_rails() {
    let app = app().await;
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/thread-upload-coverage.json"
    ))
    .unwrap();
    let row = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "top_image")
        .unwrap();
    let input = &row["responses"][0];
    let before = app
        .db()
        .read(|conn| {
            Ok((
                conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?,
                conn.query_row("SELECT COUNT(*) FROM channel_threads", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            ))
        })
        .await
        .unwrap();
    let response = app
        .david()
        .write(
            Req::new(Method::POST, input["path"].as_str().unwrap())
                .header("content-type", "application/json")
                .body(input["input"].to_string()),
        )
        .await;
    let after = app
        .db()
        .read(|conn| {
            Ok((
                conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?,
                conn.query_row("SELECT COUNT(*) FROM channel_threads", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            ))
        })
        .await
        .unwrap();
    println!(
        "WS8bm merged JPEG: Rust {}; committed messages/threads {:?}; Rails {} / (1, 1)",
        response.status,
        (after.0 - before.0, after.1 - before.1),
        input["status"]
    );
    assert_eq!((after.0 - before.0, after.1 - before.1), (1, 1));
    assert_response(&response, input);
}

#[tokio::test]
async fn human_attachment_edits_enqueue_atomically_on_roots_and_threads() {
    use base64::Engine as _;
    for in_thread in [false, true] {
        for signed in [false, true] {
            let app = app().await;
            let thread_id = if in_thread {
                Some(thread(&app).await)
            } else {
                None
            };
            let message = app
                .db()
                .write(move |tx| {
                    Message::create(
                        tx,
                        campfire_db::NewMessage {
                            room_id: ALL_TALK,
                            creator_id: DAVID,
                            thread_id,
                            markdown_source: Some("Preserve this edit".into()),
                            ..Default::default()
                        },
                    )
                })
                .await
                .unwrap();
            let vectors: Value = serde_json::from_str(include_str!(
                "../../../../../vectors/attachment_assignments.json"
            ))
            .unwrap();
            let png = base64::engine::general_purpose::STANDARD
                .decode(vectors["png_base64"].as_str().unwrap())
                .unwrap();
            let path = if let Some(id) = thread_id {
                format!("/rooms/{ALL_TALK}/threads/{id}/messages/{}", message.id)
            } else {
                format!("/rooms/{ALL_TALK}/messages/{}", message.id)
            };
            let request = Req::new(Method::PATCH, &path);
            let request = if signed {
                let staged = app
                    .booted
                    .app
                    .storage
                    .stage_bytes(
                        &png,
                        campfire_storage::Filename::new("edit.png"),
                        Some("image/png"),
                    )
                    .unwrap();
                let blob = app
                    .db()
                    .write(move |tx| super::save_staged(tx, staged))
                    .await
                    .unwrap();
                let capability = campfire_storage::paths::signed_blob_id(
                    &*app.booted.app.storage.verifier,
                    blob.id,
                    None,
                );
                request.form(&[
                    ("message[markdown_source]", "Reject this edit"),
                    ("message[attachment]", &capability),
                ])
            } else {
                request.multipart(
                    &[("message[markdown_source]", "Reject this edit")],
                    ("message[attachment]", "edit.png", "image/png", &png),
                )
            };
            app.db().write(|tx| {
                tx.conn().execute_batch("CREATE TRIGGER reject_message_edit_analysis BEFORE INSERT ON background_jobs WHEN NEW.job_class='ActiveStorage::AnalyzeJob' BEGIN SELECT RAISE(ABORT,'edit analysis must commit atomically'); END;")?;
                Ok(())
            }).await.unwrap();
            let rows = row_snapshot(&app).await;
            let files = file_snapshot(app.booted.app.storage.service.root());
            let response = app.david().write(request).await;
            assert_eq!(
                response.status,
                StatusCode::INTERNAL_SERVER_ERROR,
                "thread={in_thread} signed={signed}"
            );
            assert_eq!(
                row_snapshot(&app).await,
                rows,
                "thread={in_thread} signed={signed}"
            );
            assert_eq!(
                file_snapshot(app.booted.app.storage.service.root()),
                files,
                "thread={in_thread} signed={signed}"
            );
        }
    }
    println!(
        "WS8bm durable edit: 4 real HTTP enqueue failures; root/thread and signed/multipart; all request rows/files rolled back"
    );
}

#[tokio::test]
async fn initial_attachment_capability_and_media_matrix_matches_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/thread-upload-coverage.json"
    ))
    .unwrap();
    let mut requests = 0;
    for row in oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "upload")
    {
        let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(
            row["now"].as_str().unwrap().parse().unwrap(),
        )))
        .await
        .unwrap();
        let before = app
            .db()
            .read(|c| {
                Ok((
                    c.query_row("SELECT MAX(id) FROM messages", [], |r| r.get::<_, i64>(0))?,
                    c.query_row("SELECT MAX(id) FROM channel_threads", [], |r| {
                        r.get::<_, i64>(0)
                    })?,
                ))
            })
            .await
            .unwrap();
        if let Some(id) = row["delete_blob_file"].as_i64() {
            let blob = app
                .db()
                .read(move |c| Ok(campfire_storage::Blob::find(c, id).unwrap().unwrap()))
                .await
                .unwrap();
            app.booted.app.storage.service.delete(&blob.key).unwrap();
        }
        let input = &row["responses"][0];
        let response = app
            .david()
            .write(
                Req::new(Method::POST, input["path"].as_str().unwrap())
                    .header("content-type", "application/json")
                    .body(input["input"].to_string()),
            )
            .await;
        println!(
            "WS8bm initial attachment {}: Rust {} / Rails {}",
            row["name"], response.status, input["status"]
        );
        assert_response(&response, input);
        let state=app.db().read(move |c|{
            let clients=c.prepare("SELECT client_message_id FROM messages WHERE id>? ORDER BY id")?.query_map([before.0],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let threads=c.query_row("SELECT COUNT(*) FROM channel_threads WHERE id>?",[before.1],|r|r.get::<_,i64>(0))?;
            let attachment=c.query_row("SELECT id FROM messages WHERE id>? ORDER BY id LIMIT 1",[before.0],|r|r.get::<_,i64>(0)).ok().map(|id|{
                let blob=campfire_storage::Blob::attached(c,"Message",id,"attachment").unwrap().unwrap();
                let note=Message::find(c,id)?.forward_note;
                Ok::<_,campfire_db::Error>((json!({"id":blob.id,"metadata":serde_json::from_str::<Value>(&blob.metadata.encode()).unwrap()}),note))
            }).transpose()?;
            Ok((clients,threads,attachment))
        }).await.unwrap();
        assert_eq!(json!(state.0), row["clients"]);
        assert_eq!(state.1, row["thread_count"].as_i64().unwrap());
        if let Some((attachment, note)) = state.2 {
            assert_eq!(attachment, row["attachment"]);
            assert_eq!(json!(note), row["forward_note"]);
        } else {
            assert!(row["attachment"].is_null());
        }
        requests += 1;
    }
    assert_eq!(requests, 18);
    println!(
        "WS8bm initial attachments: 18 Rails requests byte-identical; top/nested; file/JPEG/video/BMP; invalid capabilities; committed and rolled-back rows"
    );
}
