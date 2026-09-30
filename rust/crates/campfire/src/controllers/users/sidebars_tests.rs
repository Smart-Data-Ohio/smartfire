//! Recipient-specific row permissions through the real sidebar endpoint/cache.
use crate::controllers::presenters::test_support::*;
use axum::http::StatusCode;
use campfire_db::Room;

fn direct_link(html: &str, id: i64) -> String {
    let marker = format!("<a class=\"direct__link\" data-room-id=\"{id}\"");
    let start = html.find(&marker).expect("room's direct link");
    html[start..].split('>').next().unwrap().to_string()
}
#[tokio::test]
async fn sidebar_menu_uses_membership_viewer_and_invalidates_role_cache() {
    let app = TestApp::boot().await.expect("seed required");
    let id = app
        .db()
        .write(|tx| Ok(Room::find_or_create_direct_for(tx, &[DAVID, JASON, KEVIN], KEVIN)?.id))
        .await
        .unwrap();
    let david = app.david().get(&campfire_routes::user_sidebar()).await;
    assert_eq!(david.status, StatusCode::OK);
    assert!(direct_link(&david.text(), id).contains("data-menu-can-delete=\"true\""));
    let mut kevin = app.sign_in(KEVIN).await;
    let ordinary = kevin.get(&campfire_routes::user_sidebar()).await;
    assert_eq!(ordinary.status, StatusCode::OK);
    assert!(direct_link(&ordinary.text(), id).contains("data-menu-can-delete=\"false\""));
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET role=1 WHERE id=?", [KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    let promoted = kevin.get(&campfire_routes::user_sidebar()).await;
    assert_eq!(promoted.status, StatusCode::OK);
    assert!(direct_link(&promoted.text(), id).contains("data-menu-can-delete=\"true\""));
}
