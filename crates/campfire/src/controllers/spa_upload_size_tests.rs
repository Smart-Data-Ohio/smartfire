use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use super::api_tests::{app, get, json_body};
use crate::controllers::presenters::test_support::{DAVID, KEVIN};

const MB: i64 = 1024 * 1024;

fn upload(size: i64) -> Value {
    json!({"filename": "large.bin", "byteSize": size, "checksum": "checksum", "contentType": "application/octet-stream"})
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
