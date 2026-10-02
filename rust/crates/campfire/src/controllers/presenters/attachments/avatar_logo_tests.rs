use super::*;
use crate::controllers::presenters::test_support::*;
use axum::http::Method;
use base64::Engine as _;
use campfire_kit::clock::FrozenClock;
use serde_json::Value;

#[tokio::test]
async fn avatar_bot_logo_uploads_match_pinned_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/messaging/avatar-logo-uploads.json"
    ))
    .unwrap();
    for expected in oracle["rows"].as_array().unwrap() {
        println!(
            "WS8bm avatar case: {} {} {}",
            expected["kind"], expected["mode"], expected["input"]["kind"]
        );
        let app =
            TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap())))
                .await
                .unwrap();
        app.db().write(|tx| {
            tx.conn().execute_batch("CREATE TRIGGER hold_avatar_analysis AFTER INSERT ON background_jobs WHEN NEW.job_class = 'ActiveStorage::AnalyzeJob' BEGIN UPDATE background_jobs SET run_at='2099-01-01 00:00:00' WHERE id=NEW.id; END;")?;
            Ok(())
        }).await.unwrap();
        let input = &expected["input"];
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(input["data_base64"].as_str().unwrap())
            .unwrap();
        let filename = input["filename"].as_str().unwrap();
        let mime = input["content_type"].as_str().unwrap();
        let field = if expected["kind"] == "logo" {
            "account[logo]"
        } else {
            "user[avatar]"
        };
        let name_field = if expected["kind"] == "logo" {
            "account[name]"
        } else {
            "user[name]"
        };
        let request = Req::new(Method::PATCH, expected["path"].as_str().unwrap());
        let request = if expected["mode"] == "multipart" {
            request.multipart(
                &[(name_field, "Attachment parity")],
                (field, filename, mime, &bytes),
            )
        } else {
            let staged = app
                .booted
                .app
                .storage
                .stage_bytes(&bytes, Filename::new(filename), Some(mime))
                .unwrap();
            let declared = mime.to_owned();
            let blob = app
                .db()
                .write(move |tx| {
                    let mut blob = crate::controllers::messages::save_staged(tx, staged)?;
                    tx.conn().execute(
                        "UPDATE active_storage_blobs SET content_type=?1,metadata='{}' WHERE id=?2",
                        rusqlite::params![declared, blob.id],
                    )?;
                    blob.content_type = Some(declared);
                    blob.metadata = campfire_storage::Json::object();
                    Ok(blob)
                })
                .await
                .unwrap();
            let signed = campfire_storage::paths::signed_blob_id(
                &*app.booted.app.storage.verifier,
                blob.id,
                None,
            );
            request.form(&[(name_field, "Attachment parity"), (field, &signed)])
        };
        let response = app.david().write(request).await;
        let expected_response = &expected["response"];
        assert_eq!(
            response.status.as_u16(),
            expected_response["status"].as_u64().unwrap() as u16,
            "{expected}"
        );
        assert_eq!(response.text(), expected_response["body"].as_str().unwrap());
        assert_eq!(
            response.content_type(),
            expected_response["content_type"].as_str()
        );
        assert_eq!(
            response.header("cache-control"),
            expected_response["cache_control"].as_str()
        );
        assert_eq!(response.location(), expected_response["location"].as_str());
        let kind = expected["kind"].as_str().unwrap().to_owned();
        let snapshot=app.db().read(move |conn| {
            let (record,name) = match kind.as_str() {
                "avatar" => (Record::user(DAVID),"avatar"),
                "bot" => (Record::user(BENDER),"avatar"),
                _ => (Record::account(campfire_db::Account::first(conn)?.unwrap().id),"logo"),
            };
            let record_name:String=conn.query_row(&format!("SELECT name FROM {} WHERE id=?",record.table),[record.id],|r|r.get(0))?;
            let blob=attached_blob(conn,record.record_type,record.id,name)?.unwrap_or_else(|| panic!("missing {kind} attachment; name={record_name}"));
            let jobs:i64=conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='ActiveStorage::AnalyzeJob'",[],|r|r.get(0))?;
            Ok((blob,record_name,jobs))
        }).await.unwrap();
        assert_eq!(snapshot.1, expected["record_name"]);
        assert_eq!(snapshot.2, expected["analysis_jobs"].as_i64().unwrap());
        assert_eq!(snapshot.0.filename.raw(), expected["raw_filename"]);
        assert_eq!(snapshot.0.filename.sanitized(), expected["filename"]);
        assert_eq!(snapshot.0.content_type(), expected["content_type"]);
        assert_eq!(
            serde_json::from_str::<Value>(&snapshot.0.metadata.encode()).unwrap(),
            expected["metadata"]
        );
        assert_eq!(
            app.booted
                .app
                .storage
                .service
                .download(&snapshot.0.key)
                .unwrap(),
            bytes
        );
    }
    println!(
        "WS8bm avatar/bot/logo: 18 Rails responses byte-identical; attachment bytes/filenames/MIME/metadata/job counts match"
    );
}
