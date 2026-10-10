use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use super::api_tests::{app, get, json_body};
use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, KEVIN, Req};

const MB: i64 = 1024 * 1024;

#[tokio::test]
async fn upload_size_put_without_content_length_stops_before_spooling_101_mb() {
    use axum::body::Body;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tower::ServiceExt;

    let a = app(true).await.expect("restored default seed");
    let mut david = a.sign_in(DAVID).await;
    let source = tempfile::NamedTempFile::new().unwrap();
    source.as_file().set_len(101 * MB as u64).unwrap();
    let checksum = campfire_storage::key::checksum_file(source.path()).unwrap();
    let mut metadata = upload(0);
    metadata["checksum"] = json!(checksum);
    let created = david
        .write(json_body(Method::POST, "/api/v1/uploads", &metadata))
        .await;
    assert_eq!(created.status, StatusCode::CREATED);
    let signed: campfire_api_types::DirectUpload = serde_json::from_slice(&created.body).unwrap();
    let read = Arc::new(AtomicUsize::new(0));
    let polled = read.clone();
    let chunks = futures_util::stream::iter((0..1616).map(move |_| {
        polled.fetch_add(1, Ordering::SeqCst);
        Ok::<_, std::io::Error>(bytes::Bytes::from_static(&[0; 64 * 1024]))
    }));
    let request = axum::http::Request::builder()
        .method(Method::PUT)
        .uri(&signed.upload_url)
        .version(axum::http::Version::HTTP_2)
        .header("host", "campfire.test")
        .header("cookie", david.cookie_header())
        .header("content-type", "application/octet-stream")
        .body(Body::from_stream(chunks))
        .unwrap();
    let reply = a.booted.router.clone().oneshot(request).await.unwrap();
    assert_eq!(reply.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        read.load(Ordering::SeqCst),
        1,
        "read past the declared size + 1"
    );
    let key: String = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT key FROM active_storage_blobs ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    assert!(!a.booted.app.storage.service.exist(&key));
}

#[tokio::test]
async fn upload_size_put_requires_actual_bytes_to_equal_the_declaration() {
    let a = app(true).await.expect("restored default seed");
    let mut david = a.sign_in(DAVID).await;
    for (declared, length, expected) in [
        (3, Some("3"), StatusCode::PAYLOAD_TOO_LARGE),
        (5, Some("5"), StatusCode::UNPROCESSABLE_ENTITY),
        (4, None, StatusCode::NO_CONTENT),
    ] {
        let mut metadata = upload(declared);
        metadata["checksum"] = json!(campfire_storage::key::checksum(b"xxxx"));
        let created = david
            .write(json_body(Method::POST, "/api/v1/uploads", &metadata))
            .await;
        let signed: campfire_api_types::DirectUpload =
            serde_json::from_slice(&created.body).unwrap();
        let mut put = Req::new(Method::PUT, &signed.upload_url)
            .header("content-type", "application/octet-stream")
            .body("xxxx");
        if let Some(length) = length {
            put = put.header("content-length", length);
        }
        assert_eq!(
            david.send(put).await.status,
            expected,
            "declared {declared}, header {length:?}"
        );
    }
}

#[tokio::test]
async fn upload_size_put_stores_all_received_form_bytes() {
    let a = app(true).await.expect("restored default seed");
    let mut david = a.sign_in(DAVID).await;
    let mut metadata = upload(3);
    metadata["checksum"] = json!(campfire_storage::key::checksum(b"a=1"));
    metadata["contentType"] = json!("application/x-www-form-urlencoded");
    let created = david
        .write(json_body(Method::POST, "/api/v1/uploads", &metadata))
        .await;
    let signed: campfire_api_types::DirectUpload = serde_json::from_slice(&created.body).unwrap();
    let reply = david
        .send(
            Req::new(Method::PUT, &signed.upload_url)
                .header("content-type", "application/x-www-form-urlencoded")
                .header("content-length", "3")
                .body("a=1"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let key: String = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT key FROM active_storage_blobs ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(a.booted.app.storage.service.path_for(&key)).unwrap(),
        b"a=1"
    );
}

#[tokio::test]
async fn upload_size_http2_rejects_a_101_mb_put_without_content_length() {
    use axum::body::Body;
    let a = app(true).await.expect("restored default seed");
    let mut david = a.sign_in(DAVID).await;
    let source = tempfile::NamedTempFile::new().unwrap();
    source.as_file().set_len(101 * MB as u64).unwrap();
    let mut metadata = upload(0);
    metadata["checksum"] = json!(campfire_storage::key::checksum_file(source.path()).unwrap());
    let created = david
        .write(json_body(Method::POST, "/api/v1/uploads", &metadata))
        .await;
    let signed: campfire_api_types::DirectUpload = serde_json::from_slice(&created.body).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let router = a.booted.router.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
    let (mut sender, connection) = hyper::client::conn::http2::handshake(
        hyper_util::rt::TokioExecutor::new(),
        hyper_util::rt::TokioIo::new(socket),
    )
    .await
    .unwrap();
    let connection = tokio::spawn(connection);
    let chunks = futures_util::stream::iter(
        (0..1616).map(|_| Ok::<_, std::io::Error>(bytes::Bytes::from_static(&[0; 64 * 1024]))),
    );
    let request = axum::http::Request::builder()
        .method(Method::PUT)
        .uri(format!("http://{addr}{}", signed.upload_url))
        .header("cookie", david.cookie_header())
        .header("content-type", "application/octet-stream")
        .body(Body::from_stream(chunks))
        .unwrap();
    assert!(!request.headers().contains_key("content-length"));
    let reply = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        sender.send_request(request),
    )
    .await
    .unwrap()
    .unwrap();
    let status = reply.status();
    drop(reply);
    drop(sender);
    connection.abort();
    let _ = stop.send(());
    server.await.unwrap();
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn upload_size_multipart_enforces_icon_and_branding_limits_before_staging() {
    use axum::body::Body;
    use futures_util::StreamExt;
    use tower::ServiceExt;
    let a = app(true).await.expect("restored default seed");
    let mut david = a.sign_in(DAVID).await;
    let csrf = david.authenticity_token().await;
    for (field, limit) in [
        ("workspace_icon[image]", 256 * 1024),
        ("account[logo]", 10 * MB),
        ("account[banner]", 10 * MB),
    ] {
        let prefix = bytes::Bytes::from(format!(
            "--B\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"large.png\"\r\nContent-Type: image/png\r\n\r\n"
        ));
        let chunks = std::iter::once(prefix)
            .chain((0..(limit / 1024 + 1)).map(|_| bytes::Bytes::from_static(&[0; 1024])));
        let stream = futures_util::stream::iter(chunks.map(Ok::<_, std::io::Error>))
            .chain(futures_util::stream::pending());
        let request = axum::http::Request::builder()
            .method(Method::POST)
            .uri("/account")
            .header("host", "campfire.test")
            .header("cookie", david.cookie_header())
            .header("x-csrf-token", &csrf)
            .header("content-type", "multipart/form-data; boundary=B")
            .body(Body::from_stream(stream))
            .unwrap();
        let reply = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            a.booted.router.clone().oneshot(request),
        )
        .await;
        assert!(
            reply.is_ok(),
            "{field}: waited for the oversized body to finish"
        );
        assert_eq!(
            reply.unwrap().unwrap().status(),
            StatusCode::PAYLOAD_TOO_LARGE,
            "{field}"
        );
    }
}

#[tokio::test]
async fn upload_size_shared_staging_rejects_a_file_length_mismatch() {
    use crate::controllers::presenters::attachments::{Assignment, Upload};
    let a = app(true).await.expect("restored default seed");
    let source = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(source.path(), b"four").unwrap();
    let file = campfire_kit::UploadedFile::new(
        "file.txt".into(),
        Some("text/plain".into()),
        String::new(),
        3,
        source.into_temp_path(),
    );
    let upload =
        Upload::from_param(Some(&campfire_kit::Param::File(std::sync::Arc::new(file)))).unwrap();
    assert!(matches!(
        Assignment::Create(upload).stage(&a.booted.app).await,
        Err(campfire_kit::Error::Status(
            StatusCode::UNPROCESSABLE_ENTITY
        ))
    ));
}

#[tokio::test]
async fn upload_size_multipart_stops_reading_each_attachment_at_the_workspace_limit() {
    use axum::body::Body;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tower::ServiceExt;

    let a = app(true).await.expect("restored default seed");
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE accounts SET settings = ?",
                [r#"{"upload_limit_bytes":1024}"#],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let csrf = david.authenticity_token().await;
    for field in [
        "message[attachment]",
        "attachment",
        "channel_thread[message][attachment]",
    ] {
        let read = Arc::new(AtomicUsize::new(0));
        let polled = read.clone();
        let prefix = bytes::Bytes::from(format!(
            "--B\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"large.bin\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        ));
        let chunks = std::iter::once(prefix)
            .chain((0..100).map(|_| bytes::Bytes::from_static(&[0; 1024])))
            .chain(std::iter::once(bytes::Bytes::from_static(b"\r\n--B--\r\n")));
        let stream = futures_util::stream::iter(chunks.map(move |chunk| {
            polled.fetch_add(1, Ordering::SeqCst);
            Ok::<_, std::io::Error>(chunk)
        }));
        let request = axum::http::Request::builder()
            .method(Method::POST)
            .uri(format!("/rooms/{ALL_TALK}/messages"))
            .header("host", "campfire.test")
            .header("cookie", david.cookie_header())
            .header("x-csrf-token", &csrf)
            .header("content-type", "multipart/form-data; boundary=B")
            .body(Body::from_stream(stream))
            .unwrap();
        let reply = a.booted.router.clone().oneshot(request).await.unwrap();
        assert_eq!(reply.status(), StatusCode::PAYLOAD_TOO_LARGE, "{field}");
        assert!(
            read.load(Ordering::SeqCst) <= 4,
            "{field}: drained the oversized body"
        );
    }
}

#[tokio::test]
async fn upload_size_missing_existing_files_defer_processing_but_still_enforce_the_limit() {
    use crate::controllers::presenters::attachments::Assignment;

    let a = app(true).await.expect("restored default seed");
    let blob = a
        .db()
        .read(|conn| Ok(campfire_storage::Blob::find(conn, 1).unwrap().unwrap()))
        .await
        .unwrap();
    assert!(blob.is_identified());
    assert!(blob.byte_size > 0);
    a.booted.app.storage.service.delete(&blob.key).unwrap();
    let signed =
        campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
    for assignment in [Assignment::Signed(signed), Assignment::Existing(blob.clone())] {
        let staged = assignment
            .clone()
            .stage_with_limit(&a.booted.app, blob.byte_size as u64)
            .await
            .expect("an identified blob's missing file is handled by attachment processing");
        assert!(matches!(staged, Assignment::Existing(existing) if existing.id == blob.id));
        assert!(matches!(
            assignment
                .stage_with_limit(&a.booted.app, blob.byte_size as u64 - 1)
                .await,
            Err(campfire_kit::Error::Status(StatusCode::PAYLOAD_TOO_LARGE))
        ));
    }
}

#[tokio::test]
async fn upload_size_existing_file_lengths_are_checked_before_assignment() {
    use crate::controllers::presenters::attachments::Assignment;

    let a = app(true).await.expect("restored default seed");
    let staged = a
        .booted
        .app
        .storage
        .stage_bytes(b"a file", campfire_storage::Filename::new("file.txt"), Some("text/plain"))
        .unwrap();
    let blob = a
        .db()
        .write(move |tx| crate::controllers::messages::save_staged(tx, staged))
        .await
        .unwrap();
    let signed =
        campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
    for (bytes, expected) in [
        (b"short".as_slice(), StatusCode::UNPROCESSABLE_ENTITY),
        (b"too long".as_slice(), StatusCode::PAYLOAD_TOO_LARGE),
    ] {
        std::fs::write(a.booted.app.storage.service.path_for(&blob.key), bytes).unwrap();
        for assignment in [Assignment::Signed(signed.clone()), Assignment::Existing(blob.clone())] {
            let result = assignment.stage_with_limit(&a.booted.app, blob.byte_size as u64).await;
            assert!(matches!(result, Err(campfire_kit::Error::Status(status)) if status == expected));
        }
    }
}

#[tokio::test]
async fn upload_size_existing_blobs_are_limited_for_messages_but_not_avatars() {
    let a = app(true).await.expect("restored default seed");
    let mut david = a.sign_in(DAVID).await;
    let staged = a
        .booted
        .app
        .storage
        .stage_bytes(
            b"a file",
            campfire_storage::Filename::new("file.txt"),
            Some("text/plain"),
        )
        .unwrap();
    let blob = a
        .db()
        .write(move |tx| crate::controllers::messages::save_staged(tx, staged))
        .await
        .unwrap();
    let signed_id =
        campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE accounts SET settings = ?",
                [r#"{"upload_limit_bytes":4}"#],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let rejected = david.write(json_body(Method::POST, &format!("/api/v1/rooms/{ALL_TALK}/messages"), &json!({"clientMessageId":"too-large", "markdownSource":"", "attachmentSignedId":signed_id}))).await;
    assert_eq!(
        rejected.status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "{}",
        rejected.text()
    );
    let avatar = david
        .write(json_body(
            Method::PUT,
            "/api/v1/settings/avatar",
            &json!({"signedId":signed_id}),
        ))
        .await;
    assert_eq!(avatar.status, StatusCode::OK, "{}", avatar.text());
}

fn upload(size: i64) -> Value {
    json!({"filename": "large.bin", "byteSize": size, "checksum": "checksum", "contentType": "application/octet-stream"})
}

#[tokio::test]
async fn upload_size_method_overridden_disk_put_is_bounded_before_form_parsing() {
    use axum::body::Body;
    use futures_util::StreamExt;
    use tower::ServiceExt;
    let a = app(true).await.expect("restored default seed");
    let mut david = a.sign_in(DAVID).await;
    let created = david
        .write(json_body(Method::POST, "/api/v1/uploads", &upload(0)))
        .await;
    let signed: campfire_api_types::DirectUpload = serde_json::from_slice(&created.body).unwrap();
    let chunks = [bytes::Bytes::from_static(b"--B\r\nContent-Disposition: form-data; name=\"_method\"\r\n\r\nPUT\r\n--B\r\nContent-Disposition: form-data; name=\"attachment\"; filename=\"large.bin\"\r\n\r\n"), bytes::Bytes::from_static(&[0; 1024])];
    let stream = futures_util::stream::iter(chunks.into_iter().map(Ok::<_, std::io::Error>))
        .chain(futures_util::stream::pending());
    let request = axum::http::Request::builder()
        .method(Method::POST)
        .uri(&signed.upload_url)
        .header("host", "campfire.test")
        .header("cookie", david.cookie_header())
        .header("content-type", "multipart/form-data; boundary=B")
        .body(Body::from_stream(stream))
        .unwrap();
    let reply = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        a.booted.router.clone().oneshot(request),
    )
    .await;
    assert!(
        reply.is_ok(),
        "method override waited for the oversized body to finish"
    );
    assert_eq!(
        reply.unwrap().unwrap().status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[tokio::test]
async fn upload_size_defaults_to_100_mb_and_rejects_before_allocating_a_blob() {
    let a = app(true).await.expect("restored default seed");
    let mut david = a.sign_in(DAVID).await;
    let boot = david.send(get("/api/v1/boot")).await.json();
    assert_eq!(boot["account"]["uploadLimitBytes"], 100 * MB);
    let before: i64 = a
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT COUNT(*) FROM active_storage_blobs", [], |row| {
                    row.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    let rejected = david
        .write(json_body(
            Method::POST,
            "/api/v1/uploads",
            &upload(100 * MB + 1),
        ))
        .await;
    assert_eq!(
        rejected.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        rejected.text()
    );
    assert!(
        rejected.json()["error"]["fields"]["byteSize"][0]
            .as_str()
            .unwrap()
            .contains("100 MB")
    );
    let after: i64 = a
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT COUNT(*) FROM active_storage_blobs", [], |row| {
                    row.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        david
            .write(json_body(
                Method::POST,
                "/api/v1/uploads",
                &upload(100 * MB)
            ))
            .await
            .status,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn upload_size_is_admin_configurable_and_preserves_other_settings() {
    let a = app(true).await.expect("restored default seed");
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE accounts SET settings = ?",
                [r#"{"restrict_room_creation_to_administrators":true,"future_setting":"keep"}"#],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let change = json!({"uploadLimitBytes": 125 * MB});
    assert_eq!(
        kevin
            .write(json_body(Method::PATCH, "/api/v1/admin/workspace", &change))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let saved = david
        .write(json_body(Method::PATCH, "/api/v1/admin/workspace", &change))
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.text());
    assert_eq!(saved.json()["uploadLimitBytes"], 125 * MB);
    let settings: Value = a
        .db()
        .read(|conn| {
            let account = campfire_db::Account::first(conn)?.unwrap();
            Ok(serde_json::from_str(account.settings_json.as_deref().unwrap()).unwrap())
        })
        .await
        .unwrap();
    assert_eq!(
        settings,
        json!({"restrict_room_creation_to_administrators":true,"future_setting":"keep","upload_limit_bytes":125 * MB})
    );
    for invalid in [0, -1, 9_007_199_254_740_992i64] {
        let rejected = david
            .write(json_body(
                Method::PATCH,
                "/api/v1/admin/workspace",
                &json!({"uploadLimitBytes": invalid}),
            ))
            .await;
        assert_eq!(
            rejected.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            rejected.text()
        );
    }
    assert_eq!(
        david.send(get("/api/v1/boot")).await.json()["account"]["uploadLimitBytes"],
        125 * MB
    );
    assert_eq!(
        david
            .write(json_body(
                Method::POST,
                "/api/v1/uploads",
                &upload(125 * MB)
            ))
            .await
            .status,
        StatusCode::CREATED
    );
    let rejected = david
        .write(json_body(
            Method::POST,
            "/api/v1/uploads",
            &upload(125 * MB + 1),
        ))
        .await;
    assert_eq!(rejected.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(rejected.text().contains("125 MB"));
    let legacy = json!({"blob":{"filename":"large.bin","byte_size":125 * MB + 1,"checksum":"checksum","content_type":"application/octet-stream"}});
    let rejected = david
        .write(json_body(
            Method::POST,
            "/rails/active_storage/direct_uploads",
            &legacy,
        ))
        .await;
    assert_eq!(rejected.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(rejected.text().contains("125 MB"));
}

#[tokio::test]
async fn upload_size_100_mb_streams_through_the_real_front_server_to_disk() {
    use std::io::Read;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let a = app(true).await.expect("restored default seed");
    let mut david = a.sign_in(DAVID).await;
    let mut metadata = upload(100 * MB);
    // MD5 of 100 MiB of zero bytes, independently computed.
    metadata["checksum"] = json!("LygrhOfmCNWFJEntlAv8UQ==");
    let created = david
        .write(json_body(Method::POST, "/api/v1/uploads", &metadata))
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let signed: campfire_api_types::DirectUpload = serde_json::from_slice(&created.body).unwrap();

    let http = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = http.local_addr().unwrap();
    let target_port = target.local_addr().unwrap().port();
    let config = campfire_kit::front::FrontConfig::from_lookup(|name| match name {
        "HTTP_PORT" => Some(addr.port().to_string()),
        "TARGET_PORT" => Some(target_port.to_string()),
        "HTTP_READ_TIMEOUT" | "HTTP_WRITE_TIMEOUT" => Some("300".into()),
        "LOG_REQUESTS" => Some("false".into()),
        _ => None,
    });
    assert_eq!(config.max_request_body, 0);
    let (ready, started) = tokio::sync::oneshot::channel();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let router = a.booted.router.clone();
    let server = tokio::spawn(campfire_kit::front::serve_with_bound_listeners(
        config,
        router,
        None,
        (http, target),
        move || {
            let _ = ready.send(());
        },
        async move {
            let _ = stopped.await;
        },
    ));
    started.await.unwrap();
    let mut socket = tokio::net::TcpStream::connect(addr).await.unwrap();
    let headers = format!(
        "PUT {} HTTP/1.1\r\nHost: campfire.test\r\nCookie: {}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        signed.upload_url,
        david.cookie_header(),
        100 * MB
    );
    socket.write_all(headers.as_bytes()).await.unwrap();
    let chunk = [0u8; 64 * 1024];
    for _ in 0..1600 {
        socket.write_all(&chunk).await.unwrap();
    }
    let mut response = Vec::new();
    socket.read_to_end(&mut response).await.unwrap();
    assert!(
        response.starts_with(b"HTTP/1.1 204"),
        "{}",
        String::from_utf8_lossy(&response)
    );
    let _ = stop.send(());
    server.await.unwrap().unwrap();

    let key: String = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT key FROM active_storage_blobs ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    let path = a.booted.app.storage.service.path_for(&key);
    let mut file = std::fs::File::open(path).unwrap();
    assert_eq!(file.metadata().unwrap().len(), 104_857_600);
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0;
    loop {
        let read = file.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        assert!(buffer[..read].iter().all(|byte| *byte == 0));
        total += read;
    }
    assert_eq!(total, 104_857_600);
}
