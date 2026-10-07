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
    let mut gif = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff".to_vec();
    gif.extend_from_slice(b"!\xff\x0bNETSCAPE2.0\x03\x01\x00\x00\x00");
    for pixel in [0x44, 0x4c] {
        gif.extend_from_slice(
            b"!\xf9\x04\x00\x0a\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02",
        );
        gif.extend_from_slice(&[pixel, 0x01, 0x00]);
    }
    gif.push(b';');
    gif
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
