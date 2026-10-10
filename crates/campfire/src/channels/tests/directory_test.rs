//! Our Rails callback frames compared byte-for-byte through the booted app and real sockets.
use super::*;
use crate::controllers::presenters::test_support::{BENDER_KEY, JASON, KEVIN};
use campfire_db::{NewSession, Session};
use campfire_kit::Crypto;
use serde_json::Value;
const JZ: i64 = 773523953;

async fn frozen_hub() -> (Hub, i64) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let id = app
        .db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE sessions SET two_factor_verified_at=created_at WHERE user_id=?",
                [DAVID],
            )?;
            Ok(Room::find_or_create_direct_for(tx, &[DAVID, JASON, KEVIN, JZ], KEVIN)?.id)
        })
        .await
        .unwrap();
    let listener = super::super::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (
        Hub {
            app,
            url: format!("ws://{addr}/cable"),
            origin: format!("http://{addr}"),
        },
        id,
    )
}

pub(super) async fn connect_user(hub: &Hub, user_id: i64) -> Client {
    let session = hub
        .app
        .db()
        .write(move |tx| {
            Session::start_with(
                tx,
                user_id,
                NewSession {
                    two_factor_verified: true,
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let signed = campfire_kit::RailsCrypto::new(hub.app.booted.app.secrets.clone()).sign_cookie(
        "session_token",
        &session.token,
        None,
    );
    let cookie = format!("session_token={}", campfire_kit::cookies::escape(&signed));
    let mut client = hub.connect(&cookie).await;
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    client
}

#[tokio::test]
async fn directory_http_guards_leave_room_and_recipients_unchanged() {
    let (hub, id) = frozen_hub().await;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../views/tests/golden/rooms/directory.json"
    ))
    .unwrap();
    let guards = &fixture["guards"];
    let mut recipient = hub.david().await;
    let own = identifier(json!({"channel":"ReadRoomsChannel"}));
    recipient.confirm(&own).await;
    let path = format!("/rooms/directs/{id}");
    let anonymous = hub
        .app
        .anonymous()
        .send(Req::new(Method::PATCH, &path))
        .await;
    assert_eq!(
        anonymous.status.as_u16(),
        guards["anonymous"].as_u64().unwrap() as u16
    );
    let bot = hub
        .app
        .anonymous()
        .send(Req::new(
            Method::PATCH,
            &format!("{path}?bot_key={BENDER_KEY}"),
        ))
        .await;
    assert_eq!(bot.status.as_u16(), guards["bot"].as_u64().unwrap() as u16);
    let outsider = hub
        .app
        .db()
        .write(|tx| {
            Ok(User::create(
                tx,
                campfire_db::NewUser {
                    name: "Directory Outsider".into(),
                    email_address: Some("directory-outsider@example.test".into()),
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    let denied = hub
        .app
        .sign_in(outsider)
        .await
        .write(Req::new(Method::PATCH, &path).form(&[("room[name]", "Stolen")]))
        .await;
    assert_eq!(
        denied.status.as_u16(),
        guards["outsider"].as_u64().unwrap() as u16
    );
    assert_eq!(denied.location(), guards["outsider_location"].as_str());
    assert_eq!(
        hub.app
            .db()
            .read(move |conn| Ok(Room::find(conn, id)?.name))
            .await
            .unwrap(),
        None
    );
    recipient.assert_silent().await;
}
