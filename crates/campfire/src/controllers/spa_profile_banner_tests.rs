//! A person's profile banner over the seeded app: checked as the workspace banner is, served by
//! avatar token with an animated image's still, carried on every user DTO, and published with
//! `user.updated`.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::admin_tests::{app, error, get, parse, spa, upload, write};
use super::api_tests::{Sync, serve};
use super::workspace_branding_tests::{BRANDING_TESTS, animated_gif, upload_bytes};
use crate::controllers::presenters::test_support::{DAVID, JASON};

const BANNER: &str = "/api/v1/settings/banner";

async fn directory_user(
    b: &mut crate::controllers::presenters::test_support::Browser<'_>,
    id: i64,
) -> api::User {
    let list: api::UserList = parse(&b.send(get(&format!("/api/v1/users?ids={id}"))).await);
    list.users.into_iter().find(|user| user.id == id).unwrap()
}

#[tokio::test]
async fn a_profile_banner_is_checked_as_the_workspace_banner_is() {
    let _branding = BRANDING_TESTS.lock().await;
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let forged = write(&mut b, Method::PUT, BANNER, json!({"signedId": "forged"})).await;
    assert_eq!(
        forged.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        forged.text()
    );
    assert!(error(&forged)["fields"]["signedId"].is_array());

    let (text, _) = upload_bytes(&a, b"plain words", "notes.txt", "text/plain").await;
    let refused = write(&mut b, Method::PUT, BANNER, json!({"signedId": text})).await;
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        error(&refused)["fields"]["signedId"],
        json!(["must be a PNG, JPEG, GIF or WebP image"])
    );

    let wide = super::workspace_branding_tests::png(4097, 1, false);
    let (wide, _) = upload_bytes(&a, &wide, "wide.png", "image/png").await;
    let refused = write(&mut b, Method::PUT, BANNER, json!({"signedId": wide})).await;
    assert_eq!(
        error(&refused)["fields"]["signedId"],
        json!(["must be at most 4096 × 2304 pixels"])
    );

    // An image already in use elsewhere (here, the avatar) can't also become the banner.
    let avatar = upload(&a, "me.png").await;
    let _: api::Settings = spa(
        &mut b,
        Method::PUT,
        "/api/v1/settings/avatar",
        json!({"signedId": avatar}),
    )
    .await;
    let reused = write(&mut b, Method::PUT, BANNER, json!({"signedId": avatar})).await;
    assert_eq!(
        error(&reused)["fields"]["signedId"],
        json!(["must be an image you uploaded"])
    );
    let settings: Value = parse(&b.send(get("/api/v1/settings")).await);
    assert_eq!(settings["profile"]["bannerUrl"], Value::Null);
}

#[tokio::test]
async fn a_profile_banner_is_served_carried_and_removed() {
    let _branding = BRANDING_TESTS.lock().await;
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let mut viewer = a.sign_in(JASON).await;
    assert_eq!(directory_user(&mut viewer, DAVID).await.banner_url, None);

    let still = upload(&a, "banner.png").await;
    let settings: api::Settings =
        spa(&mut b, Method::PUT, BANNER, json!({"signedId": still})).await;
    let url = settings.profile.banner_url.clone().unwrap();
    assert!(url.contains("/banner?v="), "{url}");
    assert_eq!(settings.profile.banner_still_url, None);
    let user = directory_user(&mut viewer, DAVID).await;
    assert_eq!(user.banner_url.as_deref(), Some(url.as_str()));
    assert_eq!(user.banner_still_url, None);
    let served = viewer.get(&url).await;
    assert_eq!(served.status, StatusCode::OK);
    assert_eq!(served.content_type(), Some("image/png"));
    assert_eq!(
        viewer.get(&format!("{url}&still=1")).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(a.anonymous().get(&url).await.status, StatusCode::FOUND);

    let (gif, _) = upload_bytes(&a, &animated_gif(), "moving.gif", "image/gif").await;
    let settings: api::Settings = spa(&mut b, Method::PUT, BANNER, json!({"signedId": gif})).await;
    let moving = settings.profile.banner_url.clone().unwrap();
    assert_ne!(moving, url);
    let first_frame = settings.profile.banner_still_url.clone().unwrap();
    let user = directory_user(&mut viewer, DAVID).await;
    assert_eq!(user.banner_still_url.as_deref(), Some(first_frame.as_str()));
    let served = viewer.get(&moving).await;
    assert_eq!(served.content_type(), Some("image/gif"));
    let served = viewer.get(&first_frame).await;
    assert_eq!(served.status, StatusCode::OK);
    assert_eq!(served.content_type(), Some("image/png"));

    let removed: api::Settings = spa(&mut b, Method::DELETE, BANNER, Value::Null).await;
    assert_eq!(removed.profile.banner_url, None);
    assert_eq!(removed.profile.banner_still_url, None);
    assert_eq!(directory_user(&mut viewer, DAVID).await.banner_url, None);
    assert_eq!(viewer.get(&moving).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn banner_and_avatar_changes_reach_other_people() {
    let _branding = BRANDING_TESTS.lock().await;
    let Some(a) = app().await else { return };
    let (addr, server) = serve(&a).await;
    let mut b = a.sign_in(DAVID).await;
    let observer = a.sign_in(JASON).await;
    let mut sync = Sync::connect(addr, &observer.cookie_header(), &[]).await;
    sync.welcome().await;
    let updated = |banner: bool| {
        move |event: &api::SyncEvent| {
            matches!(&event.payload, api::SyncPayload::UserUpdated(user)
                if user.id == DAVID && user.banner_url.is_some() == banner)
        }
    };
    let private =
        |event: &api::SyncEvent| matches!(event.payload, api::SyncPayload::SettingsUpdated(_));

    let signed = upload(&a, "banner.png").await;
    let settings: api::Settings =
        spa(&mut b, Method::PUT, BANNER, json!({"signedId": signed})).await;
    let event = sync.until(updated(true), private).await;
    let api::SyncPayload::UserUpdated(user) = event.payload else {
        unreachable!()
    };
    assert_eq!(user.banner_url, settings.profile.banner_url);

    let _: api::Settings = spa(&mut b, Method::DELETE, BANNER, Value::Null).await;
    sync.until(updated(false), private).await;

    let avatar = upload(&a, "me.png").await;
    let _: api::Settings = spa(
        &mut b,
        Method::PUT,
        "/api/v1/settings/avatar",
        json!({"signedId": avatar}),
    )
    .await;
    let event = sync
        .until(
            |event| matches!(&event.payload, api::SyncPayload::UserUpdated(user) if user.id == DAVID && user.has_avatar),
            private,
        )
        .await;
    let api::SyncPayload::UserUpdated(user) = event.payload else {
        unreachable!()
    };
    assert!(user.avatar_url.contains("/avatar?v="));
    server.abort();
}
