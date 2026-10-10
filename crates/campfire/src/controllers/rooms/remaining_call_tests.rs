//! Remaining Rails controller assertions, through HTTP and the real Cable hub.
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
