//! The profile write contract formerly exercised by the sidebar cache oracle.
use crate::controllers::presenters::test_support::{TestApp, Req, DAVID};
use axum::http::{Method, StatusCode};

#[tokio::test]
async fn profile_zone_patch_persists() {
    let t = TestApp::boot_frozen().await.unwrap();
    let response = t.david().write(Req::new(Method::PATCH, "/users/me/profile")
        .form(&[("user[time_zone]", "Asia/Tokyo")])).await;
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(response.location(), Some("http://campfire.test/users/me/profile"));
    assert_eq!(t.db().read(|conn| Ok(campfire_db::models::user::profile_settings::appearance(conn, DAVID)?.time_zone)).await.unwrap().as_deref(), Some("Asia/Tokyo"));
}
