use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Room};
#[tokio::test]
async fn inbound_controls_are_visible_only_to_a_human_room_administrator() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let path = format!("/rooms/opens/{HQ}/edit");
    let reply = app.sign_in(KEVIN).await.get(&path).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(!reply.text().contains("id=\"inbound-email\""));
    let mut anonymous = app.anonymous();
    assert_eq!(
        anonymous.get(&path).await.location(),
        Some("http://campfire.test/session/new")
    );
    assert_eq!(
        anonymous
            .get(&format!("{path}?bot_key={BENDER_KEY}"))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let mut david = app.david();
    let reply = david.get(&path).await;
    assert!(reply.text().contains("id=\"inbound-email\""));
    assert!(
        reply
            .text()
            .contains("Inbound email is not configured for this workspace.")
    );
    assert!(
        !david
            .get(&format!("/rooms/directs/{DIRECT_DAVID_JASON}/edit"))
            .await
            .text()
            .contains("id=\"inbound-email\"")
    );
}
#[tokio::test]
async fn inbound_section_shows_created_rotated_and_cleared_addresses_on_real_edit_pages() {
    let app = TestApp::boot_frozen_with_env(&[("INBOUND_EMAIL_DOMAIN", "mail.campfire.test")])
        .await
        .expect("seed required");
    let mut david = app.david();
    let edit = format!("/rooms/opens/{HQ}/edit");
    let action = format!("/rooms/{HQ}/inbound_email_address");
    assert!(
        david
            .get(&edit)
            .await
            .text()
            .contains("Create email address")
    );
    let mut previous = None;
    for _ in 0..2 {
        let reply = david.write(Req::new(Method::POST, &action)).await;
        assert_eq!(reply.status, StatusCode::FOUND);
        assert_eq!(
            reply.location(),
            Some(format!("http://campfire.test/rooms/{HQ}/edit").as_str())
        );
        let token = app
            .db()
            .read(|conn| Ok(Room::find(conn, HQ)?.inbound_email_token.unwrap()))
            .await
            .unwrap();
        assert_eq!(token.len(), 32);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(previous.as_ref(), Some(&token));
        let page = david.get(&edit).await;
        assert!(
            page.text()
                .contains(&format!("room-{token}@mail.campfire.test"))
        );
        assert!(page.text().contains("Rotate address"));
        assert!(page.text().contains("The old address stops working."));
        previous = Some(token);
    }
    app.db()
        .write(|tx| {
            tx.conn()
                .execute_cached("UPDATE rooms SET inbound_email_token=NULL WHERE id=?", [HQ])?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        david
            .get(&edit)
            .await
            .text()
            .contains("Create email address")
    );
}
