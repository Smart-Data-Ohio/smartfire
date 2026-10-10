//! Custom emoji originals, stills and workspace capacity through the admin API.

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use super::admin_tests::{app, error, get, parse, write};
use crate::controllers::presenters::test_support::{DAVID, TestApp};

const GIF: &[u8] = include_bytes!("../../../../fixtures/files/workspace_icons/animated.gif");
const WEBP: &[u8] = include_bytes!("../../../../fixtures/files/workspace_icons/animated.webp");
const PNG: &[u8] = include_bytes!("../../../../fixtures/files/workspace_icons/square_64.png");

fn repeated_gif(frames: usize) -> Vec<u8> {
    let starts: Vec<_> = GIF
        .windows(3)
        .enumerate()
        .filter_map(|(i, bytes)| (bytes == b"!\xf9\x04").then_some(i))
        .collect();
    let mut bytes = GIF[..starts[0]].to_vec();
    for _ in 0..frames {
        bytes.extend_from_slice(&GIF[starts[0]..starts[1]]);
    }
    bytes.push(b';');
    bytes
}

#[tokio::test]
async fn animated_emoji_bounds_dimensions_and_frame_count() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let mut wide = GIF.to_vec();
    wide[6..8].copy_from_slice(&513u16.to_le_bytes());
    let frames = repeated_gif(101);
    for (bytes, message) in [
        (wide.as_slice(), "must be at most 512 × 512 pixels"),
        (frames.as_slice(), "must contain at most 100 frames"),
    ] {
        let signed = upload(&a, bytes, "bounded.gif", "image/gif").await;
        let reply = write(
            &mut b,
            Method::POST,
            "/api/v1/admin/icons",
            json!({"name": "bounded", "title": "Bounded", "signedId": signed}),
        )
        .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert_eq!(error(&reply)["fields"]["image"], json!([message]));
    }
}

async fn upload(a: &TestApp, bytes: &[u8], filename: &str, content_type: &str) -> String {
    let staged = a
        .booted
        .app
        .storage
        .stage_bytes(
            bytes,
            campfire_storage::Filename::new(filename),
            Some(content_type),
        )
        .unwrap();
    let blob = a
        .db()
        .write(move |tx| crate::controllers::messages::save_staged(tx, staged))
        .await
        .unwrap();
    campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None)
}

#[tokio::test]
async fn static_emoji_accepts_a_1024_pixel_png_under_the_byte_limit() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let bytes = super::workspace_branding_tests::png(1024, 1024, false);
    assert!(bytes.len() < 256 * 1024);
    let signed = upload(&a, &bytes, "large_static.png", "image/png").await;
    let reply = write(
        &mut b,
        Method::POST,
        "/api/v1/admin/icons",
        json!({"name": "large_static", "title": "Large static", "signedId": signed}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let list: Value = parse(&reply);
    assert_eq!(list["animatedUsage"], 0);
    let icon = &list["icons"][0];
    assert_eq!(icon["name"], "large_static");
    assert_eq!(icon["animated"], false);
    assert_eq!(icon["stillUrl"], icon["imageUrl"]);
    let original = b.get(icon["imageUrl"].as_str().unwrap()).await;
    assert_eq!(original.status, StatusCode::OK);
    assert_eq!(original.content_type(), Some("image/png"));
    assert_eq!(original.body, bytes);
}

#[tokio::test]
async fn animated_emoji_accepts_gif_and_webp_and_serves_original_and_first_frame() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    for (name, bytes, content_type) in [
        ("dance_gif", GIF, "image/gif"),
        ("dance_webp", WEBP, "image/webp"),
    ] {
        // Neither the supplied type nor the filename identifies the format.
        let signed = upload(&a, bytes, "disguised.txt", "text/plain").await;
        let reply = write(
            &mut b,
            Method::POST,
            "/api/v1/admin/icons",
            json!({"name": name, "title": "Dance", "signedId": signed}),
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let list: Value = parse(&reply);
        let icon = list["icons"]
            .as_array()
            .unwrap()
            .iter()
            .find(|icon| icon["name"] == name)
            .unwrap();
        assert_eq!(icon["animated"], true);
        let original = b.get(icon["imageUrl"].as_str().unwrap()).await;
        assert_eq!(original.status, StatusCode::OK);
        assert_eq!(original.content_type(), Some(content_type));
        assert_eq!(original.body, bytes);
        let still_url = icon["stillUrl"].as_str().unwrap();
        let still = b.get(still_url).await;
        assert_eq!(still.status, StatusCode::OK);
        assert_eq!(still.content_type(), Some("image/png"));
        assert_eq!(&still.body[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(
            u32::from_be_bytes(still.body[16..20].try_into().unwrap()),
            64
        );
        assert_eq!(
            u32::from_be_bytes(still.body[20..24].try_into().unwrap()),
            64
        );
        let still_file = tempfile::Builder::new().suffix(".png").tempfile().unwrap();
        std::fs::write(still_file.path(), &still.body).unwrap();
        assert!(
            campfire_storage::vips::Image::open_sequential(still_file.path())
                .unwrap()
                .average()
                .unwrap()
                < 1.0,
            "the first frame is black; the second is white"
        );
        assert_ne!(original.header("etag"), still.header("etag"));
        let not_modified = b
            .send(get(still_url).header("if-none-match", still.header("etag").unwrap()))
            .await;
        assert_eq!(not_modified.status, StatusCode::NOT_MODIFIED);
        let catalog: Value = parse(&b.send(get("/api/v1/icons")).await);
        let entry = catalog["icons"]
            .as_array()
            .unwrap()
            .iter()
            .find(|icon| icon["name"] == name)
            .unwrap();
        assert_eq!(entry["animated"], true);
        assert_eq!(entry["stillUrl"], still_url);
        let suggestions: Value = parse(
            &b.send(get(&format!("/api/v1/autocomplete/icons?query={name}")))
                .await,
        );
        assert_eq!(suggestions["icons"][0]["stillUrl"], still_url);
    }
    let list: Value = parse(&b.send(get("/api/v1/admin/icons")).await);
    assert_eq!(list["animatedLimit"], 250);
    assert_eq!(list["animatedUsage"], 2);
}

#[tokio::test]
async fn animated_emoji_rejects_corrupt_disguised_and_oversized_media() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let large = vec![0; 256 * 1024 + 1];
    for (bytes, filename, content_type) in [
        (
            b"this is not a PNG image".as_slice(),
            "fake.png",
            "image/png",
        ),
        (
            b"GIF89a\x40\x00\x40\x00\x80\x00\x00".as_slice(),
            "broken.gif",
            "image/gif",
        ),
        (
            b"RIFF\x04\x00\x00\x00WEBPgarbage".as_slice(),
            "broken.webp",
            "image/webp",
        ),
        (&GIF[..GIF.len() / 2], "truncated.gif", "image/gif"),
        (&GIF[..GIF.len() - 20], "broken_last_frame.gif", "image/gif"),
        (
            &WEBP[..WEBP.len() - 20],
            "broken_last_frame.webp",
            "image/webp",
        ),
        (large.as_slice(), "large.gif", "image/gif"),
    ] {
        let signed = upload(&a, bytes, filename, content_type).await;
        let reply = write(
            &mut b,
            Method::POST,
            "/api/v1/admin/icons",
            json!({"name": "invalid_emoji", "title": "Invalid", "signedId": signed}),
        )
        .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{filename}: {}",
            reply.text()
        );
        assert!(
            error(&reply)["fields"]["image"]
                .as_array()
                .is_some_and(|errors| !errors.is_empty())
        );
    }
    let list: Value = parse(&b.send(get("/api/v1/admin/icons")).await);
    assert_eq!(list["animatedUsage"], 0);
    assert!(list["icons"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn animated_emoji_capacity_is_atomic_for_simultaneous_uploads() {
    let Some(a) = app().await else { return };
    a.db().write(|tx| {
        tx.conn().execute("UPDATE accounts SET settings = json_set(COALESCE(settings, '{}'), '$.animated_emoji_limit', 1)", [])?;
        Ok(())
    }).await.unwrap();
    let signed = upload(&a, GIF, "dance.gif", "image/gif").await;
    let mut first = a.sign_in(DAVID).await;
    let mut second = a.sign_in(DAVID).await;
    let (first, second) = tokio::join!(
        write(
            &mut first,
            Method::POST,
            "/api/v1/admin/icons",
            json!({"name": "concurrent_one", "title": "One", "signedId": signed})
        ),
        write(
            &mut second,
            Method::POST,
            "/api/v1/admin/icons",
            json!({"name": "concurrent_two", "title": "Two", "signedId": signed})
        ),
    );
    let mut statuses = [first.status.as_u16(), second.status.as_u16()];
    statuses.sort();
    assert_eq!(statuses, [200, 422]);
    assert_eq!(
        a.db()
            .read(campfire_db::models::workspace_icon::WorkspaceIcon::animated_usage)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn animated_emoji_capacity_returns_422_and_static_uploads_remain_unlimited() {
    let Some(a) = app().await else { return };
    a.db().write(|tx| {
        tx.conn().execute("UPDATE accounts SET settings = json_set(COALESCE(settings, '{}'), '$.animated_emoji_limit', 1)", [])?;
        Ok(())
    }).await.unwrap();
    let mut b = a.sign_in(DAVID).await;
    let signed = upload(&a, GIF, "dance.gif", "image/gif").await;
    for (name, expected) in [
        ("dance_one", StatusCode::OK),
        ("dance_two", StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let reply = write(
            &mut b,
            Method::POST,
            "/api/v1/admin/icons",
            json!({"name": name, "title": "Dance", "signedId": signed}),
        )
        .await;
        assert_eq!(reply.status, expected, "{}", reply.text());
        if expected == StatusCode::UNPROCESSABLE_ENTITY {
            assert_eq!(
                error(&reply)["fields"]["image"],
                json!(["animated emoji capacity reached (limit: 1)"])
            );
        }
    }
    for (name, bytes, filename, content_type) in [
        ("still_png", PNG, "still.png", "image/png"),
        (
            "still_svg",
            include_bytes!("../../../../fixtures/files/workspace_icons/clean.svg").as_slice(),
            "still.svg",
            "image/svg+xml",
        ),
    ] {
        let signed = upload(&a, bytes, filename, content_type).await;
        let reply = write(
            &mut b,
            Method::POST,
            "/api/v1/admin/icons",
            json!({"name": name, "title": "Still", "signedId": signed}),
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let list: Value = parse(&reply);
        assert_eq!(list["animatedLimit"], 1);
        assert_eq!(list["animatedUsage"], 1);
        let icon = list["icons"]
            .as_array()
            .unwrap()
            .iter()
            .find(|icon| icon["name"] == name)
            .unwrap();
        assert_eq!(icon["animated"], false);
        assert_eq!(icon["stillUrl"], icon["imageUrl"]);
    }
    let list: Value = parse(&b.send(get("/api/v1/admin/icons")).await);
    assert_eq!(list["icons"].as_array().unwrap().len(), 3);
    let id = list["icons"]
        .as_array()
        .unwrap()
        .iter()
        .find(|icon| icon["name"] == "dance_one")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let reply = write(
        &mut b,
        Method::DELETE,
        &format!("/api/v1/admin/icons/{id}"),
        Value::Null,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(parse::<Value>(&reply)["animatedUsage"], 0);
    let reply = write(
        &mut b,
        Method::POST,
        "/api/v1/admin/icons",
        json!({"name": "dance_two", "title": "Dance", "signedId": signed}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
}
