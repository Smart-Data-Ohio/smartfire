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
    let own = hub.turbo(&[&user_gid(DAVID).to_param(), "rooms"]);
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

#[tokio::test]
async fn group_directory_callbacks_match_rails_recipient_frames() {
    let (hub, id) = frozen_hub().await;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../views/tests/golden/rooms/directory.json"
    ))
    .unwrap();
    assert_eq!(id, fixture["setup"]["group_id"].as_i64().unwrap());
    let mut clients = Vec::new();
    for user in [DAVID, JASON, KEVIN, JZ] {
        let mut client = connect_user(&hub, user).await;
        let stream = format!("{}:rooms", user_gid(user).to_param());
        let identifier = hub.turbo(&[&stream]);
        client.confirm(&identifier).await;
        clients.push((user, stream, identifier, client));
    }
    let outsider = hub
        .app
        .db()
        .write(|tx| {
            Ok(User::create(
                tx,
                campfire_db::NewUser {
                    name: "Quiet Outsider".into(),
                    email_address: Some("quiet-outsider@example.test".into()),
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    let mut unrelated = connect_user(&hub, outsider).await;
    unrelated
        .confirm(&hub.turbo(&[&user_gid(outsider).to_param(), "rooms"]))
        .await;
    let mut actor = hub.app.sign_in(KEVIN).await;
    for op in fixture["operations"].as_array().unwrap() {
        let operation = op["name"].as_str().unwrap();
        let reply = match operation {
            "rename" => {
                actor
                    .write(
                        Req::new(Method::PATCH, &format!("/rooms/directs/{id}"))
                            .form(&[("room[name]", "Friday <&>")]),
                    )
                    .await
            }
            "clear_name" => {
                actor
                    .write(
                        Req::new(Method::PATCH, &format!("/rooms/directs/{id}"))
                            .form(&[("room[name]", "")]),
                    )
                    .await
            }
            "leave" => {
                hub.app
                    .sign_in(JZ)
                    .await
                    .write(Req::new(
                        Method::DELETE,
                        &format!("/rooms/directs/{id}/leave.json"),
                    ))
                    .await
            }
            "add" => {
                // The leaver reconnects before being added; the newcomer gets a prepend.
                let tuple = clients.iter_mut().find(|(user, ..)| *user == JZ).unwrap();
                tuple.3 = connect_user(&hub, JZ).await;
                tuple.3.confirm(&tuple.2).await;
                actor
                    .write(
                        Req::new(Method::POST, &format!("/rooms/directs/{id}/add_members"))
                            .form(&[("user_ids[]", &JZ.to_string())]),
                    )
                    .await
            }
            _ => unreachable!(),
        };
        assert!(
            reply.status.is_success() || reply.status.is_redirection(),
            "{operation}: {}",
            reply.text()
        );
        for (user, stream, identifier, client) in &mut clients {
            for expected in op["frames"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| f["stream"].as_str() == Some(stream.as_str()))
            {
                let frame =
                    tokio::time::timeout(std::time::Duration::from_secs(2), client.next_text())
                        .await
                        .expect("directory callback was dropped");
                let actual: Value = serde_json::from_str(&frame).unwrap();
                assert_eq!(actual["identifier"].as_str(), Some(identifier.as_str()));
                assert_eq!(
                    actual["message"], expected["html"],
                    "{operation} for {user}"
                );
                let html = actual["message"].as_str().unwrap();
                assert_eq!(campfire_cable::turbo::session_bound(html), None);
                assert!(!campfire_views::helpers::request_forgery::has_token_slots(
                    html
                ));
                if operation == "clear_name" && html.contains("room-header__identity") {
                    let header = html
                        .split_once("<template>")
                        .unwrap()
                        .1
                        .split_once("</template>")
                        .unwrap()
                        .0;
                    let shown = hub
                        .app
                        .sign_in(*user)
                        .await
                        .get(&format!("/rooms/{id}"))
                        .await;
                    assert_eq!(shown.status, 200);
                    assert!(
                        shown.text().contains(header),
                        "room page lost the recipient header/target for {user}"
                    );
                }
            }
            if operation == "leave" && *user == JZ {
                assert_eq!(
                    client.until_closed().await,
                    vec![DISCONNECT_RECONNECT.to_string()]
                );
            } else {
                client.assert_silent().await;
            }
        }
        unrelated.assert_silent().await;
    }
}

#[tokio::test]
async fn involvement_callbacks_match_rails_recipient_rows() {
    let (hub, _) = frozen_hub().await;
    let fixture: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../views/tests/golden/sidebar/involvement.json"
    ))
    .unwrap();
    let mut recipient = connect_user(&hub, DAVID).await;
    let identifier = hub.turbo(&[&user_gid(DAVID).to_param(), "rooms"]);
    recipient.confirm(&identifier).await;
    let mut unrelated = connect_user(&hub, JASON).await;
    unrelated
        .confirm(&hub.turbo(&[&user_gid(JASON).to_param(), "rooms"]))
        .await;
    let mut actor = hub.app.david();
    for operation in fixture {
        let id = operation["room_id"].as_i64().unwrap();
        let level = operation["level"].as_str().unwrap();
        let reply = actor
            .write(
                Req::new(Method::PUT, &format!("/rooms/{id}/involvement.json"))
                    .form(&[("involvement", level)]),
            )
            .await;
        assert_eq!(
            reply.status,
            axum::http::StatusCode::OK,
            "{id} {level}: {}",
            reply.text()
        );
        for expected in operation["frames"].as_array().unwrap() {
            let frame =
                tokio::time::timeout(std::time::Duration::from_secs(2), recipient.next_text())
                    .await
                    .expect("involvement frame missing");
            let actual: Value = serde_json::from_str(&frame).unwrap();
            assert_eq!(actual["identifier"].as_str(), Some(identifier.as_str()));
            assert_eq!(actual["message"], expected["html"], "room {id}, {level}");
            assert_eq!(
                campfire_cable::turbo::session_bound(actual["message"].as_str().unwrap()),
                None
            );
        }
        recipient.assert_silent().await;
        unrelated.assert_silent().await;
    }
}
