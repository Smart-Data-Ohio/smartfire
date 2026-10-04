//! HTTP upload sizes and integrity, derived from Rails through Thruster/Puma.
use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, Req, TestApp};
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/direct_upload_limits.json"
    ))
    .unwrap()
}

fn metadata(size: i64, checksum: &str, content_type: &str) -> Req {
    Req::new(Method::POST, "/rails/active_storage/direct_uploads")
        .header("content-type", "application/json")
        .body(json!({"blob":{"filename":"large.bin","byte_size":size,"checksum":checksum,"content_type":content_type}}).to_string())
}

#[tokio::test]
async fn direct_uploads_over_16_mib_match_rails_metadata_put_and_integrity() {
    let a = TestApp::boot().await.expect("default seed");
    let mut browser = a.sign_in(DAVID).await;
    for case in oracle()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["name"].as_str().unwrap().starts_with("direct_"))
    {
        let size = case["byte_size"].as_i64().unwrap();
        let content_type = case["content_type"]
            .as_str()
            .unwrap_or("application/octet-stream");
        let bytes = if let Some(fixture) = case["fixture"].as_str() {
            let mut bytes =
                std::fs::read(campfire_db::fixtures::reference_root().join(fixture)).unwrap();
            bytes.resize(size as usize, 0);
            bytes
        } else {
            vec![b'x'; size as usize]
        };
        let created = browser
            .write(metadata(
                size,
                case["checksum"].as_str().unwrap(),
                content_type,
            ))
            .await;
        assert_eq!(
            created.status.as_u16(),
            case["metadata_status"],
            "{}",
            created.text()
        );
        let blob = created.json();
        let upload = blob["direct_upload"]["url"].as_str().unwrap();
        let put = |url: &str| {
            Req::new(Method::PUT, url)
                .header("content-type", content_type)
                .header("content-length", &size.to_string())
                .body(bytes.clone())
        };
        let uploaded = browser.send(put(upload)).await;
        assert_eq!(
            uploaded.status.as_u16(),
            case["put_status"],
            "{}",
            uploaded.text()
        );
        let path = a
            .booted
            .app
            .storage
            .service
            .path_for(blob["key"].as_str().unwrap());
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            case["stored_bytes"]
        );
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        assert_eq!(blob["checksum"], case["stored_checksum"]);

        if case["bad_checksum_status"].is_null() {
            continue;
        }
        let bad = browser
            .write(metadata(size, "1B2M2Y8AsgTpgAmY7PhCfg==", content_type))
            .await
            .json();
        let rejected = browser
            .send(put(bad["direct_upload"]["url"].as_str().unwrap()))
            .await;
        assert_eq!(rejected.status.as_u16(), case["bad_checksum_status"]);
        assert_eq!(
            a.booted
                .app
                .storage
                .service
                .path_for(bad["key"].as_str().unwrap())
                .exists(),
            case["bad_checksum_file_exists"]
        );
    }
    let vectors = oracle();
    for case in vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["name"].as_str().unwrap().starts_with("metadata_"))
    {
        let created = browser
            .write(metadata(
                case["byte_size"].as_i64().unwrap(),
                "1B2M2Y8AsgTpgAmY7PhCfg==",
                "application/octet-stream",
            ))
            .await;
        assert_eq!(
            created.status.as_u16(),
            case["metadata_status"],
            "{}",
            created.text()
        );
    }
}

#[tokio::test]
async fn composer_accepts_a_100_mb_video_and_retains_the_rails_bytes() {
    let a = TestApp::boot()
        .await
        .expect("default seed")
        .without_job_runner()
        .await;
    let mut browser = a.sign_in(DAVID).await;
    let vectors = oracle();
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "composer_video")
        .unwrap();
    let mut bytes = std::fs::read(
        campfire_db::fixtures::reference_root().join(case["fixture"].as_str().unwrap()),
    )
    .unwrap();
    bytes.resize(case["byte_size"].as_u64().unwrap() as usize, 0);
    let reply = browser
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                .header("accept", "text/vnd.turbo-stream.html")
                .multipart(
                    &[],
                    (
                        "message[attachment]",
                        "large.mov",
                        "video/quicktime",
                        &bytes,
                    ),
                ),
        )
        .await;
    assert_eq!(reply.status.as_u16(), case["status"], "{}", reply.text());
    let (key, checksum): (String, String) = a.db().read(|c| {
        c.query_row("SELECT b.key,b.checksum FROM active_storage_blobs b JOIN active_storage_attachments a ON a.blob_id=b.id WHERE a.name='attachment' AND a.record_type='Message' ORDER BY a.record_id DESC LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?))).map_err(Into::into)
    }).await.unwrap();
    let path = a.booted.app.storage.service.path_for(&key);
    assert_eq!(
        std::fs::metadata(&path).unwrap().len(),
        case["stored_bytes"]
    );
    assert_eq!(checksum, case["stored_checksum"]);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    assert_eq!(reply.status, StatusCode::OK);
}
