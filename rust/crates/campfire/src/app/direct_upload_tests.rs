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
async fn direct_upload_content_length_matches_rails_with_absent_and_chunked_headers() {
    let a = TestApp::boot().await.expect("default seed");
    let mut browser = a.sign_in(DAVID).await;
    for case in oracle()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["headers"].is_object())
    {
        let created = browser
            .write(metadata(
                case["byte_size"].as_i64().unwrap(),
                case["checksum"].as_str().unwrap(),
                "application/octet-stream",
            ))
            .await;
        assert_eq!(
            created.status.as_u16(),
            case["metadata_status"],
            "{}",
            case["name"]
        );
        let blob = created.json();
        // Axum receives decoded body bytes; the front-server probe also sends real chunks.
        let mut request = Req::new(Method::PUT, blob["direct_upload"]["url"].as_str().unwrap())
            .body(case["body"].as_str().unwrap());
        for (name, value) in case["headers"].as_object().unwrap() {
            request = request.header(name, value.as_str().unwrap());
        }
        let reply = browser.send(request).await;
        assert_eq!(
            reply.status.as_u16(),
            case["put_status"],
            "{}: {}",
            case["name"],
            reply.text()
        );
        let path = a
            .booted
            .app
            .storage
            .service
            .path_for(blob["key"].as_str().unwrap());
        assert_eq!(path.exists(), case["file_exists"], "{}", case["name"]);
        if path.exists() {
            assert_eq!(
                std::fs::metadata(&path).unwrap().len(),
                case["stored_bytes"]
            );
            assert_eq!(
                campfire_storage::key::checksum_file(&path).unwrap(),
                case["stored_checksum"]
            );
        }
    }
}

#[tokio::test]
async fn direct_upload_destination_failures_return_500_without_staging_files() {
    let a = TestApp::boot().await.expect("default seed");
    let mut browser = a.sign_in(DAVID).await;
    for block_publication in [false, true] {
        let blob = browser
            .write(metadata(
                3,
                &campfire_storage::key::checksum(b"xxx"),
                "application/octet-stream",
            ))
            .await
            .json();
        let path = a
            .booted
            .app
            .storage
            .service
            .path_for(blob["key"].as_str().unwrap());
        let parent = path.parent().unwrap();
        if block_publication {
            std::fs::create_dir_all(&path).unwrap();
        } else {
            std::fs::create_dir_all(parent.parent().unwrap()).unwrap();
            std::fs::write(parent, b"blocked directory").unwrap();
        }
        let reply = browser
            .send(
                Req::new(Method::PUT, blob["direct_upload"]["url"].as_str().unwrap())
                    .header("content-type", "application/octet-stream")
                    .header("content-length", "3")
                    .body("xxx"),
            )
            .await;
        assert_eq!(
            reply.status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "{}",
            reply.text()
        );
        if block_publication {
            let entries: Vec<_> = std::fs::read_dir(parent)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            assert_eq!(entries, [path], "staged upload remains after failed rename");
        } else {
            assert!(!path.exists());
            assert_eq!(std::fs::read(parent).unwrap(), b"blocked directory");
        }
    }
}

#[tokio::test]
async fn direct_upload_checksum_mismatch_returns_422_and_preserves_a_complete_blob() {
    let a = TestApp::boot().await.expect("default seed");
    let mut browser = a.sign_in(DAVID).await;
    let blob = browser
        .write(metadata(
            3,
            &campfire_storage::key::checksum(b"xxx"),
            "application/octet-stream",
        ))
        .await
        .json();
    let request = |body: &str| {
        Req::new(Method::PUT, blob["direct_upload"]["url"].as_str().unwrap())
            .header("content-type", "application/octet-stream")
            .header("content-length", "3")
            .body(body)
    };
    assert_eq!(
        browser.send(request("xxx")).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        browser.send(request("yyy")).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let path = a
        .booted
        .app
        .storage
        .service
        .path_for(blob["key"].as_str().unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), b"xxx");
    assert_eq!(
        std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
        1
    );
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
