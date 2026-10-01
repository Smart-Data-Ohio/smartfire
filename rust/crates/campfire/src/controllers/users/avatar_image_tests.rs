//! Uploaded image and unresizable fallback criteria from Users::AvatarsControllerTest.
use crate::controllers::presenters::test_support::{DAVID, Req, TestApp};
use axum::http::{Method, StatusCode};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use campfire_db::User;
use sha2::{Digest, Sha256};

async fn check(file: &'static str) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_avatar_images.json"
    ))
    .unwrap();
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["file"] == file)
        .unwrap();
    // Reuse the committed Rails upload fixtures, rather than any local reference or scratch file.
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vectors/users_logos")
            .join(file),
    )
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        case["input_sha256"].as_str().unwrap()
    );
    let mut browser = app.david();
    let uploaded = browser
        .write(Req::new(Method::PATCH, "/users/me/profile").multipart(
            &[],
            (
                "user[avatar]",
                file,
                if file.ends_with("jpg") {
                    "image/jpeg"
                } else {
                    "image/bmp"
                },
                &bytes,
            ),
        ))
        .await;
    assert_eq!(uploaded.status, StatusCode::FOUND);
    assert_eq!(
        uploaded.location(),
        Some("http://campfire.test/users/me/profile")
    );
    let user = app.db().read(|conn| User::find(conn, DAVID)).await.unwrap();
    let path =
        crate::controllers::presenters::user_summary(&app.booted.app.secrets, &user).avatar_path;
    let shown = browser
        .send(Req::new(Method::GET, &path).header("accept", "image/svg+xml"))
        .await;
    assert_eq!(
        shown.status.as_u16(),
        case["status"].as_u64().unwrap() as u16
    );
    assert_eq!(
        shown.body,
        STANDARD.decode(case["body"].as_str().unwrap()).unwrap(),
        "{file}: complete Rails body"
    );
    for (header, key) in [
        ("content-type", "content_type"),
        ("cache-control", "cache_control"),
        ("etag", "etag"),
    ] {
        assert_eq!(shown.header(header), case[key].as_str(), "{file}: {header}");
    }
    let fresh = browser
        .send(
            Req::new(Method::GET, &path)
                .header("accept", "image/svg+xml")
                .header("if-none-match", shown.header("etag").unwrap()),
        )
        .await;
    // The upload's notice contributes to the first ETag and is swept by that GET in both
    // apps. The next request gets the stable validator; only then is a conditional GET 304.
    assert_eq!(
        fresh.status.as_u16(),
        case["fresh_status"].as_u64().unwrap() as u16
    );
    assert_eq!(
        fresh.body,
        STANDARD
            .decode(case["fresh_body"].as_str().unwrap())
            .unwrap()
    );
    assert_eq!(fresh.header("etag"), case["fresh_etag"].as_str());
    let stable = browser
        .send(
            Req::new(Method::GET, &path)
                .header("accept", "image/svg+xml")
                .header("if-none-match", fresh.header("etag").unwrap()),
        )
        .await;
    assert_eq!(
        stable.status.as_u16(),
        case["stable_status"].as_u64().unwrap() as u16
    );
    assert_eq!(stable.status, StatusCode::NOT_MODIFIED);
    assert_eq!(
        stable.body,
        STANDARD
            .decode(case["stable_body"].as_str().unwrap())
            .unwrap()
    );
}

#[tokio::test]
async fn uploaded_avatar_image_uses_rails_bytes_headers_and_freshness() {
    check("moon.jpg").await;
}

#[tokio::test]
async fn unresizable_avatar_falls_back_to_rails_initials_bytes_and_headers() {
    check("pixel.bmp").await;
}
