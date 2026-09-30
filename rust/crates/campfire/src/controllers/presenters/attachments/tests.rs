use axum::http::{Method, StatusCode};
use base64::Engine as _;
use campfire_storage::{Blob, Filename};
use sha2::Digest as _;

use super::*;
use crate::controllers::presenters::test_support::*;

fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/attachment_assignments.json"
    )))
    .unwrap()
}

fn png() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(vectors()["png_base64"].as_str().unwrap())
        .unwrap()
}

fn analyzer_vectors() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/attachment_analyzers.json"
    )))
    .unwrap()
}

async fn analyzer_blob(app: &TestApp, case: &serde_json::Value) -> Blob {
    let bytes = if let Some(fixture) = case["reference_fixture"].as_str() {
        std::fs::read(campfire_db::fixtures::reference_root().join(fixture)).unwrap()
    } else if let Some(fixture) = case["rust_fixture"].as_str() {
        std::fs::read(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(fixture),
        )
        .unwrap()
    } else {
        base64::engine::general_purpose::STANDARD
            .decode(case["data_base64"].as_str().unwrap())
            .unwrap()
    };
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(&bytes)),
        case["sha256"].as_str().unwrap()
    );
    let staged = app
        .booted
        .app
        .storage
        .stage_bytes(
            &bytes,
            Filename::new(case["filename"].as_str().unwrap()),
            case["content_type"].as_str(),
        )
        .unwrap();
    // A direct upload stores its row before identification; assignment identifies it.
    let declared_type = case["content_type"].as_str().map(str::to_string);
    app.db()
        .write(move |tx| {
            let mut blob = crate::controllers::messages::save_staged(tx, staged)?;
            tx.conn().execute(
                "UPDATE active_storage_blobs SET content_type = ?1 WHERE id = ?2",
                rusqlite::params![declared_type, blob.id],
            )?;
            blob.content_type = declared_type;
            blob.update_metadata(tx.conn(), campfire_storage::Json::object())
                .map_err(storage_error)?;
            Ok(blob)
        })
        .await
        .unwrap()
}

fn filename_vectors() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/attachment_filenames.json"
    )))
    .unwrap()
}

async fn filename_assignment(index: usize, reject_jobs: bool) {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    hold_analysis(&app).await;
    if reject_jobs {
        reject_analysis(&app).await;
    }
    let expected = filename_vectors()["assignments"][index].clone();
    let blob = analyzer_blob(&app, &expected).await;
    let reply = app
        .david()
        .write(Req::new(Method::PATCH, "/account").form(&[("account[logo]", &signed(&app, &blob))]))
        .await;
    assert_eq!(
        reply.status.as_u16(),
        expected["status"].as_u64().unwrap() as u16,
        "{}",
        reply.text()
    );
    let kind = expected["kind"].as_str().unwrap().to_string();
    app.db()
        .read(move |conn| {
            let account = campfire_db::Account::first(conn)?.unwrap();
            let attached = attached_blob(conn, "Account", account.id, "logo")?.unwrap();
            assert_eq!(attached.id, blob.id);
            assert_eq!(attached.filename.raw(), expected["stored_filename"]);
            assert_eq!(attached.filename.sanitized(), expected["sanitized"]);
            assert_eq!(attached.content_type(), expected["identified_content_type"]);
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&attached.metadata.encode()).unwrap(),
                expected["metadata"]
            );
            assert_eq!(
                analysis_jobs(conn)?,
                expected["analysis_jobs"].as_i64().unwrap()
            );
            Ok(())
        })
        .await
        .unwrap();
    export_analyzer_readback(&app, &kind).await;
}

#[tokio::test]
async fn signed_filename_slash_runs_null_analyzer_without_job() {
    filename_assignment(0, true).await;
}

#[tokio::test]
async fn signed_filename_space_queues_image_analyzer() {
    filename_assignment(1, false).await;
}

fn signed(app: &TestApp, blob: &Blob) -> String {
    campfire_storage::paths::signed_blob_id(&*app.booted.app.storage.verifier, blob.id, None)
}

fn analysis_jobs(conn: &Connection) -> campfire_db::Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM background_jobs WHERE job_class = 'ActiveStorage::AnalyzeJob'",
        [],
        |row| row.get(0),
    )?)
}

async fn hold_analysis(app: &TestApp) {
    // Retain real durable jobs for inspection without racing the running worker.
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER hold_analysis AFTER INSERT ON background_jobs WHEN NEW.job_class = 'ActiveStorage::AnalyzeJob' BEGIN UPDATE background_jobs SET status = 'held' WHERE id = NEW.id; END;")?;
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn null_analyzer_signed_text_ignores_rejected_jobs() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    hold_analysis(&app).await;
    let blob = analyzer_blob(&app, &analyzer_vectors()["cases"][0]).await;
    reject_analysis(&app).await;
    let reply = app
        .david()
        .write(Req::new(Method::PATCH, "/account").form(&[
            ("account[name]", "Null analyzer committed"),
            ("account[logo]", &signed(&app, &blob)),
        ]))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND, "{}", reply.text());
    app.db()
        .read(move |conn| {
            let account = campfire_db::Account::first(conn)?.unwrap();
            assert_eq!(account.name, "Null analyzer committed");
            let attached = attached_blob(conn, "Account", account.id, "logo")?.unwrap();
            assert_eq!(attached.id, blob.id);
            assert!(attached.is_identified());
            assert!(attached.is_analyzed());
            assert_eq!(analysis_jobs(conn)?, 0);
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn null_analyzer_runs_after_attachment_commit() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    hold_analysis(&app).await;
    let blob = analyzer_blob(&app, &analyzer_vectors()["cases"][0]).await;
    let id = blob.id;
    let assignment = Assignment::Signed(signed(&app, &blob))
        .stage(&app.booted.app)
        .await
        .unwrap();
    app.db()
        .write(move |tx| {
            let record = Record::account(campfire_db::Account::first(tx.conn())?.unwrap().id);
            // Runs before the analysis hook, and can already see the committed attachment.
            tx.after_commit(move |tx| {
                assert!(!tx.in_transaction());
                assert_eq!(
                    attached_blob(tx.conn(), "Account", record.id, "logo")?
                        .unwrap()
                        .id,
                    id
                );
                assert!(
                    !Blob::find(tx.conn(), id)
                        .map_err(storage_error)?
                        .unwrap()
                        .is_analyzed()
                );
                Ok(())
            });
            assign(tx, record, "logo", assignment)?;
            assert!(
                !Blob::find(tx.conn(), id)
                    .map_err(storage_error)?
                    .unwrap()
                    .is_analyzed()
            );
            assert_eq!(analysis_jobs(tx.conn())?, 0);
            Ok(())
        })
        .await
        .unwrap();
    app.db()
        .read(move |conn| {
            assert!(
                Blob::find(conn, id)
                    .map_err(storage_error)?
                    .unwrap()
                    .is_analyzed()
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn null_analyzer_failure_preserves_attachment() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    hold_analysis(&app).await;
    let blob = analyzer_blob(&app, &analyzer_vectors()["cases"][0]).await;
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_null_analysis BEFORE UPDATE OF metadata ON active_storage_blobs WHEN json_extract(NEW.metadata, '$.analyzed') = 1 BEGIN SELECT RAISE(ABORT, 'reject inline analysis'); END;")?;
        Ok(())
    }).await.unwrap();
    let reply = app
        .david()
        .write(Req::new(Method::PATCH, "/account").form(&[
            ("account[name]", "Null analyzer committed"),
            ("account[logo]", &signed(&app, &blob)),
        ]))
        .await;
    let expected = analyzer_vectors()["inline_failure"].clone();
    assert_eq!(
        reply.status.as_u16(),
        expected["status"].as_u64().unwrap() as u16,
        "{}",
        reply.text()
    );
    app.db()
        .read(move |conn| {
            let account = campfire_db::Account::first(conn)?.unwrap();
            assert_eq!(account.name, expected["account_name"].as_str().unwrap());
            let attached = attached_blob(conn, "Account", account.id, "logo")?.unwrap();
            assert_eq!(attached.id, blob.id);
            assert!(attached.is_identified());
            assert!(!attached.is_analyzed());
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&attached.metadata.encode()).unwrap(),
                expected["metadata"]
            );
            assert_eq!(
                analysis_jobs(conn)?,
                expected["analysis_jobs"].as_i64().unwrap()
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn attachment_analyzers_match_pinned_rails() {
    for case in analyzer_vectors()["cases"].as_array().unwrap() {
        let Some(app) = TestApp::boot().await else {
            return;
        };
        hold_analysis(&app).await;
        let blob = analyzer_blob(&app, case).await;
        let reply = app
            .david()
            .write(
                Req::new(Method::PATCH, "/account")
                    .form(&[("account[logo]", &signed(&app, &blob))]),
            )
            .await;
        assert_eq!(
            reply.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{}: {}",
            case["kind"],
            reply.text()
        );
        let expected = case.clone();
        app.db()
            .read(move |conn| {
                let account = campfire_db::Account::first(conn)?.unwrap();
                let attached = attached_blob(conn, "Account", account.id, "logo")?.unwrap();
                assert_eq!(attached.id, blob.id);
                assert_eq!(
                    attached.content_type(),
                    expected["identified_content_type"].as_str().unwrap()
                );
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&attached.metadata.encode()).unwrap(),
                    expected["metadata"],
                    "{}",
                    expected["kind"]
                );
                assert_eq!(
                    analysis_jobs(conn)?,
                    expected["analysis_jobs"].as_i64().unwrap(),
                    "{}",
                    expected["kind"]
                );
                assert_eq!(
                    campfire_storage::analyze::Analyzer::for_content_type(attached.content_type())
                        .analyze_later(),
                    expected["analyze_later"].as_bool().unwrap()
                );
                Ok(())
            })
            .await
            .unwrap();
        export_analyzer_readback(&app, case["kind"].as_str().unwrap()).await;
        if case["analyze_later"].as_bool().unwrap() {
            let replacement = analyzer_blob(&app, case).await;
            reject_analysis(&app).await;
            let reply = app
                .david()
                .write(Req::new(Method::PATCH, "/account").form(&[
                    ("account[name]", "must roll back"),
                    ("account[logo]", &signed(&app, &replacement)),
                ]))
                .await;
            assert_eq!(
                reply.status,
                StatusCode::INTERNAL_SERVER_ERROR,
                "{}",
                case["kind"]
            );
            app.db()
                .read(move |conn| {
                    let account = campfire_db::Account::first(conn)?.unwrap();
                    assert_ne!(account.name, "must roll back");
                    assert_eq!(
                        attached_blob(conn, "Account", account.id, "logo")?
                            .unwrap()
                            .id,
                        blob.id
                    );
                    assert!(
                        !Blob::find(conn, replacement.id)
                            .map_err(storage_error)?
                            .unwrap()
                            .is_identified()
                    );
                    assert_eq!(analysis_jobs(conn)?, 1);
                    Ok(())
                })
                .await
                .unwrap();
        }
    }
}

async fn export_analyzer_readback(app: &TestApp, kind: &str) {
    // Optional output created by this run, never an input required for the test to pass.
    let Some(output) = std::env::var_os("ATTACHMENT_ANALYZER_READBACK_DIR") else {
        return;
    };
    let output = std::path::PathBuf::from(output).join(kind);
    std::fs::create_dir_all(output.join("db")).unwrap();
    let database = output.join("db/production.sqlite3");
    app.db()
        .write(move |tx| {
            tx.after_commit(move |tx| {
                tx.conn()
                    .execute("VACUUM INTO ?1", [database.to_string_lossy().as_ref()])?;
                Ok(())
            });
            Ok(())
        })
        .await
        .unwrap();
    copy_files(
        &app.booted.app.config.storage.files,
        &output.join("storage"),
    );
}

async fn blob(app: &TestApp) -> Blob {
    let staged = app
        .booted
        .app
        .storage
        .stage_bytes(
            &png(),
            Filename::new("attachment-parity.png"),
            Some("image/png"),
        )
        .unwrap();
    app.db()
        .write(move |tx| {
            let mut blob = crate::controllers::messages::save_staged(tx, staged)?;
            blob.update_metadata(tx.conn(), campfire_storage::Json::object())
                .map_err(storage_error)?;
            Ok(blob)
        })
        .await
        .unwrap()
}

async fn reject_analysis(app: &TestApp) {
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_analysis BEFORE INSERT ON background_jobs WHEN NEW.job_class = 'ActiveStorage::AnalyzeJob' BEGIN SELECT RAISE(ABORT, 'reject analysis'); END;")?;
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn durable_analysis_logo_upload_is_atomic() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let before = app
        .db()
        .read(|conn| {
            Ok((
                campfire_db::Account::first(conn)?.unwrap().name,
                conn.query_row("SELECT count(*) FROM active_storage_blobs", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            ))
        })
        .await
        .unwrap();
    reject_analysis(&app).await;
    let reply = app
        .david()
        .write(Req::new(Method::PATCH, "/account").multipart(
            &[("account[name]", "must roll back")],
            ("account[logo]", "parity.png", "image/png", &png()),
        ))
        .await;
    assert_eq!(
        reply.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        reply.text()
    );
    let after = app
        .db()
        .read(|conn| {
            Ok((
                campfire_db::Account::first(conn)?.unwrap().name,
                conn.query_row("SELECT count(*) FROM active_storage_blobs", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(after, before);
}

#[tokio::test]
async fn durable_analysis_message_update_is_atomic() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let message = app
        .db()
        .write(|tx| {
            campfire_db::Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    body: Some("original".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let id = message.id;
    let before_body = app
        .db()
        .read(move |conn| campfire_db::Message::find(conn, id)?.body_html(conn))
        .await
        .unwrap();
    reject_analysis(&app).await;
    let reply = app
        .david()
        .write(
            Req::new(
                Method::PATCH,
                &format!("/rooms/{ALL_TALK}/messages/{}", message.id),
            )
            .multipart(
                &[("message[body]", "must roll back")],
                ("message[attachment]", "parity.png", "image/png", &png()),
            ),
        )
        .await;
    assert_eq!(
        reply.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        reply.text()
    );
    app.db()
        .read(move |conn| {
            let after = campfire_db::Message::find(conn, message.id)?;
            assert_eq!(after.updated_at, message.updated_at);
            assert_eq!(after.body_html(conn)?, before_body);
            assert!(
                Blob::attached(conn, "Message", message.id, "attachment")
                    .unwrap()
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn signed_blob_logo_assignment_matches_rails() {
    for case in vectors()["cases"].as_array().unwrap() {
        let Some(app) = TestApp::boot().await else {
            return;
        };
        let blob = blob(&app).await;
        assert_eq!(blob.id, vectors()["blob_id"].as_i64().unwrap());
        let before = app
            .db()
            .read(|conn| {
                Ok(
                    conn.query_row("SELECT count(*) FROM active_storage_blobs", [], |r| {
                        r.get::<_, i64>(0)
                    })?,
                )
            })
            .await
            .unwrap();
        let reply = app
            .david()
            .write(
                Req::new(Method::PATCH, "/account")
                    .form(&[("account[logo]", case["signed_id"].as_str().unwrap())]),
            )
            .await;
        assert_eq!(
            reply.status.as_u16(),
            case["statuses"]["logo"].as_u64().unwrap() as u16,
            "{}: {}",
            case["case"],
            reply.text()
        );
        app.db()
            .read(move |conn| {
                assert_eq!(
                    conn.query_row("SELECT count(*) FROM active_storage_blobs", [], |r| r
                        .get::<_, i64>(0))?,
                    before
                );
                Ok(())
            })
            .await
            .unwrap();
        if reply.status == StatusCode::FOUND {
            app.db()
                .read(move |conn| {
                    let account = campfire_db::Account::first(conn)?.unwrap();
                    assert_eq!(
                        attached_blob(conn, "Account", account.id, "logo")?
                            .unwrap()
                            .id,
                        blob.id
                    );
                    Ok(())
                })
                .await
                .unwrap();
        }
    }
}

#[tokio::test]
async fn durable_analysis_retry_does_not_reanalyze_or_touch() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let blob = blob(&app).await;
    crate::active_storage::analyze(&app.booted.app, blob.id)
        .await
        .unwrap();
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_reanalysis BEFORE UPDATE OF metadata ON active_storage_blobs BEGIN SELECT RAISE(ABORT, 'reanalyzed'); END;")?;
        Ok(())
    }).await.unwrap();
    crate::active_storage::analyze(&app.booted.app, blob.id)
        .await
        .unwrap();
}

#[tokio::test]
async fn signed_blob_workspace_assignment_uses_the_existing_blob() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let blob = blob(&app).await;
    let params = [(
        "image".to_string(),
        campfire_kit::Param::from(vectors()["cases"][0]["signed_id"].as_str().unwrap()),
    )]
    .into_iter()
    .collect();
    let assignment = Assignment::from_params(&params, "image")
        .unwrap()
        .stage(&app.booted.app)
        .await
        .unwrap();
    app.db().write(move |tx| {
        tx.conn().execute("INSERT INTO workspace_icons (name,title,creator_id,created_at,updated_at) VALUES ('attach_parity','Attachment parity',?1,?2,?2)", rusqlite::params![DAVID, tx.now().to_string()])?;
        let id = tx.conn().last_insert_rowid();
        assign(tx, Record { record_type: "WorkspaceIcon", table: "workspace_icons", id }, "image", assignment)?;
        assert_eq!(attached_blob(tx.conn(), "WorkspaceIcon", id, "image")?.unwrap().id, blob.id);
        Ok(())
    }).await.unwrap();
}

async fn wait_analyzed(app: &crate::app::App, id: i64) -> Blob {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let blob = app
            .db
            .read(move |conn| Blob::find(conn, id).map_err(storage_error))
            .await
            .unwrap()
            .unwrap();
        if blob.is_analyzed() {
            return blob;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "analysis did not finish"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn durable_analysis_survives_restart_and_repeated_delivery() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let app = test.booted.app.clone();
    let config = app.config.clone();
    test.booted
        .jobs
        .shutdown(std::time::Duration::from_secs(1))
        .await;
    let staged = app
        .storage
        .stage_bytes(&png(), Filename::new("restart.png"), Some("image/png"))
        .unwrap();
    let id = app
        .db
        .write(move |tx| {
            let account = campfire_db::Account::first(tx.conn())?.unwrap();
            attach(tx, Record::account(account.id), "logo", staged)?;
            Ok(attached_blob(tx.conn(), "Account", account.id, "logo")?
                .unwrap()
                .id)
        })
        .await
        .unwrap();
    let jobs = app.db.read(campfire_jobs::inspect::all).await.unwrap();
    assert_eq!(
        jobs.iter()
            .filter(|job| job.class == "ActiveStorage::AnalyzeJob")
            .count(),
        1
    );
    assert!(
        !app.db
            .read(move |conn| Ok(Blob::find(conn, id).unwrap().unwrap().is_analyzed()))
            .await
            .unwrap()
    );
    drop(app);
    drop(test.booted.router);
    drop(test.booted.app);
    let restarted = crate::app::boot_with_clock(config, seed_clock())
        .await
        .unwrap();
    let analyzed = wait_analyzed(&restarted.app, id).await;
    assert_eq!(
        analyzed.metadata.encode(),
        serde_json::to_string(&vectors()["analysis"]["metadata"]).unwrap()
    );
    restarted.app.db.write(move |tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_reanalysis BEFORE UPDATE OF metadata ON active_storage_blobs BEGIN SELECT RAISE(ABORT, 'reanalyzed'); END;")?;
        tx.emit_after_commit(Event::job(&crate::jobs::AnalyzeJob { blob_id: id }));
        tx.emit_after_commit(Event::job(&crate::jobs::AnalyzeJob { blob_id: id }));
        tx.emit_after_commit(Event::job(&crate::jobs::AnalyzeJob { blob_id: -1 }));
        Ok(())
    }).await.unwrap();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let jobs = restarted
            .app
            .db
            .read(campfire_jobs::inspect::all)
            .await
            .unwrap();
        if !jobs
            .iter()
            .any(|job| job.class == "ActiveStorage::AnalyzeJob")
        {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "duplicate jobs did not drain: {jobs:?}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    restarted
        .jobs
        .shutdown(std::time::Duration::from_secs(1))
        .await;
}

#[tokio::test]
async fn signed_blob_reassignment_preserves_rows_and_rails_can_read_them() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    let blob = blob(&app).await;
    let signed = vectors()["cases"][0]["signed_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut david = app.david();
    assert_eq!(
        david
            .write(Req::new(Method::PATCH, "/account").form(&[("account[logo]", &signed)]))
            .await
            .status,
        StatusCode::FOUND
    );
    assert_eq!(
        david
            .write(Req::new(Method::PATCH, "/users/me/profile").form(&[("user[avatar]", &signed)]))
            .await
            .status,
        StatusCode::FOUND
    );
    let message = app
        .db()
        .write(|tx| {
            campfire_db::Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    body: Some("attachment readback".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let path = format!("/rooms/{ALL_TALK}/messages/{}", message.id);
    assert_eq!(
        david
            .write(Req::new(Method::PATCH, &path).form(&[("message[attachment]", &signed)]))
            .await
            .status,
        StatusCode::FOUND
    );
    let params = [(
        "image".to_string(),
        campfire_kit::Param::from(signed.clone()),
    )]
    .into_iter()
    .collect();
    let assignment = Assignment::from_params(&params, "image")
        .unwrap()
        .stage(&app.booted.app)
        .await
        .unwrap();
    app.db().write(move |tx| {
        tx.conn().execute("INSERT INTO workspace_icons (name,title,creator_id,created_at,updated_at) VALUES ('attach_readback','Attachment parity',?1,?2,?2)", rusqlite::params![DAVID, tx.now().to_string()])?;
        let id = tx.conn().last_insert_rowid();
        assign(tx, Record { record_type: "WorkspaceIcon", table: "workspace_icons", id }, "image", assignment)
    }).await.unwrap();
    wait_analyzed(&app.booted.app, blob.id).await;
    let before = app
        .db()
        .read(move |conn| {
            campfire_storage::blob::attachment_records(conn, blob.id).map_err(storage_error)
        })
        .await
        .unwrap();
    assert_eq!(before.len(), 4);
    let ids = app
        .db()
        .read(move |conn| {
            let mut query = conn.prepare(
                "SELECT id FROM active_storage_attachments WHERE blob_id = ?1 ORDER BY id",
            )?;
            Ok(query
                .query_map([blob.id], |row| row.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
        .unwrap();
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_attachment_jobs BEFORE INSERT ON background_jobs WHEN NEW.job_class IN ('ActiveStorage::AnalyzeJob','ActiveStorage::PurgeJob') BEGIN SELECT RAISE(ABORT, 'unexpected reassignment job'); END;")?;
        Ok(())
    }).await.unwrap();
    assert_eq!(
        david
            .write(Req::new(Method::PATCH, "/account").form(&[("account[logo]", &signed)]))
            .await
            .status,
        StatusCode::FOUND
    );
    assert_eq!(
        david
            .write(Req::new(Method::PATCH, &path).form(&[("message[attachment]", &signed)]))
            .await
            .status,
        StatusCode::FOUND
    );
    app.db()
        .read(move |conn| {
            let mut query = conn.prepare(
                "SELECT id FROM active_storage_attachments WHERE blob_id = ?1 ORDER BY id",
            )?;
            assert_eq!(
                query
                    .query_map([blob.id], |row| row.get::<_, i64>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?,
                ids
            );
            Ok(())
        })
        .await
        .unwrap();
    // Optional differential artifact; this test has no dependency on an existing local path.
    if let Some(output) = std::env::var_os("ATTACHMENT_READBACK_DIR") {
        let output = std::path::PathBuf::from(output);
        std::fs::create_dir_all(output.join("db")).unwrap();
        let database = output.join("db/production.sqlite3");
        app.db()
            .write(move |tx| {
                tx.after_commit(move |tx| {
                    tx.conn()
                        .execute("VACUUM INTO ?1", [database.to_string_lossy().as_ref()])?;
                    Ok(())
                });
                Ok(())
            })
            .await
            .unwrap();
        let files = output.join("storage");
        copy_files(&app.booted.app.config.storage.files, &files);
        std::fs::write(output.join("expected.json"), serde_json::to_vec(&serde_json::json!({"blob_id": blob.id, "message_id": message.id, "metadata": vectors()["analysis"]["metadata"]})).unwrap()).unwrap();
    }
}

fn copy_files(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_files(&entry.path(), &to.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

async fn counts(app: &TestApp) -> (i64, i64, i64, i64) {
    app.db()
        .read(|conn| {
            let count = |table: &str| {
                conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                    r.get::<_, i64>(0)
                })
            };
            Ok((
                count("accounts")?,
                count("users")?,
                count("active_storage_blobs")?,
                count("active_storage_attachments")?,
            ))
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn durable_analysis_other_callers_roll_back() {
    for kind in ["profile", "bot create", "bot update", "join", "first run"] {
        let Some(app) = TestApp::boot_seed(if kind == "first run" {
            "first_run"
        } else {
            "default"
        })
        .await
        else {
            return;
        };
        let before = counts(&app).await;
        let mut browser = if matches!(kind, "join" | "first run") {
            app.anonymous()
        } else {
            app.david()
        };
        if kind == "bot create" {
            assert_eq!(
                browser
                    .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
                    .await
                    .status,
                StatusCode::FOUND
            );
        }
        reject_analysis(&app).await;
        let (method, path, fields) = match kind {
            "profile" => (
                Method::PATCH,
                "/users/me/profile".to_string(),
                vec![("user[name]", "must roll back")],
            ),
            "bot create" => (
                Method::POST,
                "/account/bots".into(),
                vec![("user[name]", "rollback bot")],
            ),
            "bot update" => (
                Method::PATCH,
                format!("/account/bots/{BENDER}"),
                vec![("user[name]", "must roll back")],
            ),
            "join" => {
                let join_code = app
                    .db()
                    .read(|conn| Ok(campfire_db::Account::first(conn)?.unwrap().join_code))
                    .await
                    .unwrap();
                (
                    Method::POST,
                    format!("/join/{join_code}"),
                    vec![
                        ("user[name]", "Rollback person"),
                        ("user[email_address]", "attachment@example.test"),
                        ("user[password]", "fixture-password"),
                    ],
                )
            }
            _ => (
                Method::POST,
                "/first_run".into(),
                vec![
                    ("user[name]", "Rollback person"),
                    ("user[email_address]", "attachment@example.test"),
                    ("user[password]", "fixture-password"),
                ],
            ),
        };
        let reply = browser
            .write(
                Req::new(method, &path)
                    .multipart(&fields, ("user[avatar]", "parity.png", "image/png", &png())),
            )
            .await;
        assert_eq!(
            reply.status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "{kind}: {}",
            reply.text()
        );
        assert_eq!(counts(&app).await, before, "{kind}");
        if matches!(kind, "profile" | "bot update") {
            let id = if kind == "profile" { DAVID } else { BENDER };
            assert_ne!(
                app.db()
                    .read(move |conn| Ok(campfire_db::User::find(conn, id)?.name))
                    .await
                    .unwrap(),
                "must roll back"
            );
        }
    }
}
