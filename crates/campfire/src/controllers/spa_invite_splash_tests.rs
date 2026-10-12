use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::{BRANDING_TESTS, animated_gif, png, upload_bytes};
use crate::controllers::presenters::test_support::{DAVID, KEVIN};
use crate::controllers::spa::admin_tests::{app, error, get, parse, spa, write};

const PATH: &str = "/api/v1/admin/workspace/invite_splash";

#[tokio::test]
async fn invite_splash_lifecycle_and_signed_out_branding() {
    let _branding = BRANDING_TESTS.lock().await;
    let a = app().await.expect("restore the frozen seeds");
    let mut admin = a.sign_in(DAVID).await;
    let initial: api::Workspace = parse(&admin.send(get("/api/v1/admin/workspace")).await);
    assert_eq!(initial.invite_splash_url, None);
    let about = a
        .anonymous()
        .send(get("/api/v1/public_pages/about"))
        .await
        .json();
    assert_eq!(about["workspace"]["inviteSplashUrl"], Value::Null);

    let _: api::Workspace = spa(
        &mut admin,
        Method::PATCH,
        "/api/v1/admin/workspace",
        json!({"name":"Harbor", "description":"Our community"}),
    )
    .await;
    let (logo, _) = upload_bytes(&a, &png(32, 32, false), "logo.png", "image/png").await;
    let logo: api::Workspace = spa(
        &mut admin,
        Method::PUT,
        "/api/v1/admin/workspace/logo",
        json!({"signedId":logo}),
    )
    .await;
    let (first, first_id) = upload_bytes(&a, &png(160, 90, false), "splash.png", "image/png").await;
    let first: api::Workspace = spa(&mut admin, Method::PUT, PATH, json!({"signedId":first})).await;
    let first_url = first.invite_splash_url.unwrap();
    assert_eq!(first_url, format!("/account/invite_splash?v={first_id}"));
    let public = a.anonymous().get(&first_url).await;
    assert_eq!(public.status, StatusCode::OK, "{}", public.text());
    assert_eq!(public.content_type(), Some("image/png"));
    assert_eq!(&public.body[..8], b"\x89PNG\r\n\x1a\n");
    assert!(public.header("cache-control").unwrap().contains("public"));
    let etag = public.header("etag").unwrap();
    assert_eq!(
        a.anonymous()
            .send(get(&first_url).header("if-none-match", etag))
            .await
            .status,
        StatusCode::NOT_MODIFIED
    );

    let code = a
        .db()
        .read(|conn| Ok(campfire_db::Account::first(conn)?.unwrap().join_code))
        .await
        .unwrap();
    for path in [
        format!("/api/v1/join/{code}"),
        "/api/v1/session/boot".into(),
        "/api/v1/public_pages/about".into(),
    ] {
        let reply = a.anonymous().send(get(&path)).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.text());
        let workspace = reply.json()["workspace"].clone();
        assert_eq!(workspace["name"], "Harbor");
        assert_eq!(workspace["description"], "Our community");
        assert_eq!(workspace["logoUrl"], logo.logo_url);
        assert_eq!(workspace["inviteSplashUrl"], first_url);
        assert_eq!(workspace.as_object().unwrap().len(), 4);
    }

    // Animated inputs use the existing banner pipeline, but public decoration stays still.
    let (second, second_id) = upload_bytes(&a, &animated_gif(), "splash.gif", "image/gif").await;
    let second: api::Workspace =
        spa(&mut admin, Method::PUT, PATH, json!({"signedId":second})).await;
    let second_url = second.invite_splash_url.unwrap();
    assert_eq!(
        second_url,
        format!("/account/invite_splash?v={second_id}&still=1")
    );
    assert_ne!(second_url, first_url);
    let public = a.anonymous().get(&second_url).await;
    assert_eq!(public.status, StatusCode::OK);
    assert_eq!(public.content_type(), Some("image/png"));
    let blob_ids = a.db().read(|conn| {
        let mut query = conn.prepare("SELECT blob_id FROM active_storage_attachments WHERE record_type='Account' AND name='invite_splash'")?;
        Ok(query.query_map([], |row| row.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }).await.unwrap();
    assert_eq!(blob_ids, vec![second_id]);
    let refreshed = a
        .anonymous()
        .send(get(&first_url).header("if-none-match", etag))
        .await;
    assert_eq!(refreshed.status, StatusCode::OK);

    let removed: api::Workspace = spa(&mut admin, Method::DELETE, PATH, Value::Null).await;
    assert_eq!(removed.invite_splash_url, None);
    assert_eq!(removed.logo_url, logo.logo_url);
    assert_eq!(
        a.anonymous().get("/account/invite_splash").await.status,
        StatusCode::NOT_FOUND
    );
    let about = a
        .anonymous()
        .send(get("/api/v1/public_pages/about"))
        .await
        .json();
    assert_eq!(about["workspace"]["inviteSplashUrl"], Value::Null);
    let audits = a
        .db()
        .read(|conn| {
            let mut query = conn.prepare(
                "SELECT details FROM audit_logs WHERE action='account.settings.change' ORDER BY id",
            )?;
            Ok(query
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
        .unwrap()
        .into_iter()
        .map(|row| serde_json::from_str::<Value>(&row).unwrap())
        .filter(|row| row.get("invite_splash").is_some())
        .collect::<Vec<_>>();
    assert_eq!(
        audits,
        vec![
            json!({"invite_splash":{"before":null,"after":first_id}}),
            json!({"invite_splash":{"before":first_id,"after":second_id}}),
            json!({"invite_splash":{"before":second_id,"after":null}}),
        ]
    );
}

#[tokio::test]
async fn invite_splash_permissions_and_validation_preserve_the_attachment() {
    let _branding = BRANDING_TESTS.lock().await;
    let a = app().await.expect("restore the frozen seeds");
    let (signed, _) = upload_bytes(&a, &png(160, 90, false), "splash.png", "image/png").await;
    let mut admin = a.sign_in(DAVID).await;
    let saved: api::Workspace =
        spa(&mut admin, Method::PUT, PATH, json!({"signedId":signed})).await;
    let mut member = a.sign_in(KEVIN).await;
    for method in [Method::PUT, Method::DELETE] {
        assert_eq!(
            write(
                &mut member,
                method.clone(),
                PATH,
                json!({"signedId":signed})
            )
            .await
            .status,
            StatusCode::FORBIDDEN
        );
        let anonymous = a
            .anonymous()
            .send(crate::controllers::spa::admin_tests::json_body(
                method,
                PATH,
                &json!({"signedId":signed}),
            ))
            .await;
        assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
    }
    let (text, _) = upload_bytes(&a, b"text", "text.txt", "text/plain").await;
    let (corrupt, _) = upload_bytes(&a, b"not PNG bytes", "bad.png", "image/png").await;
    let (wide, _) = upload_bytes(&a, &png(4097, 1, false), "wide.png", "image/png").await;
    let (large, id) = upload_bytes(&a, &png(16, 9, false), "large.png", "image/png").await;
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE active_storage_blobs SET byte_size=? WHERE id=?",
                [10 * 1024 * 1024 + 1, id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    for (signed, message) in [
        ("forged", "isn't an uploaded file"),
        (text.as_str(), "must be a PNG, JPEG, GIF or WebP image"),
        (corrupt.as_str(), "must be a PNG, JPEG, GIF or WebP image"),
        (wide.as_str(), "must be at most 4096 × 2304 pixels"),
        (large.as_str(), "must be 10 MB or smaller"),
    ] {
        let reply = write(&mut admin, Method::PUT, PATH, json!({"signedId":signed})).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert_eq!(error(&reply)["fields"]["signedId"], json!([message]));
        let current: api::Workspace = parse(&admin.send(get("/api/v1/admin/workspace")).await);
        assert_eq!(current.invite_splash_url, saved.invite_splash_url);
    }
}
