//! PR192 R5: Rails' handled missing-file redirects/proxies, without regeneration.
use super::agent_review_r3_tests::fresh_source;
use super::presenters::test_support::{Reply, Req, TestApp};
use base64::{Engine, engine::general_purpose::STANDARD};
use campfire_kit::Method;
use campfire_storage::{Blob, Variation};
use serde_json::Value;

async fn fixed_key(app: &TestApp, id: i64, key: &'static str) -> Blob {
    let blob = app
        .db()
        .read(move |conn| Ok(Blob::find(conn, id).unwrap().unwrap()))
        .await
        .unwrap();
    let storage = &app.booted.app.storage;
    let bytes = std::fs::read(storage.service.path_for(&blob.key)).unwrap();
    storage
        .service
        .upload(key, bytes.as_slice(), blob.checksum.as_deref())
        .unwrap();
    storage.service.delete(&blob.key).unwrap();
    let time = std::time::UNIX_EPOCH
        + std::time::Duration::from_secs(app.booted.app.clock.now().as_second() as u64);
    std::fs::File::open(storage.service.path_for(key))
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(time))
        .unwrap();
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE active_storage_blobs SET key=? WHERE id=?",
                rusqlite::params![key, id],
            )?;
            Ok(Blob::find(tx.conn(), id).unwrap().unwrap())
        })
        .await
        .unwrap()
}
fn request(path: &str) -> Req {
    Req::new(Method::GET, path)
        .header("accept", "*/*")
        .header("host", "campfire.test")
}
fn compare(reply: &Reply, expected: &Value, label: &str) {
    compare_headers(reply, expected, label);
    assert_eq!(
        reply.status.as_u16(),
        expected["status"].as_u64().unwrap() as u16,
        "{label}: {}",
        reply.text()
    );
    assert_eq!(
        reply.body.as_slice(),
        STANDARD
            .decode(expected["body_base64"].as_str().unwrap())
            .unwrap(),
        "{label}: exact response bytes"
    );
}
fn compare_headers(reply: &Reply, expected: &Value, label: &str) {
    let actual = reply
        .headers
        .keys()
        .map(|name| {
            let values = reply
                .headers
                .get_all(name)
                .iter()
                .map(|value| value.to_str().unwrap().to_owned())
                .collect::<Vec<_>>();
            (name.as_str().to_owned(), serde_json::json!(values))
        })
        .collect::<serde_json::Map<_, _>>();
    compare_full_proxy_headers(
        &Value::Object(actual),
        &expected["headers"],
        label.contains("proxy"),
    );
}

pub(super) fn compare_full_proxy_headers(actual: &Value, expected: &Value, streamed: bool) {
    let mut actual = actual.as_object().expect("all response headers").clone();
    let mut expected = expected
        .as_object()
        .expect("all Rails response headers")
        .clone();
    for headers in [&actual, &expected] {
        for (name, values) in headers {
            assert_eq!(
                values.as_array().expect("header values").len(),
                1,
                "duplicate {name}"
            );
            assert!(values[0].is_string(), "{name}: header value");
            let value = values[0].as_str().unwrap();
            assert!(!value.contains(['\r', '\n']), "{name}: newline in header");
            match name.as_str() {
                "date" => {
                    httpdate::parse_http_date(value).expect("valid HTTP Date");
                }
                "x-request-id" => {
                    let id = uuid::Uuid::parse_str(value).expect("UUID request ID");
                    assert_eq!(id.get_version_num(), 4);
                    assert_eq!(id.to_string(), value);
                }
                "x-runtime" => {
                    let (seconds, fraction) = value.split_once('.').expect("runtime decimal");
                    assert!(!seconds.is_empty() && seconds.bytes().all(|c| c.is_ascii_digit()));
                    assert_eq!(fraction.len(), 6);
                    assert!(fraction.bytes().all(|c| c.is_ascii_digit()));
                }
                _ => {}
            }
        }
    }
    for name in ["date", "x-request-id", "x-runtime"] {
        actual.remove(name);
        expected.remove(name);
    }
    if streamed {
        for (name, value) in [
            ("permissions-policy", crate::security::PERMISSIONS_POLICY),
            ("referrer-policy", "strict-origin-when-cross-origin"),
            ("x-content-type-options", "nosniff"),
            ("x-frame-options", "SAMEORIGIN"),
            ("x-permitted-cross-domain-policies", "none"),
            ("x-xss-protection", "0"),
        ] {
            assert!(expected.get(name).is_none(), "Rails streamed {name}");
            assert_eq!(
                actual.remove(name),
                Some(serde_json::json!([value])),
                "approved addition {name}"
            );
        }
    }
    assert_eq!(actual, expected, "every non-approved response header");
}

#[test]
fn ws11_proxy_header_oracle_rejects_unapproved_changes() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_blob_proxy_headers.json"
    ))
    .unwrap();
    for case in vector["cases"].as_array().unwrap() {
        let expected = &case["headers"];
        let mut actual = expected.clone();
        for (name, value) in crate::security::default_headers() {
            actual[name.as_str()] = serde_json::json!([value.to_str().unwrap()]);
        }
        compare_full_proxy_headers(&actual, expected, true);
        for (name, value) in [
            ("content-transfer-encoding", serde_json::json!(["unexpected"])),
            ("x-unexpected", serde_json::json!(["new"])),
            ("content-type", serde_json::json!(["wrong"])),
            ("x-request-id", serde_json::json!(["same", "same"])),
            ("x-frame-options", serde_json::json!(["DENY"])),
        ] {
            let mut changed = actual.clone();
            changed[name] = value;
            assert!(
                std::panic::catch_unwind(|| compare_full_proxy_headers(&changed, expected, true))
                    .is_err(),
                "must reject {name}"
            );
        }
        let mut changed = actual.clone();
        changed
            .as_object_mut()
            .unwrap()
            .remove("content-security-policy");
        // A missing CSP must be rejected whenever that response renders it.
        if expected.get("content-security-policy").is_some() {
            assert!(
                std::panic::catch_unwind(|| compare_full_proxy_headers(&changed, expected, true))
                    .is_err()
            );
        }
    }
}
async fn missing(kind: &str, route: &str) {
    crate::security::with_proxy_fixture_nonce(missing_with_nonce(kind, route)).await
}
async fn missing_with_nonce(kind: &str, route: &str) {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_review192r5_representations.json"
    ))
    .unwrap();
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == kind)
        .unwrap();
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    let source = fresh_source(&app, if kind == "jpeg_variant" { 1 } else { 9 }).await;
    assert_eq!(source, case["source_id"].as_i64().unwrap());
    let blob = fixed_key(&app, source, "ws11apir5source").await;
    let storage = &app.booted.app.storage;
    let changes = if kind == "jpeg_variant" {
        storage
            .variation_for(&blob, &Variation::resize_to_limit(1200, 800, None))
            .unwrap()
    } else {
        Variation::format_only("webp")
    };
    // Decode the signed HTTP key: Ruby string formats use their distinct digest.
    let variation = Variation::decode(
        &*storage.verifier,
        &changes.key(&*storage.verifier),
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
    let image = fixed_key(&app, image.id, "ws11apir5variant").await;
    let preview = app
        .db()
        .read(move |conn| {
            Ok(Blob::attached(conn, "ActiveStorage::Blob", source, "preview_image").unwrap())
        })
        .await
        .unwrap();
    let preview = if let Some(preview) = preview {
        Some(fixed_key(&app, preview.id, "ws11apir5preview").await)
    } else {
        None
    };
    assert_eq!(image.id, case["variant_id"].as_i64().unwrap());
    let redirect = campfire_storage::paths::representation_redirect_path(
        &*storage.verifier,
        &blob,
        &variation,
    );
    let proxy =
        campfire_storage::paths::representation_proxy_path(&*storage.verifier, &blob, &variation);
    assert_eq!(redirect, case["redirect"]["path"].as_str().unwrap());
    assert_eq!(proxy, case["proxy"]["path"].as_str().unwrap());
    let positive_redirect = app.anonymous().send(request(&redirect)).await;
    assert_eq!(positive_redirect.status, 302);
    let disk_path = positive_redirect
        .header("location")
        .unwrap()
        .strip_prefix("http://campfire.test")
        .unwrap()
        .to_string();
    let positive_disk = app.anonymous().send(request(&disk_path)).await;
    let positive_proxy = app.anonymous().send(request(&proxy)).await;
    assert_eq!(positive_disk.status, 200);
    assert_eq!(positive_proxy.status, 200);
    let missing = if kind == "video_preview" {
        preview.as_ref().unwrap()
    } else {
        &image
    };
    storage.service.delete(&missing.key).unwrap();
    app.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let rows = super::agent_review_tests::snapshot(&app).await;
    let files = super::agent_review_tests::stored_files(&app);
    let reply = app
        .anonymous()
        .send(request(if route == "redirect" {
            &redirect
        } else {
            &proxy
        }))
        .await;
    // Assert the regression before comparing the positive-control media bytes.
    compare_headers(&reply, &case[route], &format!("{kind} {route}"));
    compare_headers(&positive_proxy, &case["baseline_proxy"], "positive proxy");
    compare(&reply, &case[route], &format!("{kind} {route}"));
    let redirected = app.anonymous().send(request(&redirect)).await;
    compare(&redirected, &case["repeat_redirect"], "repeat redirect");
    let followed = app.anonymous().send(request(&disk_path)).await;
    compare(&followed, &case["disk"], "disk followup");
    compare(
        &positive_redirect,
        &case["baseline_redirect"],
        "positive redirect",
    );
    compare(&positive_disk, &case["baseline_disk"], "positive disk");
    compare(&positive_proxy, &case["baseline_proxy"], "positive proxy");
    assert_eq!(super::agent_review_tests::snapshot(&app).await, rows);
    assert_eq!(super::agent_review_tests::stored_files(&app), files);
    assert!(!storage.service.exist(&missing.key));
    println!(
        "PR192_R5_REPRESENTATION {kind} {route} status={} disk_status={} rows/files/jobs=unchanged regenerated=false",
        reply.status.as_u16(),
        followed.status.as_u16()
    );
}
macro_rules! cases {($($name:ident=>$kind:literal,$route:literal);* $(;)?)=>{$(
    #[tokio::test] async fn $name(){missing($kind,$route).await;}
)*};}
cases! {
    pr192_r5_video_missing_preview_redirect=>"video_preview","redirect";
    pr192_r5_video_missing_preview_proxy=>"video_preview","proxy";
    pr192_r5_video_missing_variant_redirect=>"video_variant","redirect";
    pr192_r5_video_missing_variant_proxy=>"video_variant","proxy";
    pr192_r5_jpeg_missing_variant_redirect=>"jpeg_variant","redirect";
    pr192_r5_jpeg_missing_variant_proxy=>"jpeg_variant","proxy";
}

#[tokio::test]
async fn ws11_next2_blob_proxy_every_header_and_body() {
    crate::security::with_proxy_fixture_nonce(async {
        let vector: Value = serde_json::from_str(include_str!(
            "../../../../vectors/agent_blob_proxy_headers.json"
        ))
        .unwrap();
        let app = TestApp::boot_frozen()
            .await
            .expect("default seed required")
            .without_job_runner()
            .await;
        let storage = app.booted.app.storage.clone();
        let id = app
            .db()
            .write(move |tx| {
                let staged = storage
                    .stage_bytes(
                        b"proxy header fixture\n",
                        campfire_storage::Filename::new("fixture.txt"),
                        Some("text/plain"),
                    )
                    .unwrap();
                let blob = staged.insert(tx.conn(), tx.now().jiff()).unwrap();
                crate::active_storage::keep_after_commit(tx, staged);
                Ok(blob.id)
            })
            .await
            .unwrap();
        assert_eq!(id, vector["blob_id"].as_i64().unwrap());
        let blob = fixed_key(&app, id, "ws11api-next2-blob").await;
        let path = campfire_storage::paths::blob_proxy_path(
            &*app.booted.app.storage.verifier,
            &blob,
            None,
        );
        assert_eq!(path, vector["path"].as_str().unwrap());
        app.db()
            .write(|tx| {
                tx.conn().execute("DELETE FROM background_jobs", [])?;
                Ok(())
            })
            .await
            .unwrap();
        let rows = super::agent_review_tests::snapshot(&app).await;
        for case in vector["cases"].as_array().unwrap() {
            if case["name"] == "missing" {
                app.booted.app.storage.service.delete(&blob.key).unwrap();
            }
            let mut req = request(&path);
            if let Some(range) = case["range"].as_str() {
                req = req.header("range", range);
            }
            let reply = app.anonymous().send(req).await;
            // The proxy controller selects Live::Response for ranges too.
            compare(&reply, case, "blob proxy");
            assert_eq!(super::agent_review_tests::snapshot(&app).await, rows);
        }
        assert!(!app.booted.app.storage.service.exist(&blob.key));
        println!("WS11 blob proxy: 4 responses; all headers/body bytes; rows/jobs unchanged");
    })
    .await;
}
