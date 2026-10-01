use crate::controllers::presenters::test_support::*;
use askama::Template;
use axum::http::{Method, StatusCode};
use serde_json::Value;
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/users_welcome.json")).unwrap()
}
#[tokio::test]
async fn welcome_redirects_to_original_or_last_visible_room_like_rails() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut browser = app.david();
    assert_eq!(
        browser.get("/").await.location(),
        vectors()["first"]["location"].as_str()
    );
    browser.absorb_cookie_header("last_room=486777696");
    assert_eq!(
        browser.get("/").await.location(),
        vectors()["last"]["location"].as_str()
    );
    browser.absorb_cookie_header("last_room=340026324");
    assert_eq!(
        browser.get("/").await.location(),
        vectors()["first"]["location"].as_str()
    );
}
#[tokio::test]
async fn empty_workspace_body_sidebar_and_frame_match_rails() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    for block in ["html", "sidebar"] {
        let actual = crate::controllers::users::people_tests::render_with(
            &app,
            |_| {},
            |ctx| {
                let page = campfire_views::welcome::Show {
                    ctx,
                    current_user_name: "David".into(),
                };
                if block == "html" {
                    page.as_content().render().unwrap()
                } else {
                    page.as_sidebar().render().unwrap()
                }
            },
        );
        if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(format!("{dir}/welcome-{block}.actual"), &actual).unwrap();
            std::fs::write(
                format!("{dir}/welcome-{block}.expected"),
                vectors()["page"][block].as_str().unwrap(),
            )
            .unwrap();
        }
        assert_eq!(
            actual,
            vectors()["page"][block].as_str().unwrap(),
            "{block}: complete Rails bytes"
        );
    }
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM memberships WHERE user_id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = app.david().get("/").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.text().contains("No rooms yet"));
    assert!(reply.text().contains("workspace-navigation__open"));
    let frame = app
        .david()
        .send(Req::new(Method::GET, "/").header("turbo-frame", "welcome"))
        .await;
    assert_eq!(frame.status, StatusCode::OK);
    assert!(!frame.text().contains("<!DOCTYPE"));
    assert!(
        frame
            .text()
            .contains(vectors()["page"]["html"].as_str().unwrap())
    );
}
