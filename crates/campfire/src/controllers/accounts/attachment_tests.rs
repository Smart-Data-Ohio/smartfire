//! Owned icon/logo endpoints use #171's shared attachment producer. Expected rows are Rails.
use crate::controllers::presenters::{attachments, test_support::*};
use axum::http::{Method, StatusCode};
use campfire_storage::{Blob, Filename, Json};
use serde_json::Value;
fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/users_attachment_endpoints.json"
    ))
    .unwrap()
}
pub(super) async fn direct_blob(app: &TestApp, bytes: &[u8], filename: &str) -> Blob {
    let staged = app
        .booted
        .app
        .storage
        .stage_bytes(
            bytes,
            Filename::new(filename),
            Some("application/octet-stream"),
        )
        .unwrap();
    app.db().write(move |tx| {
        let mut blob = crate::controllers::messages::save_staged(tx, staged)?;
        tx.conn().execute("UPDATE active_storage_blobs SET content_type='application/octet-stream' WHERE id=?", [blob.id])?;
        blob.content_type = Some("application/octet-stream".into());
        blob.update_metadata(tx.conn(), Json::object()).map_err(attachments::storage_error)?;
        Ok(blob)
    }).await.unwrap()
}
pub(super) fn signed(app: &TestApp, blob: &Blob) -> String {
    campfire_storage::paths::signed_blob_id(&*app.booted.app.storage.verifier, blob.id, None)
}
async fn hold(app: &TestApp) {
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws8br2_hold_analysis AFTER INSERT ON background_jobs WHEN NEW.job_class='ActiveStorage::AnalyzeJob' BEGIN UPDATE background_jobs SET status='held' WHERE id=NEW.id; END;")?;
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn signed_icon_and_logo_http_assignments_match_rails_filenames_metadata_and_jobs() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    hold(&app).await;
    let mut browser = app.david();
    for row in vectors()["rows"].as_array().unwrap() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vectors/workspace_icons")
            .join(row["fixture"].as_str().unwrap());
        let blob = direct_blob(
            &app,
            &std::fs::read(fixture).unwrap(),
            row["filename"].as_str().unwrap(),
        )
        .await;
        let before = app.db().read(|c| Ok(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='ActiveStorage::AnalyzeJob'", [], |r| r.get::<_, i64>(0))?)).await.unwrap();
        let before_audits = app
            .db()
            .read(|c| {
                Ok(c.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| {
                    r.get::<_, i64>(0)
                })?)
            })
            .await
            .unwrap();
        let signature = signed(&app, &blob);
        let logo = row["kind"] == "logo";
        let reply = if logo {
            browser
                .write(Req::new(Method::PATCH, "/account").form(&[("account[logo]", &signature)]))
                .await
        } else {
            browser
                .write(Req::new(Method::POST, "/account/icons").form(&[
                    ("workspace_icon[name]", row["name"].as_str().unwrap()),
                    ("workspace_icon[title]", "Signed blob"),
                    ("workspace_icon[image]", &signature),
                ]))
                .await
        };
        assert_eq!(
            reply.status.as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{}: {}",
            row["name"],
            reply.text()
        );
        assert_eq!(reply.location(), row["location"].as_str());
        let row = row.clone();
        app.db().read(move |c| {
            let record = if logo { campfire_db::Account::first(c)?.unwrap().id } else { c.query_row("SELECT id FROM workspace_icons WHERE name=?", [row["name"].as_str().unwrap()], |r| r.get(0))? };
            let attached = attachments::attached_blob(c, if logo {"Account"} else {"WorkspaceIcon"}, record, if logo {"logo"} else {"image"})?.unwrap();
            assert_eq!(attached.id, blob.id, "reuse the signed blob, never copy it");
            assert_eq!(attached.filename.raw(), row["raw_filename"]);
            assert_eq!(attached.filename.sanitized(), row["sanitized"]);
            assert_eq!(attached.content_type(), row["content_type"]);
            let mut expected_metadata = row["metadata"].clone();
            if logo {
                expected_metadata[campfire_storage::branding::METADATA_KEY] = serde_json::json!(true);
            }
            assert_eq!(serde_json::from_str::<Value>(&attached.metadata.encode()).unwrap(), expected_metadata);
            let jobs = c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='ActiveStorage::AnalyzeJob'", [], |r| r.get::<_, i64>(0))?;
            assert_eq!(jobs - before, row["analysis_jobs"].as_i64().unwrap());
            let audits = c.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| r.get::<_, i64>(0))?;
            assert_eq!(audits - before_audits, row["audits"].as_i64().unwrap());
            Ok(())
        }).await.unwrap();
    }
}
#[tokio::test]
async fn signed_icon_analysis_enqueue_failure_rolls_back_icon_identification_and_audit() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vectors/workspace_icons/square_64.png"),
    )
    .unwrap();
    let blob = direct_blob(&app, &bytes, "square_64.png ").await;
    let before = app
        .db()
        .read(|c| {
            Ok(c.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| {
                r.get::<_, i64>(0)
            })?)
        })
        .await
        .unwrap();
    app.db().write(|tx| { tx.conn().execute_batch("CREATE TRIGGER ws8br2_reject_analysis BEFORE INSERT ON background_jobs WHEN NEW.job_class='ActiveStorage::AnalyzeJob' BEGIN SELECT RAISE(ABORT,'deliberate analysis enqueue failure'); END;")?; Ok(()) }).await.unwrap();
    let reply = app
        .david()
        .write(Req::new(Method::POST, "/account/icons").form(&[
            ("workspace_icon[name]", "enqueue_failure"),
            ("workspace_icon[title]", "Enqueue failure"),
            ("workspace_icon[image]", &signed(&app, &blob)),
        ]))
        .await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    app.db()
        .read(move |c| {
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM workspace_icons WHERE name='enqueue_failure'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM active_storage_attachments WHERE blob_id=?",
                    [blob.id],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| r
                    .get::<_, i64>(0))?,
                before
            );
            let stored = Blob::find(c, blob.id)
                .map_err(attachments::storage_error)?
                .unwrap();
            assert!(!stored.is_identified());
            assert!(!stored.is_analyzed());
            assert_eq!(stored.content_type(), "application/octet-stream");
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn logo_null_analysis_and_audit_failures_match_rails_after_commit_boundaries() {
    for row in vectors()["null_analysis"].as_array().unwrap() {
        let app = TestApp::boot_frozen().await.expect("seed required");
        let blob = direct_blob(&app, b"plain logo fixture\n", "notes.txt ").await;
        let fail = row["fail_analysis"].as_bool().unwrap();
        let fail_audit = row["fail_audit"].as_bool().unwrap();
        app.db().write(move |tx| {
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            if fail {
                tx.conn().execute_batch("CREATE TRIGGER ws8br2_reject_null_analysis BEFORE UPDATE OF metadata ON active_storage_blobs WHEN json_extract(NEW.metadata,'$.analyzed')=1 BEGIN SELECT RAISE(ABORT,'deliberate null analysis failure'); END;")?;
            }
            if fail_audit {
                tx.conn().execute_batch("CREATE TRIGGER ws8br2_reject_logo_audit BEFORE INSERT ON audit_logs WHEN NEW.action='account.settings.change' BEGIN SELECT RAISE(ABORT,'deliberate audit failure'); END;")?;
            }
            Ok(())
        }).await.unwrap();
        let response = app
            .david()
            .write(
                Req::new(Method::PATCH, "/account")
                    .header("user-agent", "ws8br2-attachment-fixture")
                    .header("x-forwarded-for", "127.0.0.1")
                    .form(&[("account[logo]", &signed(&app, &blob))]),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            row["status"].as_u64().unwrap() as u16
        );
        assert_eq!(response.location(), row["location"].as_str());
        let row = row.clone();
        app.db().read(move |c| {
            let account = campfire_db::Account::first(c)?.unwrap();
            let attached = attachments::attached_blob(c, "Account", account.id, "logo")?.unwrap();
            assert_eq!(attached.id == blob.id, row["blob_reused"].as_bool().unwrap());
            assert_eq!(attached.filename.raw(), row["raw_filename"]);
            assert_eq!(attached.filename.sanitized(), row["sanitized"]);
            assert_eq!(attached.content_type(), row["content_type"]);
            assert_eq!(serde_json::from_str::<Value>(&attached.metadata.encode()).unwrap(), row["metadata"]);
            let jobs: i64 = c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='ActiveStorage::AnalyzeJob'", [], |r| r.get(0))?;
            assert_eq!(jobs, row["analysis_jobs"].as_i64().unwrap());
            let mut q = c.prepare("SELECT id FROM audit_logs ORDER BY id")?;
            let ids = q.query_map([], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let audits = ids.into_iter().map(|id| campfire_db::models::audit_log::AuditLog::find(c,id).map(|a|a.snapshot())).collect::<campfire_db::Result<Vec<_>>>()?;
            assert_eq!(audits.len(), row["audits"].as_array().unwrap().len(), "successful save callbacks reach the WS9 audit; failures preserve the logo and write no audit");
            assert_eq!(serde_json::json!(audits), row["audits"]);
            Ok(())
        }).await.unwrap();
    }
}
