//! Recipient-specific row permissions through the real sidebar endpoint/cache.
use crate::controllers::presenters::test_support::*;
use axum::http::StatusCode;
use campfire_db::Room;

#[test]
fn direct_labels_match_rails_ascii_word_splitting() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../views/tests/golden/rooms/directory.json"
    ))
    .unwrap();
    for case in fixture["labels"].as_array().unwrap() {
        let members: Vec<_> = case["names"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| campfire_views::users::UserSummary {
                name: name.as_str().unwrap().into(),
                ..Default::default()
            })
            .collect();
        let actual = crate::controllers::presenters::accounts::sidebar_direct_label(None, &members);
        assert_eq!(
            actual,
            case["label"].as_str().unwrap(),
            "{:?}",
            case["names"]
        );
    }
}

#[tokio::test]
async fn seeded_group_row_matches_rails_membership_order() {
    let app = TestApp::boot().await.expect("seed required");
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../views/tests/golden/sidebar/rows.json"
    ))
    .unwrap();
    let row = fixtures
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "direct_seed_group")
        .unwrap();
    let reply = app.david().get(&campfire_routes::user_sidebar()).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        reply.text().contains(row["html"].as_str().unwrap()),
        "seeded sidebar row differs from Rails bytes"
    );
}

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
