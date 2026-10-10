use super::*;
use campfire_db::User;
use crate::controllers::presenters;

#[test]
fn concurrent_first_requests_all_get_the_whole_file() {
    let threads: Vec<_> = (0..16).map(|_| std::thread::spawn(|| std::fs::read(asset_file("default-bot-avatar.svg").unwrap()).unwrap())).collect();
    let contents: Vec<Vec<u8>> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert!(!contents[0].is_empty());
    assert!(contents.iter().all(|c| c == &contents[0]));
}

#[tokio::test]
async fn signed_default_bot_avatar_serves_retained_bytes_and_headers() {
    use axum::http::{Method, StatusCode};
    use crate::controllers::presenters::test_support::{TestApp, Req, BENDER};
    let app = TestApp::boot_frozen().await.expect("seed required");
    let user = app.db().read(|conn| User::find(conn, BENDER)).await.unwrap();
    let path = presenters::user_summary(&app.booted.app.secrets, &user).avatar_path;
    let mut browser = app.david();
    let shown = browser.send(Req::new(Method::GET, &path)).await;
    assert_eq!(shown.status, StatusCode::OK);
    assert_eq!(shown.body, include_bytes!("../../../../../static_assets/media/images/default-bot-avatar.svg"));
    assert_eq!(shown.header("content-type"), Some("image/svg+xml"));
    assert_eq!(shown.header("cache-control"), Some("max-age=1800, public, stale-while-revalidate=604800"));
    let head = browser.send(Req::new(Method::HEAD, &path)).await;
    assert_eq!(head.status, StatusCode::OK);
    assert!(head.body.is_empty());
    assert_eq!(head.header("content-length"), shown.header("content-length"));
}

#[tokio::test]
async fn default_initials_svg_matches_rails_bytes_and_cache_validation() {
    use axum::http::{Method, StatusCode};
    use crate::controllers::presenters::test_support::{TestApp, Req, DAVID};
    let Some(app) = TestApp::boot_frozen().await else { return };
    let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../../../vectors/users_avatars.json")).unwrap();
    let user = app.db().read(move |conn| User::find(conn, DAVID)).await.unwrap();
    let token = presenters::user_summary(&app.booted.app.secrets, &user).avatar_path;
    let mut browser = app.david();
    for vector in vectors["cases"].as_array().unwrap() {
        let name = vector["name"].as_str().unwrap().to_string();
        app.db().write(move |tx| {
            let mut user = User::find(tx.conn(), DAVID)?;
            user.update(tx, campfire_db::UserChanges { name: Some(name), ..Default::default() })
        }).await.unwrap();
        let user = app.db().read(move |conn| User::find(conn, DAVID)).await.unwrap();
        assert_eq!(user.initials(), vector["initials"].as_str().unwrap());
        assert_eq!(campfire_presentation::helpers::initials(&user.name), user.initials());
        let response = browser.send(Req::new(Method::GET, &token).header("accept", "image/svg+xml")).await;
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.text(), vector["body"].as_str().unwrap(), "name: {}", vector["name"]);
        assert_eq!(response.header("cache-control"), vector["cache_control"].as_str());
        assert_eq!(response.header("etag"), vector["etag"].as_str());
        let fresh = browser.send(Req::new(Method::GET, &token).header("accept", "image/svg+xml").header("if-none-match", response.header("etag").unwrap())).await;
        assert_eq!(fresh.status, StatusCode::NOT_MODIFIED);
        assert!(fresh.body.is_empty());
    }
}
