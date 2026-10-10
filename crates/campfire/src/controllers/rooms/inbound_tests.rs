use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Room};
#[tokio::test]
async fn inbound_address_creation_and_rotation_persist_distinct_tokens() {
    let app = TestApp::boot_frozen_with_env(&[("INBOUND_EMAIL_DOMAIN", "mail.campfire.test")])
        .await
        .expect("seed required");
    let mut david = app.david();
    let action = format!("/rooms/{HQ}/inbound_email_address");
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
}
