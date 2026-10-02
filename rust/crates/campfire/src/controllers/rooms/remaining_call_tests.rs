//! Remaining Rails controller assertions, through HTTP and the real Cable hub.
use super::call_channel_broadcast_tests::{Socket, next, socket};
use super::call_channel_tests::configured;
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig};
use campfire_db::{CachedStatements, Membership, Room, RoomType, Session, StageRole};
use serde_json::Value;
const JZ: i64 = 773523953;
fn domain() -> HuddleConfig {
    HuddleConfig {
        api_secret: Some("ws13-fixture-api-secret".into()),
        admin_configured: true,
    }
}
async fn stage(test: &TestApp) -> (Room, i64, i64, i64) {
    test.db()
        .write(|tx| {
            tx.conn()
                .execute_cached("UPDATE users SET role=0 WHERE id=?", [JASON])?;
            let room = Room::create_for(
                tx,
                RoomType::Stage,
                Some("Town Hall"),
                DAVID,
                &[DAVID, JASON, KEVIN],
            )?;
            let host = Membership::find_by_room_and_user(tx.conn(), room.id, DAVID)?
                .unwrap()
                .id;
            let listener = Membership::find_by_room_and_user(tx.conn(), room.id, JASON)?
                .unwrap()
                .id;
            let speaker = Membership::find_by_room_and_user(tx.conn(), room.id, KEVIN)?
                .unwrap()
                .id;
            Membership::find(tx.conn(), speaker)?.change_stage_role_with_config(
                tx,
                StageRole::Speaker,
                &domain(),
            )?;
            Ok((room, host, listener, speaker))
        })
        .await
        .unwrap()
}
async fn grant(test: &TestApp, room: i64, user: i64, member: i64) -> HuddleGrant {
    test.db()
        .write(move |tx| {
            let session = Session::start(tx, user, None, None)?;
            HuddleGrant::issue(tx, session.id, member, room, &domain())
        })
        .await
        .unwrap()
}
async fn frames(test: &TestApp, socket: &mut Socket, user: i64) -> Vec<String> {
    test.booted.app.broadcasts.replace(
        &crate::channels::broadcasts::Stream::user_rooms(user),
        "ws13_remaining_barrier",
        "",
    );
    let mut frames = Vec::new();
    loop {
        let frame = next(socket).await;
        let html = frame["message"].as_str().unwrap();
        if html.contains("target=\"ws13_remaining_barrier\"") {
            break;
        }
        assert!(campfire_cable::turbo::session_bound(html).is_none());
        frames.push(html.to_owned());
    }
    frames
}
#[tokio::test]
async fn remaining_call_security_keeps_stage_actions_private_and_type_scoped() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let (room, host, listener, speaker) = stage(&test).await;
    test.db()
        .write(move |tx| {
            tx.conn()
                .execute_cached("UPDATE users SET role=1 WHERE id=?", [JZ])?;
            tx.conn().execute_cached(
                "UPDATE memberships SET hand_raised_at=? WHERE id=?",
                rusqlite::params![tx.now(), listener],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut outsider = test.sign_in(JZ).await;
    for (method, path) in [
        (
            Method::PATCH,
            format!("/rooms/{}/stage/roles/{listener}", room.id),
        ),
        (Method::POST, format!("/rooms/{}/stage/hand", room.id)),
        (Method::DELETE, format!("/rooms/{}/stage/hand", room.id)),
    ] {
        assert_eq!(
            outsider
                .write(Req::new(method, &path).form(&[("stage_role", "speaker")]))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    test.db()
        .write(|tx| {
            tx.conn()
                .execute_cached("UPDATE users SET role=0 WHERE id=?", [JZ])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut outsider = test.sign_in(JZ).await;
    for method in [Method::POST, Method::DELETE] {
        assert_eq!(
            outsider
                .write(Req::new(method, &format!("/rooms/{}/stage/hand", room.id)))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    let mut audience = test.sign_in(JASON).await;
    let denied = audience
        .write(
            Req::new(Method::DELETE, &format!("/rooms/{}/stage/hand", room.id))
                .form(&[("membership_id", &speaker.to_string())]),
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    assert_eq!(denied.text(), "Only hosts can lower another member's hand");
    for user in [JASON, KEVIN] {
        let mut browser = test.sign_in(user).await;
        for (method, action) in [
            (Method::POST, "mute"),
            (Method::DELETE, "mute"),
            (Method::POST, "disconnect"),
        ] {
            assert_eq!(
                browser
                    .write(Req::new(
                        method,
                        &format!("/rooms/{}/call_moderation/{host}/{action}", room.id)
                    ))
                    .await
                    .status,
                StatusCode::FORBIDDEN
            );
        }
    }
    let voice = test
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Voice, Some("Lounge"), DAVID, &[DAVID, JASON]))
        .await
        .unwrap();
    let mut admin = test.sign_in(DAVID).await;
    for kind in [RoomType::Voice, RoomType::Closed] {
        let other = if kind == RoomType::Voice {
            voice.clone()
        } else {
            test.db()
                .write(|tx| {
                    Room::create_for(
                        tx,
                        RoomType::Closed,
                        Some("Channel"),
                        DAVID,
                        &[DAVID, JASON],
                    )
                })
                .await
                .unwrap()
        };
        let member = test
            .db()
            .read(move |c| Ok(Membership::find_by_room_and_user(c, other.id, JASON)?.unwrap()))
            .await
            .unwrap();
        for (method, path) in [
            (
                Method::PATCH,
                format!("/rooms/{}/stage/roles/{}", member.room_id, member.id),
            ),
            (
                Method::POST,
                format!("/rooms/{}/stage/hand", member.room_id),
            ),
            (
                Method::DELETE,
                format!("/rooms/{}/stage/hand", member.room_id),
            ),
        ] {
            assert_eq!(
                admin
                    .write(Req::new(method, &path).form(&[("stage_role", "speaker")]))
                    .await
                    .status,
                StatusCode::NOT_FOUND
            );
        }
        if kind == RoomType::Closed {
            let invalid = test
                .db()
                .write(move |tx| {
                    Membership::find(tx.conn(), member.id)?.set_server_muted(tx, true, &domain())
                })
                .await
                .unwrap_err();
            let campfire_db::Error::RecordInvalid(errors) = invalid else {
                panic!("{invalid:?}")
            };
            assert!(
                errors
                    .full_messages()
                    .iter()
                    .any(|e| e.contains("only exists on stage and voice rooms"))
            );
        }
    }
    assert_eq!(
        test.db()
            .read(move |c| Ok(Membership::find(c, listener)?.stage_role))
            .await
            .unwrap(),
        Some(StageRole::Listener)
    );
    assert!(
        test.db()
            .read(move |c| Ok(Membership::find(c, host)?.server_muted_at.is_none()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn remaining_roles_allow_an_admin_member_to_repair_a_hostless_stage() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let (room, _, listener, _) = stage(&test).await;
    let id = room.id;
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE memberships SET stage_role='listener' WHERE room_id=?",
                [id],
            )?;
            tx.conn()
                .execute_cached("UPDATE users SET role=1 WHERE id=?", [KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut admin = test.sign_in(KEVIN).await;
    assert_eq!(
        admin
            .write(
                Req::new(
                    Method::PATCH,
                    &format!("/rooms/{id}/stage/roles/{listener}")
                )
                .form(&[("stage_role", "host")])
            )
            .await
            .status,
        StatusCode::FOUND
    );
    assert_eq!(
        test.db()
            .read(move |c| Ok(Membership::find(c, listener)?.stage_role))
            .await
            .unwrap(),
        Some(StageRole::Host)
    );
}
#[tokio::test]
async fn remaining_moderation_revokes_tokens_on_mute_and_unmute_and_checks_drift() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let (room, host, _, speaker) = stage(&test).await;
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE memberships SET stage_role='host' WHERE room_id=? AND user_id=?",
                rusqlite::params![room.id, JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let original = grant(&test, room.id, KEVIN, speaker).await;
    let mut admin = test.sign_in(DAVID).await;
    let mut member = test.sign_in(KEVIN).await;
    let path = format!("/rooms/{}/call_moderation/{speaker}/mute", room.id);
    let mut host_browser = test.sign_in(JASON).await;
    let mut issued = vec![original.id];
    for (method, muted) in [(Method::POST, true), (Method::DELETE, false)] {
        assert_eq!(
            host_browser.write(Req::new(method, &path)).await.status,
            StatusCode::FOUND
        );
        let previous = *issued.last().unwrap();
        let state=test.db().read(move |c|Ok((Membership::find(c,speaker)?.server_muted_at.is_some(),HuddleGrant::find_by_id(c,previous)?.unwrap().revoked(),c.query_row_cached("SELECT COUNT(*) FROM huddle_cleanups WHERE huddle_grant_id=? AND operation='remove_participant'",[previous],|r|r.get::<_,i64>(0))?))).await.unwrap();
        assert_eq!(state, (muted, true, 1));
        let response = member
            .write(Req::new(
                Method::POST,
                &format!("/rooms/{}/huddle", room.id),
            ))
            .await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        let body = response.json();
        issued.push(body["grant_id"].as_i64().unwrap());
        assert_ne!(*issued.last().unwrap(), previous);
        // Read claims using the same JWT verifier's decoded JSON serialization below.
        let payload = body["token"].as_str().unwrap().split('.').nth(1).unwrap();
        use base64::Engine;
        let payload: Value = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(payload)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(payload["video"]["canPublish"], !muted);
    }
    let fresh = *issued.last().unwrap();
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE memberships SET server_muted_at=? WHERE id=?",
                rusqlite::params![tx.now(), speaker],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let stale = test
        .db()
        .read(move |c| HuddleGrant::find_by_id(c, fresh)?.unwrap().authorized(c))
        .await
        .unwrap();
    assert!(!stale);
    let (status, _) = crate::controllers::internal_huddle_tests::request(
        &test,
        Method::GET,
        &format!("/internal/huddle/grants/{fresh}?record_seen=0"),
        Some("ws13-fixture-gateway-secret"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(status, 404);
    assert!(
        test.db()
            .read(move |c| Ok(HuddleGrant::find_by_id(c, fresh)?.unwrap().revoked()))
            .await
            .unwrap()
    );
    // Administrative rank allows moderating another administrator.
    test.db()
        .write(move |tx| {
            tx.conn()
                .execute_cached("UPDATE users SET role=1 WHERE id=?", [KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    for (method, action) in [
        (Method::POST, "mute"),
        (Method::DELETE, "mute"),
        (Method::POST, "disconnect"),
    ] {
        assert_eq!(
            admin
                .write(Req::new(
                    method,
                    &format!("/rooms/{}/call_moderation/{speaker}/{action}", room.id)
                ))
                .await
                .status,
            StatusCode::FOUND
        );
    }
    assert!(
        test.db()
            .read(move |c| Ok(Membership::find(c, host)?.server_muted_at.is_none()))
            .await
            .unwrap()
    );
}
#[derive(Debug)]
struct Clock(std::sync::atomic::AtomicI64);
impl campfire_kit::Clock for Clock {
    fn now(&self) -> jiff::Timestamp {
        jiff::Timestamp::from_second(self.0.load(std::sync::atomic::Ordering::SeqCst)).unwrap()
    }
}
#[tokio::test]
async fn remaining_hands_roles_and_mute_deliver_exact_personalized_frames() {
    use futures_util::SinkExt;
    use tokio_tungstenite::tungstenite::Message;
    let clock = std::sync::Arc::new(Clock(std::sync::atomic::AtomicI64::new(1772467200)));
    let Some(test) = TestApp::boot_with_huddle_and_clock(configured(), clock.clone()).await else {
        return;
    };
    let (room, host, listener, speaker) = stage(&test).await;
    // Make Kevin an audience member for the per-viewer roster assertions.
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE memberships SET stage_role='listener' WHERE id=?",
                [speaker],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let listener_socket = crate::channels::tests::support::bind_listener().await;
    let addr = listener_socket.local_addr().unwrap();
    let (stop, stopping) = tokio::sync::oneshot::channel();
    let router = test.booted.router.clone();
    let serving = tokio::spawn(async move {
        axum::serve(listener_socket, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap()
    });
    let mut host_socket = socket(&test, addr, DAVID).await;
    let mut target_socket = socket(&test, addr, JASON).await;
    let mut audience_socket = socket(&test, addr, KEVIN).await;
    let stream = crate::channels::broadcasts::Stream::room_messages(&room);
    let identifier=serde_json::json!({"channel":"RoomMessagesChannel","signed_stream_name":rails_compat::turbo::signed_stream_name(&test.booted.app.secrets,&stream.streamables())}).to_string();
    host_socket
        .send(Message::Text(
            serde_json::json!({"command":"subscribe","identifier":identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(next(&mut host_socket).await["type"], "confirm_subscription");
    let mut browser = test.sign_in(JASON).await;
    assert_eq!(
        browser
            .write(Req::new(
                Method::POST,
                &format!("/rooms/{}/stage/hand", room.id)
            ))
            .await
            .status,
        StatusCode::FOUND
    );
    for (client, user, manager) in [
        (&mut host_socket, DAVID, true),
        (&mut target_socket, JASON, false),
        (&mut audience_socket, KEVIN, false),
    ] {
        let frames = frames(&test, client, user).await;
        assert_eq!(frames.len(), 1, "{frames:?}");
        assert!(frames[0].contains("Hand raised"));
        assert_eq!(frames[0].contains("Invite to speak"), manager);
        assert_eq!(frames[0].contains("Make host"), manager);
    }
    let first = test
        .db()
        .read(move |c| Ok(Membership::find(c, listener)?.hand_raised_at.unwrap()))
        .await
        .unwrap();
    clock.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut kevin = test.sign_in(KEVIN).await;
    assert_eq!(
        kevin
            .write(Req::new(
                Method::POST,
                &format!("/rooms/{}/stage/hand", room.id)
            ))
            .await
            .status,
        StatusCode::FOUND
    );
    for (client, user) in [
        (&mut host_socket, DAVID),
        (&mut target_socket, JASON),
        (&mut audience_socket, KEVIN),
    ] {
        assert_eq!(frames(&test, client, user).await.len(), 1);
    }
    clock.0.fetch_add(5, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        browser
            .write(Req::new(
                Method::POST,
                &format!("/rooms/{}/stage/hand", room.id)
            ))
            .await
            .status,
        StatusCode::FOUND
    );
    for (client, user) in [
        (&mut host_socket, DAVID),
        (&mut target_socket, JASON),
        (&mut audience_socket, KEVIN),
    ] {
        assert!(frames(&test, client, user).await.is_empty());
    }
    let queue = test
        .db()
        .read(move |c| {
            Ok((
                Membership::find(c, listener)?.hand_raised_at.unwrap(),
                Membership::find(c, speaker)?.hand_raised_at.unwrap(),
            ))
        })
        .await
        .unwrap();
    assert_eq!(queue.0, first);
    assert!(queue.0 < queue.1);
    let original = grant(&test, room.id, JASON, listener).await;
    let host_grant = grant(&test, room.id, DAVID, host).await;
    // Drain issuance presence frames before measuring role effects.
    for (client, user) in [
        (&mut host_socket, DAVID),
        (&mut target_socket, JASON),
        (&mut audience_socket, KEVIN),
    ] {
        frames(&test, client, user).await;
    }
    let mut admin = test.sign_in(DAVID).await;
    assert_eq!(
        admin
            .write(
                Req::new(
                    Method::PATCH,
                    &format!("/rooms/{}/stage/roles/{listener}", room.id)
                )
                .form(&[("stage_role", "speaker")])
            )
            .await
            .status,
        StatusCode::FOUND
    );
    for (client, user, count, manager) in [
        (&mut host_socket, DAVID, 3, true),
        (&mut target_socket, JASON, 4, false),
        (&mut audience_socket, KEVIN, 2, false),
    ] {
        let frames = frames(&test, client, user).await;
        assert_eq!(frames.len(), count, "{frames:?}"); // Host also subscribes to the single room presence frame.
        let roster = frames
            .iter()
            .find(|h| h.contains("target=\"stage_roster_"))
            .unwrap();
        assert_eq!(roster.contains("Invite to speak"), manager);
        assert_eq!(roster.contains("Make host"), manager);
    }
    let state = test
        .db()
        .read(move |c| {
            Ok((
                Membership::find(c, listener)?.stage_role,
                Membership::find(c, listener)?.hand_raised_at,
                HuddleGrant::find_by_id(c, original.id)?.unwrap().revoked(),
                HuddleGrant::find_by_id(c, host_grant.id)?
                    .unwrap()
                    .revoked(),
                c.query_row_cached(
                    "SELECT COUNT(*) FROM huddle_cleanups WHERE huddle_grant_id=?",
                    [original.id],
                    |r| r.get::<_, i64>(0),
                )?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(state, (Some(StageRole::Speaker), None, true, false, 1));
    assert_eq!(
        admin
            .write(Req::new(
                Method::POST,
                &format!("/rooms/{}/call_moderation/{listener}/mute", room.id)
            ))
            .await
            .status,
        StatusCode::FOUND
    );
    for (client, user, count) in [
        (&mut host_socket, DAVID, 1),
        (&mut target_socket, JASON, 2),
        (&mut audience_socket, KEVIN, 1),
    ] {
        let frames = frames(&test, client, user).await;
        assert_eq!(frames.len(), count, "{frames:?}");
        let roster = frames
            .iter()
            .find(|h| h.contains("target=\"stage_roster_"))
            .unwrap();
        assert!(roster.contains("Muted"));
        if user == DAVID {
            assert!(roster.contains("Unmute"));
        }
        if user == JASON {
            let event = frames
                .iter()
                .find(|h| h.contains("action=\"append\""))
                .unwrap();
            assert!(event.contains("target=\"huddle_role_events\""));
            assert!(event.contains("data-huddle-rejoin-server-muted=\"true\""));
        }
    }
    for client in [&mut host_socket, &mut target_socket, &mut audience_socket] {
        client.close(None).await.unwrap();
    }
    stop.send(()).unwrap();
    serving.await.unwrap();
}
