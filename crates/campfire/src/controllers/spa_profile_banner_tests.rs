//! A person's profile banner over the seeded app: checked as the workspace banner is, served by
//! avatar token with an animated image's still, carried on every user DTO, and published with
//! `user.updated`.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::admin_tests::{app, error, get, owned_upload, parse, set_uploader, spa, upload, write};
use super::api_tests::{Sync, serve};
use super::workspace_branding_tests::{BRANDING_TESTS, animated_gif, upload_bytes};
use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, JASON, KEVIN, TestApp};

const BANNER: &str = "/api/v1/settings/banner";

const REFUSED: &str = "must be an image you uploaded";

/// `upload_bytes`, as `owner`'s own direct upload: its signed id and blob id.
async fn owned_bytes(
    a: &TestApp,
    bytes: &[u8],
    filename: &str,
    content_type: &str,
    owner: i64,
) -> (String, i64) {
    let (signed_id, id) = upload_bytes(a, bytes, filename, content_type).await;
    set_uploader(a, id, owner).await;
    (signed_id, id)
}

/// Attaches blob `id` as `name` on `record_type` `record_id`, as another save would.
async fn attach(
    a: &TestApp,
    id: i64,
    record_type: &'static str,
    record_id: i64,
    name: &'static str,
) {
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "INSERT INTO active_storage_attachments (name, record_type, record_id, blob_id, created_at) \
                 VALUES (?1, ?2, ?3, ?4, '2026-03-02 15:30:00')",
                (name, record_type, record_id, id),
            )?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn attachments_of(a: &TestApp, user: i64) -> i64 {
    a.db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM active_storage_attachments WHERE record_type = 'User' AND record_id = ?1",
                [user],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap()
}

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

    let (text, _) = owned_bytes(&a, b"plain words", "notes.txt", "text/plain", DAVID).await;
    let refused = write(&mut b, Method::PUT, BANNER, json!({"signedId": text})).await;
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        error(&refused)["fields"]["signedId"],
        json!(["must be a PNG, JPEG, GIF or WebP image"])
    );

    let wide = super::workspace_branding_tests::png(4097, 1, false);
    let (wide, _) = owned_bytes(&a, &wide, "wide.png", "image/png", DAVID).await;
    let refused = write(&mut b, Method::PUT, BANNER, json!({"signedId": wide})).await;
    assert_eq!(
        error(&refused)["fields"]["signedId"],
        json!(["must be at most 4096 × 2304 pixels"])
    );

    // An image already in use elsewhere (here, the avatar) can't also become the banner.
    let avatar = owned_upload(&a, "me.png", DAVID).await;
    let _: api::Settings = spa(
        &mut b,
        Method::PUT,
        "/api/v1/settings/avatar",
        json!({"signedId": avatar}),
    )
    .await;
    let reused = write(&mut b, Method::PUT, BANNER, json!({"signedId": avatar})).await;
    assert_eq!(error(&reused)["fields"]["signedId"], json!([REFUSED]));
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

    let still = owned_upload(&a, "banner.png", DAVID).await;
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

    let (gif, _) = owned_bytes(&a, &animated_gif(), "moving.gif", "image/gif", DAVID).await;
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

    let signed = owned_upload(&a, "banner.png", DAVID).await;
    let settings: api::Settings =
        spa(&mut b, Method::PUT, BANNER, json!({"signedId": signed})).await;
    let event = sync.until(updated(true), private).await;
    let api::SyncPayload::UserUpdated(user) = event.payload else {
        unreachable!()
    };
    assert_eq!(user.banner_url, settings.profile.banner_url);

    let _: api::Settings = spa(&mut b, Method::DELETE, BANNER, Value::Null).await;
    sync.until(updated(false), private).await;

    let avatar = owned_upload(&a, "me.png", DAVID).await;
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

#[tokio::test]
async fn only_your_own_unattached_upload_can_become_your_banner() {
    let _branding = BRANDING_TESTS.lock().await;
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;

    // A blob with no uploader on record (a legacy one) is nobody's, so it isn't David's.
    let ownerless = upload(&a, "legacy.png").await;
    let refused = write(&mut b, Method::PUT, BANNER, json!({"signedId": ownerless})).await;
    assert_eq!(
        refused.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        refused.text()
    );
    assert_eq!(error(&refused)["fields"]["signedId"], json!([REFUSED]));

    let theirs = owned_upload(&a, "jason.png", JASON).await;
    let refused = write(&mut b, Method::PUT, BANNER, json!({"signedId": theirs})).await;
    assert_eq!(error(&refused)["fields"]["signedId"], json!([REFUSED]));

    // David's own upload, once a message holds it, stays the message's.
    let (posted, posted_id) = owned_bytes(
        &a,
        include_bytes!("../../../../fixtures/files/workspace_icons/square_64.png"),
        "posted.png",
        "image/png",
        DAVID,
    )
    .await;
    attach(&a, posted_id, "Message", ALL_TALK, "attachment").await;
    let refused = write(&mut b, Method::PUT, BANNER, json!({"signedId": posted})).await;
    assert_eq!(error(&refused)["fields"]["signedId"], json!([REFUSED]));
    assert_eq!(attachments_of(&a, DAVID).await, 0);

    // The avatar takes the same rule.
    let refused = write(
        &mut b,
        Method::PUT,
        "/api/v1/settings/avatar",
        json!({"signedId": ownerless}),
    )
    .await;
    assert_eq!(error(&refused)["fields"]["signedId"], json!([REFUSED]));
    let refused = write(
        &mut b,
        Method::PUT,
        "/api/v1/settings/avatar",
        json!({"signedId": posted}),
    )
    .await;
    assert_eq!(error(&refused)["fields"]["signedId"], json!([REFUSED]));
    let settings: Value = parse(&b.send(get("/api/v1/settings")).await);
    assert_eq!(settings["profile"]["bannerUrl"], Value::Null);
    assert_eq!(settings["profile"]["avatarAttached"], json!(false));
}

#[tokio::test]
async fn a_removed_persons_banner_is_purged_and_no_longer_served() {
    let _branding = BRANDING_TESTS.lock().await;
    let Some(a) = app().await else { return };
    let mut kevin = a.sign_in(KEVIN).await;
    let mut viewer = a.sign_in(JASON).await;
    let signed = owned_upload(&a, "banner.png", KEVIN).await;
    let settings: api::Settings =
        spa(&mut kevin, Method::PUT, BANNER, json!({"signedId": signed})).await;
    let url = settings.profile.banner_url.unwrap();
    assert_eq!(viewer.get(&url).await.status, StatusCode::OK);

    a.db()
        .write(|tx| campfire_db::User::find(tx.conn(), KEVIN)?.destroy(tx))
        .await
        .unwrap();
    assert_eq!(
        attachments_of(&a, KEVIN).await,
        0,
        "the banner goes with them"
    );
    assert_eq!(viewer.get(&url).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_banner_left_on_a_missing_person_is_not_served() {
    let _branding = BRANDING_TESTS.lock().await;
    let Some(a) = app().await else { return };
    let mut viewer = a.sign_in(JASON).await;
    let gone = 987_654_321;
    let (_, blob) = owned_bytes(
        &a,
        include_bytes!("../../../../fixtures/files/workspace_icons/square_64.png"),
        "orphan.png",
        "image/png",
        gone,
    )
    .await;
    attach(&a, blob, "User", gone, "banner").await;
    let token = campfire_runtime::presenters::avatar_token(&a.booted.app.secrets, gone);
    let reply = viewer.get(&format!("/users/{token}/banner?v={blob}")).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}
