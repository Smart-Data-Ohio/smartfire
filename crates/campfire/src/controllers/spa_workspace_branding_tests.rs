//! Workspace banner, animated image delivery and branding sync over the seeded app.

use axum::http::{Method, StatusCode};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::admin_tests::{app, classic, error, get, parse, spa, upload, write};
use super::api_tests::{Sync, serve};
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, TestApp};

/// Two 1x1 frames, black then white, looping forever.
fn animated_gif() -> Vec<u8> {
    gif_with_canvas(1, 1, 2)
}

fn gif_with_canvas(width: u16, height: u16, frames: usize) -> Vec<u8> {
    let mut gif = b"GIF89a".to_vec();
    gif.extend_from_slice(&width.to_le_bytes());
    gif.extend_from_slice(&height.to_le_bytes());
    gif.extend_from_slice(b"\x80\x00\x00\x00\x00\x00\xff\xff\xff");
    gif.extend_from_slice(b"!\xff\x0bNETSCAPE2.0\x03\x01\x00\x00\x00");
    for pixel in [0x44, 0x4c].into_iter().cycle().take(frames) {
        gif.extend_from_slice(
            b"!\xf9\x04\x00\x0a\x00\x00\x00,\x00\x00\x00\x00",
        );
        gif.extend_from_slice(&width.to_le_bytes());
        gif.extend_from_slice(&height.to_le_bytes());
        gif.extend_from_slice(b"\x00\x02\x02");
        gif.extend_from_slice(&[pixel, 0x01, 0x00]);
    }
    gif.push(b';');
    gif
}

fn png_chunk(png: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut crc = u32::MAX;
    for &byte in name.iter().chain(data) {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    png.extend_from_slice(name);
    png.extend_from_slice(data);
    png.extend_from_slice(&(!crc).to_be_bytes());
}

/// Real, compressible RGB PNGs, optionally with two APNG frames.
fn png(width: u32, height: u32, apng: bool) -> Vec<u8> {
    use std::io::Write;

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = width.to_be_bytes().to_vec();
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    png_chunk(&mut png, b"IHDR", &header);
    if apng {
        png_chunk(&mut png, b"acTL", &[0, 0, 0, 2, 0, 0, 0, 0]);
    }
    for frame in 0u32..if apng { 2 } else { 1 } {
        if apng {
            let mut control = frame.to_be_bytes().to_vec();
            control.extend_from_slice(&width.to_be_bytes());
            control.extend_from_slice(&height.to_be_bytes());
            control.extend_from_slice(&[0; 8]);
            control.extend_from_slice(&[0, 1, 0, 10, 0, 0]);
            png_chunk(&mut png, b"fcTL", &control);
        }
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        let row = vec![if frame == 0 { 0 } else { 255 }; width as usize * 3];
        for _ in 0..height {
            encoder.write_all(&[0]).unwrap();
            encoder.write_all(&row).unwrap();
        }
        let compressed = encoder.finish().unwrap();
        if frame == 0 {
            png_chunk(&mut png, b"IDAT", &compressed);
        } else {
            let mut data = (frame * 2).to_be_bytes().to_vec();
            data.extend_from_slice(&compressed);
            png_chunk(&mut png, b"fdAT", &data);
        }
    }
    png_chunk(&mut png, b"IEND", &[]);
    png
}

async fn upload_bytes(
    a: &TestApp,
    bytes: &[u8],
    filename: &str,
    content_type: &str,
) -> (String, i64) {
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
    (
        campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None),
        blob.id,
    )
}

async fn banner_audits(a: &TestApp) -> Vec<Value> {
    a.db().read(|conn| {
        let mut query = conn.prepare("SELECT details FROM audit_logs WHERE action = 'account.settings.change' ORDER BY id")?;
        Ok(query.query_map([], |row| row.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?
            .into_iter().map(|value| serde_json::from_str::<Value>(&value).unwrap()).filter(|value| value.get("banner").is_some()).collect())
    }).await.unwrap()
}

#[tokio::test]
async fn spa_workspace_branding_permissions_and_validation() {
    let Some(a) = app().await else { return };
    let banner = "/api/v1/admin/workspace/banner";
    let signed = upload(&a, "banner.png").await;
    let mut member = a.sign_in(KEVIN).await;
    for method in [Method::PUT, Method::DELETE] {
        let reply = write(
            &mut member,
            method.clone(),
            banner,
            json!({"signedId": signed}),
        )
        .await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN);
        let reply = a
            .anonymous()
            .send(super::admin_tests::json_body(
                method,
                banner,
                &json!({"signedId": signed}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    }
    let (text, _) = upload_bytes(&a, b"not an image", "file.txt", "text/plain").await;
    let (large, id) = upload_bytes(
        &a,
        include_bytes!("../../../../fixtures/files/workspace_icons/square_64.png"),
        "large.png",
        "image/png",
    )
    .await;
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE active_storage_blobs SET byte_size = ? WHERE id = ?",
                [10 * 1024 * 1024 + 1, id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut admin = a.sign_in(DAVID).await;
    for kind in ["logo", "banner"] {
        let path = format!("/api/v1/admin/workspace/{kind}");
        for (signed, message) in [
            ("forged", "isn't an uploaded file"),
            (text.as_str(), "must be a PNG, JPEG, GIF or WebP image"),
            (large.as_str(), "must be 10 MB or smaller"),
        ] {
            let reply = write(&mut admin, Method::PUT, &path, json!({"signedId": signed})).await;
            assert_eq!(
                reply.status,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{}",
                reply.text()
            );
            assert_eq!(error(&reply)["fields"]["signedId"], json!([message]));
        }
    }
    let initial: api::Workspace = parse(&admin.send(get("/api/v1/admin/workspace")).await);
    assert!(!initial.logo_attached);
    assert_eq!(initial.banner_url, None);
    assert_eq!(banner_audits(&a).await, Vec::<Value>::new());
}

#[tokio::test]
async fn spa_workspace_branding_validates_bytes_dimensions_and_frame_budget() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let oversized = png(5000, 10, false);
    let too_tall = png(10, 5000, false);
    let banner_width = png(4097, 100, false);
    let banner_height = png(100, 2305, false);
    let excessive_frames = gif_with_canvas(4096, 2304, 11);
    for kind in ["logo", "banner"] {
        let dimension_message = if kind == "logo" {
            "must be at most 4096 × 4096 pixels"
        } else {
            "must be at most 4096 × 2304 pixels"
        };
        let mut cases = vec![
            (
                b"this is text, not a PNG".as_slice(),
                "image/png",
                "must be a PNG, JPEG, GIF or WebP image",
            ),
            (
                b"GIF89a\x01\x00\x01\x00\x80\x00\x00".as_slice(),
                "image/gif",
                "must be a PNG, JPEG, GIF or WebP image",
            ),
            (oversized.as_slice(), "image/png", dimension_message),
            (too_tall.as_slice(), "image/png", dimension_message),
            (
                excessive_frames.as_slice(),
                "image/gif",
                "must contain at most 100 megapixels across all frames",
            ),
        ];
        if kind == "banner" {
            cases.push((&banner_width, "image/png", dimension_message));
            cases.push((&banner_height, "image/png", dimension_message));
        }
        for (bytes, content_type, message) in cases {
            let (signed, _) = upload_bytes(&a, bytes, "image.png", content_type).await;
            let reply = write(
                &mut admin,
                Method::PUT,
                &format!("/api/v1/admin/workspace/{kind}"),
                json!({"signedId": signed}),
            )
            .await;
            assert_eq!(
                reply.status,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{}",
                reply.text()
            );
            assert_eq!(error(&reply)["fields"]["signedId"], json!([message]));
            for path in ["/api/v1/boot", "/api/v1/admin/workspace", "/app/"] {
                assert_eq!(admin.get(path).await.status, StatusCode::OK, "{path}");
            }
            let workspace: api::Workspace =
                parse(&admin.send(get("/api/v1/admin/workspace")).await);
            assert!(!workspace.logo_attached);
            assert_eq!(workspace.banner_url, None);
        }
    }
    assert!(banner_audits(&a).await.is_empty());
}

#[tokio::test]
async fn spa_workspace_branding_corrupt_frames_degrade_to_static() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let mut bytes = animated_gif();
    // The header and two frame records remain readable; the first frame's LZW data is invalid.
    let at = bytes
        .windows(5)
        .position(|window| window == [2, 2, 0x44, 1, 0])
        .unwrap();
    bytes[at + 2..at + 4].copy_from_slice(&[0xff, 0xff]);
    for kind in ["logo", "banner"] {
        let (signed, id) = upload_bytes(&a, &bytes, "broken.gif", "image/gif").await;
        let workspace: api::Workspace = spa(
            &mut admin,
            Method::PUT,
            &format!("/api/v1/admin/workspace/{kind}"),
            json!({"signedId": signed}),
        )
        .await;
        let url = if kind == "logo" {
            assert_eq!(workspace.logo_still_url, None);
            workspace.logo_url
        } else {
            assert_eq!(workspace.banner_still_url, None);
            workspace.banner_url.unwrap()
        };
        a.db()
            .read(move |conn| {
                let blob = campfire_storage::Blob::find(conn, id).unwrap().unwrap();
                assert_eq!(
                    blob.metadata.get(campfire_storage::branding::ANIMATED_KEY),
                    Some(&campfire_storage::Json::Bool(false))
                );
                assert_eq!(
                    conn.query_row(
                        "SELECT COUNT(*) FROM active_storage_variant_records WHERE blob_id = ?",
                        [id],
                        |row| row.get::<_, i64>(0)
                    )?,
                    0
                );
                Ok(())
            })
            .await
            .unwrap();
        for path in ["/api/v1/boot", "/api/v1/admin/workspace", "/app/"] {
            assert_eq!(admin.get(path).await.status, StatusCode::OK, "{path}");
        }
        let boot: Value = parse(&admin.send(get("/api/v1/boot")).await);
        assert_eq!(boot["account"][format!("{kind}StillUrl")], Value::Null);
        assert_eq!(admin.get(&url).await.status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn spa_workspace_branding_legacy_metadata_and_broken_sources_never_break_rendering() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    for kind in ["logo", "banner"] {
        let (signed, id) = upload_bytes(&a, &animated_gif(), "animation.gif", "image/gif").await;
        let workspace: api::Workspace = spa(
            &mut admin,
            Method::PUT,
            &format!("/api/v1/admin/workspace/{kind}"),
            json!({"signedId": signed}),
        )
        .await;
        let still = if kind == "logo" {
            workspace.logo_still_url.unwrap()
        } else {
            workspace.banner_still_url.unwrap()
        };
        let storage = a.booted.app.storage.clone();
        let (source_key, still_key) = a
            .db()
            .read(move |conn| {
                let blob = campfire_storage::Blob::find(conn, id).unwrap().unwrap();
                let image_kind = if kind == "logo" {
                    campfire_storage::branding::Kind::Logo
                } else {
                    campfire_storage::branding::Kind::Banner
                };
                let image = storage
                    .existing_variant(conn, &blob, &image_kind.still_variation())
                    .unwrap()
                    .unwrap();
                assert!(campfire_storage::branding::animated(&blob));
                Ok((blob.key, image.key))
            })
            .await
            .unwrap();
        a.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE active_storage_blobs SET metadata = '{}' WHERE id = ?",
                    [id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let workspace: api::Workspace = parse(&admin.send(get("/api/v1/admin/workspace")).await);
        let boot: Value = parse(&admin.send(get("/api/v1/boot")).await);
        assert_eq!(boot["account"][format!("{kind}StillUrl")], Value::Null);
        if kind == "logo" {
            assert_eq!(workspace.logo_still_url, None);
            assert!(!workspace.logo_url.contains("animated=1"));
        } else {
            assert_eq!(workspace.banner_still_url, None);
        }
        assert_eq!(admin.get("/app/").await.status, StatusCode::OK);
        assert_eq!(admin.get(&still).await.status, StatusCode::NOT_FOUND);

        // A stale animated flag and a damaged source must never be consulted by a presenter.
        a.db().write(move |tx| {
            tx.conn().execute(
                "UPDATE active_storage_blobs SET metadata = '{\"branding_animated\":true}' WHERE id = ?", [id],
            )?;
            Ok(())
        }).await.unwrap();
        std::fs::write(
            a.booted.app.storage.service.path_for(&source_key),
            b"broken GIF",
        )
        .unwrap();
        for path in ["/api/v1/boot", "/api/v1/admin/workspace", "/app/"] {
            assert_eq!(admin.get(path).await.status, StatusCode::OK, "{path}");
        }
        let prepared = admin.get(&still).await;
        assert_eq!(prepared.status, StatusCode::OK);
        assert_eq!(prepared.content_type(), Some("image/png"));
        std::fs::remove_file(a.booted.app.storage.service.path_for(&still_key)).unwrap();
        assert_eq!(admin.get(&still).await.status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn spa_workspace_branding_apng_is_static_and_served_as_png() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let bytes = png(2, 2, true);
    for kind in ["logo", "banner"] {
        let (signed, id) = upload_bytes(&a, &bytes, "animation.png", "image/png").await;
        let workspace: api::Workspace = spa(
            &mut admin,
            Method::PUT,
            &format!("/api/v1/admin/workspace/{kind}"),
            json!({"signedId": signed}),
        )
        .await;
        let url = if kind == "logo" {
            assert_eq!(workspace.logo_still_url, None);
            workspace.logo_url
        } else {
            assert_eq!(workspace.banner_still_url, None);
            workspace.banner_url.unwrap()
        };
        let response = admin.get(&url).await;
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.content_type(), Some("image/png"));
        assert_eq!(&response.body[..8], b"\x89PNG\r\n\x1a\n");
        a.db()
            .read(move |conn| {
                let blob = campfire_storage::Blob::find(conn, id).unwrap().unwrap();
                assert_eq!(
                    blob.metadata.get(campfire_storage::branding::ANIMATED_KEY),
                    Some(&campfire_storage::Json::Bool(false))
                );
                Ok(())
            })
            .await
            .unwrap();
        let boot: Value = parse(&admin.send(get("/api/v1/boot")).await);
        assert_eq!(boot["account"][format!("{kind}StillUrl")], Value::Null);
    }
}

#[tokio::test]
async fn spa_workspace_branding_banner_lifecycle_boot_and_audits() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let boot: Value = parse(&b.send(get("/api/v1/boot")).await);
    for key in ["logoUrl", "logoStillUrl", "bannerUrl", "bannerStillUrl"] {
        assert_eq!(boot["account"][key], Value::Null, "{key}");
    }
    let first = upload(&a, "banner.png").await;
    let first: api::Workspace = spa(
        &mut b,
        Method::PUT,
        "/api/v1/admin/workspace/banner",
        json!({"signedId":first}),
    )
    .await;
    let url = first.banner_url.unwrap();
    assert!(url.starts_with("/account/banner?v="));
    assert_eq!(first.banner_still_url, None);
    let response = b.get(&url).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.content_type(), Some("image/png"));
    assert_eq!(&response.body[..8], b"\x89PNG\r\n\x1a\n");
    let etag = response.header("etag").unwrap();
    let cached = b.send(get(&url).header("if-none-match", etag)).await;
    assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
    let boot: Value = parse(&b.send(get("/api/v1/boot")).await);
    assert_eq!(boot["account"]["bannerUrl"], url);
    let second = upload(&a, "replacement.png").await;
    let second: api::Workspace = spa(
        &mut b,
        Method::PUT,
        "/api/v1/admin/workspace/banner",
        json!({"signedId":second}),
    )
    .await;
    assert_ne!(second.banner_url.as_ref().unwrap(), &url);
    // The frozen clock never advanced: this version is independent of account.updated_at.
    let refreshed = b.send(get(&url).header("if-none-match", etag)).await;
    assert_eq!(refreshed.status, StatusCode::OK);
    let removed: api::Workspace = spa(
        &mut b,
        Method::DELETE,
        "/api/v1/admin/workspace/banner",
        Value::Null,
    )
    .await;
    assert_eq!(removed.banner_url, None);
    assert_eq!(removed.banner_still_url, None);
    assert_eq!(b.get("/account/banner").await.status, StatusCode::NOT_FOUND);
    let changes = banner_audits(&a).await;
    assert_eq!(changes.len(), 3);
    assert_eq!(changes[0]["banner"]["before"], Value::Null);
    assert!(changes[0]["banner"]["after"].is_number());
    assert_ne!(
        changes[1]["banner"]["before"],
        changes[1]["banner"]["after"]
    );
    assert_eq!(changes[2]["banner"], json!({"before":true,"after":false}));
}

#[tokio::test]
async fn spa_workspace_branding_static_banner_resizes_and_keeps_jpeg() {
    let Some(a) = app().await else { return };
    let (signed, _) = upload_bytes(
        &a,
        include_bytes!("../../../../fixtures/files/black_hole.jpg"),
        "banner.jpg",
        "image/jpeg",
    )
    .await;
    let mut b = a.sign_in(DAVID).await;
    let workspace: api::Workspace = spa(
        &mut b,
        Method::PUT,
        "/api/v1/admin/workspace/banner",
        json!({"signedId":signed}),
    )
    .await;
    assert_eq!(workspace.banner_still_url, None);
    let reply = b.get(workspace.banner_url.as_ref().unwrap()).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.content_type(), Some("image/jpeg"));
    let image = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(image.path(), &reply.body).unwrap();
    let image = campfire_storage::vips::Image::open_sequential(image.path()).unwrap();
    assert_eq!((image.width(), image.height()), (1920, 1080));
}

#[tokio::test]
async fn spa_workspace_branding_animated_sources_and_png_stills() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let gif = animated_gif();
    // The same two frames encoded as animated WebP, without ancillary metadata.
    let webp = STANDARD.decode("UklGRpQAAABXRUJQVlA4WAoAAAACAAAAAAAAAAAAQU5JTQYAAAD/////AABBTk1GMAAAAAAAAAAAAAAAAAAAAGQAAAJWUDggGAAAADABAJ0BKgEAAQACADQlpAADcAD++/1QAEFOTUYwAAAAAAAAAAAAAAAAAAAAZAAAAFZQOCAYAAAANAEAnQEqAQABAAAANCWkAANwAP77lAAA").unwrap();
    for (bytes, filename, content_type) in [
        (&gif, "animation.gif", "image/gif"),
        (&webp, "animation.webp", "image/webp"),
    ] {
        for kind in ["logo", "banner"] {
            let (signed, _) = upload_bytes(&a, bytes, filename, content_type).await;
            let workspace: api::Workspace = spa(
                &mut b,
                Method::PUT,
                &format!("/api/v1/admin/workspace/{kind}"),
                json!({"signedId": signed}),
            )
            .await;
            let (url, still) = if kind == "logo" {
                (workspace.logo_url, workspace.logo_still_url.unwrap())
            } else {
                (
                    workspace.banner_url.unwrap(),
                    workspace.banner_still_url.unwrap(),
                )
            };
            let animated = b.get(&url).await;
            assert_eq!(animated.status, StatusCode::OK);
            assert_eq!(animated.content_type(), Some(content_type));
            assert_eq!(&animated.body, bytes, "original contains both frames");
            let png = b.get(&still).await;
            assert_eq!(png.status, StatusCode::OK);
            assert_eq!(png.content_type(), Some("image/png"));
            assert_eq!(&png.body[..8], b"\x89PNG\r\n\x1a\n");
            assert_ne!(animated.header("etag"), png.header("etag"));
            let boot: Value = parse(&b.send(get("/api/v1/boot")).await);
            assert_eq!(boot["account"][format!("{kind}Url")], url);
            assert_eq!(boot["account"][format!("{kind}StillUrl")], still);
        }
    }
    let classic_logo = b.get("/account/logo").await;
    assert_eq!(classic_logo.content_type(), Some("image/png"));
    for kind in ["logo", "banner"] {
        let signed = upload(&a, "static.png").await;
        let workspace: api::Workspace = spa(
            &mut b,
            Method::PUT,
            &format!("/api/v1/admin/workspace/{kind}"),
            json!({"signedId":signed}),
        )
        .await;
        assert_eq!(
            if kind == "logo" {
                workspace.logo_still_url
            } else {
                workspace.banner_still_url
            },
            None
        );
    }
    let (legacy, _) = upload_bytes(
        &a,
        include_bytes!("../../../../fixtures/files/pixel.bmp"),
        "legacy.bmp",
        "image/bmp",
    )
    .await;
    classic(
        &mut b,
        Method::PATCH,
        "/account",
        &[("account[logo]", &legacy)],
    )
    .await;
    let workspace: api::Workspace = parse(&b.send(get("/api/v1/admin/workspace")).await);
    assert!(workspace.logo_attached);
    assert_eq!(workspace.logo_still_url, None);
    let spa_logo = b.get(&workspace.logo_url).await;
    let classic_logo = b.get("/account/logo").await;
    assert_eq!(spa_logo.status, StatusCode::OK);
    assert_eq!(
        spa_logo.body, classic_logo.body,
        "legacy logos retain the classic stock-icon fallback"
    );
}

async fn media_slots_are_free() {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let (available, capacity) = campfire_web::active_storage::media_permits();
            if available == capacity {
                assert_eq!(available, capacity);
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("cancelled decoder must stop and release its media permit");
}

#[tokio::test]
async fn spa_workspace_branding_deadline_stops_headers_and_decodes_and_releases_slots() {
    use campfire_storage::branding::test_hooks;
    use campfire_storage::vips::StallPhase;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    for kind in ["logo", "banner"] {
        for phase in [StallPhase::Header, StallPhase::Evaluation] {
            media_slots_are_free().await;
            let (signed, id) = upload_bytes(&a, &animated_gif(), "stalled.gif", "image/gif").await;
            let key = a
                .db()
                .read(move |conn| Ok(campfire_storage::Blob::find(conn, id).unwrap().unwrap().key))
                .await
                .unwrap();
            let stall = test_hooks::stall(&key, phase);
            let started = Instant::now();
            let reply = tokio::time::timeout(
                Duration::from_secs(2),
                write(
                    &mut admin,
                    Method::PUT,
                    &format!("/api/v1/admin/workspace/{kind}"),
                    json!({"signedId": signed}),
                ),
            )
            .await
            .expect("upload deadline must answer the request");
            assert_eq!(
                reply.status,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{}",
                reply.text()
            );
            assert_eq!(
                error(&reply)["fields"]["signedId"],
                json!(["couldn't be read as an image"])
            );
            assert!(
                stall.reached.load(Ordering::Relaxed),
                "the real libvips callback was entered"
            );
            media_slots_are_free().await;
            assert!(started.elapsed() < Duration::from_secs(2));
            drop(stall);

            // Exercise pixel evaluation again, with the same limiter after cancellation.
            let (signed, _) = upload_bytes(&a, &animated_gif(), "normal.gif", "image/gif").await;
            let workspace: api::Workspace = spa(
                &mut admin,
                Method::PUT,
                &format!("/api/v1/admin/workspace/{kind}"),
                json!({"signedId": signed}),
            )
            .await;
            let still = if kind == "logo" {
                workspace.logo_still_url
            } else {
                workspace.banner_still_url
            };
            assert_eq!(admin.get(&still.unwrap()).await.status, StatusCode::OK);
        }
    }
}

#[tokio::test]
async fn spa_workspace_branding_static_cache_miss_cancels_and_releases_slots() {
    use campfire_storage::branding::test_hooks;
    use campfire_storage::vips::StallPhase;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    for kind in ["logo", "banner"] {
        let (signed, id) = upload_bytes(&a, &png(2, 2, false), "static.png", "image/png").await;
        let workspace: api::Workspace = spa(
            &mut admin,
            Method::PUT,
            &format!("/api/v1/admin/workspace/{kind}"),
            json!({"signedId": signed}),
        )
        .await;
        let url = if kind == "logo" {
            workspace.logo_url
        } else {
            workspace.banner_url.unwrap()
        };
        let key = a
            .db()
            .read(move |conn| {
                assert_eq!(
                    conn.query_row(
                        "SELECT COUNT(*) FROM active_storage_variant_records WHERE blob_id = ?",
                        [id],
                        |row| row.get::<_, i64>(0)
                    )?,
                    0
                );
                Ok(campfire_storage::Blob::find(conn, id).unwrap().unwrap().key)
            })
            .await
            .unwrap();
        media_slots_are_free().await;
        let stall = test_hooks::stall(&key, StallPhase::Evaluation);
        let response = tokio::time::timeout(Duration::from_secs(2), admin.get(&url))
            .await
            .unwrap();
        assert_eq!(response.status, StatusCode::NOT_FOUND);
        assert!(stall.reached.load(Ordering::Relaxed));
        media_slots_are_free().await;
        drop(stall);
        assert_eq!(
            admin.get(&url).await.status,
            StatusCode::OK,
            "retry an uncached, cancelled variant"
        );
    }
}

#[tokio::test]
async fn spa_workspace_branding_legacy_tiff_and_large_png_render_uncached_static_urls() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let input = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(input.path(), png(2, 2, false)).unwrap();
    let tiff = tempfile::Builder::new().suffix(".tiff").tempfile().unwrap();
    campfire_storage::vips::Image::open_sequential(input.path())
        .unwrap()
        .write_to_file(tiff.path())
        .unwrap();
    for (bytes, filename, content_type, expected_width) in [
        (
            std::fs::read(tiff.path()).unwrap(),
            "legacy.tiff",
            "image/tiff",
            2,
        ),
        (png(5000, 10, false), "legacy.png", "image/png", 512),
    ] {
        let (signed, id) = upload_bytes(&a, &bytes, filename, content_type).await;
        classic(
            &mut admin,
            Method::PATCH,
            "/account",
            &[("account[logo]", &signed)],
        )
        .await;
        a.db()
            .read(move |conn| {
                let blob = campfire_storage::Blob::find(conn, id).unwrap().unwrap();
                assert_eq!(
                    blob.metadata.get(campfire_storage::branding::ANIMATED_KEY),
                    None
                );
                assert_eq!(
                    conn.query_row(
                        "SELECT COUNT(*) FROM active_storage_variant_records WHERE blob_id = ?",
                        [id],
                        |row| row.get::<_, i64>(0)
                    )?,
                    0
                );
                Ok(())
            })
            .await
            .unwrap();
        let boot: Value = parse(&admin.get("/api/v1/boot").await);
        let workspace: api::Workspace = parse(&admin.get("/api/v1/admin/workspace").await);
        assert_eq!(boot["account"]["logoUrl"], workspace.logo_url);
        let response = admin
            .get(boot["account"]["logoUrl"].as_str().unwrap())
            .await;
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.content_type(), Some("image/png"));
        assert_eq!(&response.body[..8], b"\x89PNG\r\n\x1a\n");
        let output = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(output.path(), response.body).unwrap();
        let image = campfire_storage::vips::Image::open_sequential(output.path()).unwrap();
        assert_eq!(
            image.width(),
            expected_width,
            "converted the legacy source rather than serving the stock icon"
        );
        assert!(image.height() <= 512);
    }
}

#[tokio::test]
async fn spa_workspace_branding_legacy_pixel_budget_falls_back_to_stock_logo() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let stock = admin.get("/account/logo").await.body;
    // Only the header is needed: no allocation or decode of this 100+ MP canvas.
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = 10001u32.to_be_bytes().to_vec();
    header.extend_from_slice(&10000u32.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    png_chunk(&mut bytes, b"IHDR", &header);
    png_chunk(&mut bytes, b"IDAT", &[0]);
    png_chunk(&mut bytes, b"IEND", &[]);
    let (signed, id) = upload_bytes(&a, &bytes, "huge.png", "image/png").await;
    classic(
        &mut admin,
        Method::PATCH,
        "/account",
        &[("account[logo]", &signed)],
    )
    .await;
    let boot: Value = parse(&admin.get("/api/v1/boot").await);
    let response = admin
        .get(boot["account"]["logoUrl"].as_str().unwrap())
        .await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, stock);
    assert_eq!(admin.get("/account/logo").await.body, stock);
    a.db()
        .write(move |tx| {
            use crate::controllers::presenters::attachments::{self, Assignment, Record};
            let account = campfire_db::Account::first(tx.conn())?.unwrap();
            let blob = campfire_storage::Blob::find(tx.conn(), id)
                .unwrap()
                .unwrap();
            attachments::assign(
                tx,
                Record::account(account.id),
                "banner",
                Assignment::Existing(blob),
            )
        })
        .await
        .unwrap();
    let boot: Value = parse(&admin.get("/api/v1/boot").await);
    assert_eq!(
        admin
            .get(boot["account"]["bannerUrl"].as_str().unwrap())
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    media_slots_are_free().await;
}

#[tokio::test]
async fn spa_workspace_branding_legacy_cache_miss_cancels_with_stock_fallback() {
    use campfire_storage::branding::test_hooks;
    use campfire_storage::vips::StallPhase;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let stock = admin.get("/account/logo").await.body;
    let (signed, id) = upload_bytes(&a, &png(2, 2, false), "legacy.png", "image/png").await;
    classic(
        &mut admin,
        Method::PATCH,
        "/account",
        &[("account[logo]", &signed)],
    )
    .await;
    let key = a
        .db()
        .read(move |conn| Ok(campfire_storage::Blob::find(conn, id).unwrap().unwrap().key))
        .await
        .unwrap();
    let boot: Value = parse(&admin.get("/api/v1/boot").await);
    let url = boot["account"]["logoUrl"].as_str().unwrap();
    for path in [url, "/account/logo"] {
        media_slots_are_free().await;
        let stall = test_hooks::stall(&key, StallPhase::Evaluation);
        let reply = tokio::time::timeout(Duration::from_secs(2), admin.get(path))
            .await
            .unwrap();
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.body, stock);
        assert!(stall.reached.load(Ordering::Relaxed));
        media_slots_are_free().await;
    }
    let reply = admin.get(url).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_ne!(reply.body, stock);
}

async fn branding_still(a: &TestApp, id: i64, kind: &'static str) -> campfire_storage::Blob {
    let storage = a.booted.app.storage.clone();
    a.db()
        .read(move |conn| {
            let blob = campfire_storage::Blob::find(conn, id).unwrap().unwrap();
            let kind = if kind == "logo" {
                campfire_storage::branding::Kind::Logo
            } else {
                campfire_storage::branding::Kind::Banner
            };
            Ok(storage
                .existing_variant(conn, &blob, &kind.still_variation())
                .unwrap()
                .unwrap())
        })
        .await
        .unwrap()
}

async fn still_was_purged(a: &TestApp, still: campfire_storage::Blob) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while a.booted.app.storage.service.exist(&still.key) {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("purge job must delete the old still file");
    a.db()
        .read(move |conn| {
            assert!(
                campfire_storage::Blob::find(conn, still.id)
                    .unwrap()
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn spa_workspace_branding_replacing_and_removing_animations_purges_still_files() {
    use crate::controllers::presenters::test_support::SEED_NOW;
    let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    // Keep the real job runner: replacing/removing a source enqueues dependent purges.
    let Some(a) = TestApp::boot_seed_with_env("default", clock, &[("SPA_ENABLED", "1")]).await
    else {
        return;
    };
    let mut admin = a.sign_in(DAVID).await;
    for kind in ["logo", "banner"] {
        let mut previous = None;
        for _ in 0..2 {
            let (signed, id) =
                upload_bytes(&a, &animated_gif(), "animation.gif", "image/gif").await;
            let _: api::Workspace = spa(
                &mut admin,
                Method::PUT,
                &format!("/api/v1/admin/workspace/{kind}"),
                json!({"signedId": signed}),
            )
            .await;
            let still = branding_still(&a, id, kind).await;
            assert!(a.booted.app.storage.service.exist(&still.key));
            if let Some(previous) = previous.take() {
                still_was_purged(&a, previous).await;
            }
            previous = Some(still);
        }
        let _: api::Workspace = spa(
            &mut admin,
            Method::DELETE,
            &format!("/api/v1/admin/workspace/{kind}"),
            Value::Null,
        )
        .await;
        still_was_purged(&a, previous.unwrap()).await;
    }
}

async fn updated(sync: &mut Sync) -> api::WorkspaceBranding {
    let event = sync
        .until(
            |event| matches!(event.payload, api::SyncPayload::WorkspaceUpdated(_)),
            |_| false,
        )
        .await;
    assert_eq!(event.topic, "user");
    match event.payload {
        api::SyncPayload::WorkspaceUpdated(branding) => branding,
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn spa_workspace_branding_updates_reach_other_users_and_classic_writes() {
    let Some(a) = app().await else { return };
    let (addr, server) = serve(&a).await;
    let observer = a.sign_in(JASON).await;
    let mut sync = Sync::connect(addr, &observer.cookie_header(), &[]).await;
    sync.welcome().await;
    let mut b = a.sign_in(DAVID).await;
    let signed = upload(&a, "banner.png").await;
    let workspace: api::Workspace = spa(
        &mut b,
        Method::PUT,
        "/api/v1/admin/workspace/banner",
        json!({"signedId":signed}),
    )
    .await;
    let event = updated(&mut sync).await;
    assert_eq!(event.banner_url, workspace.banner_url);
    assert_eq!(event.name, workspace.name);
    assert_eq!(event.logo_url, None);
    assert_eq!(event.logo_still_url, None);
    assert_eq!(event.banner_still_url, None);
    let signed = upload(&a, "logo.png").await;
    let _: api::Workspace = spa(
        &mut b,
        Method::PUT,
        "/api/v1/admin/workspace/logo",
        json!({"signedId":signed}),
    )
    .await;
    assert!(updated(&mut sync).await.logo_url.is_some());
    let _: api::Workspace = spa(
        &mut b,
        Method::DELETE,
        "/api/v1/admin/workspace/logo",
        Value::Null,
    )
    .await;
    let removed = updated(&mut sync).await;
    assert_eq!(removed.logo_url, None);
    assert_eq!(removed.logo_still_url, None);
    assert_eq!(removed.banner_url, workspace.banner_url);
    let _: api::Workspace = spa(
        &mut b,
        Method::PATCH,
        "/api/v1/admin/workspace",
        json!({"name":"Renamed"}),
    )
    .await;
    assert_eq!(updated(&mut sync).await.name, "Renamed");
    let _: api::Workspace = spa(
        &mut b,
        Method::DELETE,
        "/api/v1/admin/workspace/banner",
        Value::Null,
    )
    .await;
    assert_eq!(updated(&mut sync).await.banner_url, None);
    let signed = upload(&a, "classic.png").await;
    classic(
        &mut b,
        Method::PATCH,
        "/account",
        &[("account[name]", "Classic"), ("account[logo]", &signed)],
    )
    .await;
    let event = updated(&mut sync).await;
    assert_eq!(event.name, "Classic");
    assert!(event.logo_url.is_some());
    classic(&mut b, Method::DELETE, "/account/logo", &[]).await;
    assert_eq!(updated(&mut sync).await.logo_url, None);
    server.abort();
}
