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
    for (header, value) in expected["headers"].as_object().unwrap() {
        assert_eq!(reply.header(header), value.as_str(), "{label}: {header}");
    }
}
async fn missing(kind: &str, route: &str) {
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
