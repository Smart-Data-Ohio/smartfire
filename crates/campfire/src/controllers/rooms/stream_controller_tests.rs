//! Remaining assertions from test/controllers/rooms/stage/streams_controller_test.rb.
//! Domain implementations and signatures belong to WS13b and remain unchanged.
use super::call_channel_tests::configured;
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig, stream::Stream};
use campfire_db::{CachedStatements, Membership, Room, RoomType};
const JZ: i64 = 773523953;

async fn fixture(test: &TestApp) -> Room {
    test.db()
        .write(|tx| {
            Room::create_for(
                tx,
                RoomType::Stage,
                Some("Town Hall"),
                DAVID,
                &[DAVID, JASON, KEVIN],
            )
        })
        .await
        .unwrap()
}
async fn member(test: &TestApp, room_id: i64, user: i64) -> Membership {
    test.db()
        .read(move |conn| Ok(Membership::find_by_room_and_user(conn, room_id, user)?.unwrap()))
        .await
        .unwrap()
}
async fn role(test: &TestApp, room_id: i64, user: i64, role: &str) {
    let role = role.to_string();
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE memberships SET stage_role=? WHERE room_id=? AND user_id=?",
                rusqlite::params![role, room_id, user],
            )?;
            Ok(())
        })
        .await
        .unwrap();
}
async fn grant(test: &TestApp, room_id: i64, user: i64, seen: Option<i64>, revoked: bool) -> i64 {
    let membership = member(test, room_id, user).await.id;
    test.db()
        .write(move |tx| {
            let session = campfire_db::Session::start(tx, user, None, None)?;
            let grant = HuddleGrant::issue(
                tx,
                session.id,
                membership,
                room_id,
                &HuddleConfig {
                    api_secret: Some("ws13-fixture-api-secret".into()),
                    admin_configured: false,
                },
            )?;
            // Exactly Rails' issue_in_call_grant! helper: no presence job is enqueued.
            let at = seen.map(|age| tx.now().ago(jiff::SignedDuration::from_secs(age)));
            tx.conn().execute_cached(
                "UPDATE huddle_grants SET last_seen_at=?,revoked_at=? WHERE id=?",
                rusqlite::params![at, revoked.then(|| tx.now()), grant.id],
            )?;
            Ok(grant.id)
        })
        .await
        .unwrap()
}
async fn live_count(test: &TestApp) -> i64 {
    test.db()
        .read(|c| {
            Ok(c.query_row_cached(
                "SELECT COUNT(*) FROM streams WHERE ended_at IS NULL",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap()
}
async fn live(test: &TestApp, room_id: i64) -> Option<Stream> {
    test.db()
        .read(move |c| Stream::live_for_room(c, room_id))
        .await
        .unwrap()
}

#[tokio::test]
async fn stream_controller_start_denials_preserve_global_stream_count_and_exact_bodies() {
    for (name, actor, seen, revoked, muted, quality, expected, body) in [
        (
            "listener",
            KEVIN,
            Some(0),
            false,
            false,
            Some("1080p15"),
            StatusCode::FORBIDDEN,
            "Only hosts and speakers can go live",
        ),
        (
            "muted host",
            DAVID,
            Some(0),
            false,
            true,
            Some("1080p15"),
            StatusCode::FORBIDDEN,
            "Muted members cannot go live",
        ),
        (
            "revoked host",
            DAVID,
            Some(0),
            true,
            false,
            Some("1080p15"),
            StatusCode::FORBIDDEN,
            "Join the stage before going live",
        ),
        (
            "quiet host",
            DAVID,
            None,
            false,
            false,
            Some("1080p15"),
            StatusCode::FORBIDDEN,
            "Join the stage before going live",
        ),
        (
            "stale host",
            DAVID,
            Some(25),
            false,
            false,
            Some("1080p15"),
            StatusCode::FORBIDDEN,
            "Join the stage before going live",
        ),
        (
            "missing quality",
            DAVID,
            Some(0),
            false,
            false,
            None,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Unknown stream quality",
        ),
    ] {
        let Some(test) = TestApp::boot_with_huddle(configured()).await else {
            return;
        };
        let room = fixture(&test).await;
        grant(&test, room.id, actor, seen, revoked).await;
        if muted {
            let id = room.id;
            test.db()
                .write(move |tx| {
                    tx.conn().execute_cached(
                        "UPDATE memberships SET server_muted_at=? WHERE room_id=? AND user_id=?",
                        rusqlite::params![tx.now(), id, actor],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        let before = live_count(&test).await;
        let mut browser = test.sign_in(actor).await;
        let path = format!("/rooms/{}/stage/stream", room.id);
        let request = Req::new(Method::POST, &path);
        let response = browser
            .write(if let Some(q) = quality {
                request.form(&[("quality", q)])
            } else {
                request
            })
            .await;
        assert_eq!(response.status, expected, "{name}: {}", response.text());
        assert_eq!(response.text(), body, "{name}");
        assert_eq!(live_count(&test).await, before, "{name}");
        assert!(live(&test, room.id).await.is_none(), "{name}");
    }
}

#[tokio::test]
async fn stream_controller_stop_permissions_and_unknown_ids_preserve_stream_state() {
    for (name, actor, speaker, admin, with_stream, unknown, turbo, status) in [
        (
            "speaker presenter",
            JASON,
            true,
            false,
            true,
            false,
            false,
            StatusCode::FOUND,
        ),
        (
            "administrator listener",
            KEVIN,
            false,
            true,
            true,
            false,
            false,
            StatusCode::FOUND,
        ),
        (
            "listener",
            KEVIN,
            false,
            false,
            true,
            false,
            false,
            StatusCode::FORBIDDEN,
        ),
        (
            "unknown id",
            DAVID,
            false,
            false,
            true,
            true,
            false,
            StatusCode::FOUND,
        ),
        (
            "no live listener",
            KEVIN,
            false,
            false,
            false,
            false,
            false,
            StatusCode::FORBIDDEN,
        ),
        (
            "legacy host stop",
            DAVID,
            false,
            false,
            true,
            false,
            true,
            StatusCode::FOUND,
        ),
    ] {
        let Some(test) = TestApp::boot_with_huddle(configured()).await else {
            return;
        };
        let room = fixture(&test).await;
        if speaker {
            role(&test, room.id, JASON, "speaker").await;
        }
        if admin {
            test.db()
                .write(|tx| {
                    tx.conn()
                        .execute_cached("UPDATE users SET role=1 WHERE id=?", [KEVIN])?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        let presenter = if speaker { JASON } else { DAVID };
        let membership = member(&test, room.id, presenter).await.id;
        let room_id = room.id;
        let stream = if with_stream {
            Some(
                test.db()
                    .write(move |tx| {
                        Stream::create(tx, room_id, membership, presenter, "1080p15", None)
                    })
                    .await
                    .unwrap(),
            )
        } else {
            None
        };
        let before = live_count(&test).await;
        let mut browser = test.sign_in(actor).await;
        let path = format!("/rooms/{room_id}/stage/stream");
        let mut request = Req::new(Method::DELETE, &path);
        if unknown {
            request = request.form(&[("stream_id", "-1")]);
        }
        if turbo {
            request = request.header("accept", "text/vnd.turbo-stream.html");
        }
        let response = browser.write(request).await;
        assert_eq!(response.status, status, "{name}: {}", response.text());
        if status == StatusCode::FOUND {
            assert_eq!(
                response.location(),
                Some(format!("http://campfire.test/rooms/{room_id}").as_str()),
                "{name}"
            );
        }
        let ended = with_stream && status != StatusCode::FORBIDDEN && !unknown;
        assert_eq!(live_count(&test).await, before - i64::from(ended), "{name}");
        assert_eq!(
            live(&test, room_id).await.is_some(),
            with_stream && !ended,
            "{name}"
        );
        if let Some(s) = stream {
            assert_eq!(
                test.db()
                    .read(move |c| Ok(Stream::find_by_id(c, s.id)?.unwrap().live()))
                    .await
                    .unwrap(),
                !ended,
                "{name}"
            );
        }
    }
}

#[tokio::test]
async fn stream_controller_outsiders_administrators_and_non_stage_namespaces_are_not_found() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let room = fixture(&test).await;
    for administrator in [false, true] {
        test.db()
            .write(move |tx| {
                tx.conn().execute_cached(
                    "UPDATE users SET role=? WHERE id=?",
                    rusqlite::params![i64::from(administrator), JZ],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = test.sign_in(JZ).await;
        let path = format!("/rooms/{}/stage/stream", room.id);
        for verb in [Method::POST, Method::DELETE] {
            let response = browser
                .write(Req::new(verb, &path).form(&[("quality", "1080p15")]))
                .await;
            assert_eq!(response.status, StatusCode::NOT_FOUND);
            assert!(live(&test, room.id).await.is_none());
        }
    }
    let membership = member(&test, room.id, DAVID).await.id;
    let room_id = room.id;
    let stream = test
        .db()
        .write(move |tx| Stream::create(tx, room_id, membership, DAVID, "1080p15", None))
        .await
        .unwrap();
    let mut outsider = test.sign_in(JZ).await;
    let response = outsider
        .write(Req::new(
            Method::DELETE,
            &format!("/rooms/{room_id}/stage/stream"),
        ))
        .await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(live(&test, room_id).await.unwrap().id, stream.id);
    let mut browser = test.sign_in(DAVID).await;
    for kind in [
        RoomType::Voice,
        RoomType::Open,
        RoomType::Closed,
        RoomType::Direct,
    ] {
        let other = test
            .db()
            .write(move |tx| Room::create_for(tx, kind, Some("Lounge"), DAVID, &[DAVID, JASON]))
            .await
            .unwrap();
        for verb in [Method::POST, Method::DELETE] {
            let response = browser
                .write(
                    Req::new(verb, &format!("/rooms/{}/stage/stream", other.id))
                        .form(&[("quality", "1080p15")]),
                )
                .await;
            assert_eq!(response.status, StatusCode::NOT_FOUND, "{kind:?}");
            assert!(live(&test, other.id).await.is_none());
        }
    }
}

#[tokio::test]
async fn stream_controller_successor_and_members_edit_preserve_the_rails_lifecycle() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let room = fixture(&test).await;
    let room_id = room.id;
    role(&test, room_id, JASON, "speaker").await;
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE users SET role=CASE WHEN id=? THEN 0 ELSE 1 END WHERE id IN (?,?)",
                rusqlite::params![JASON, JASON, KEVIN],
            )?;
            let host = Membership::find_by_room_and_user(tx.conn(), room_id, DAVID)?.unwrap();
            host.destroy(tx)?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        member(&test, room_id, KEVIN)
            .await
            .stage_role
            .unwrap()
            .name(),
        "host"
    );
    let grant_id = grant(&test, room_id, JASON, Some(0), false).await;
    let before = live_count(&test).await;
    let mut speaker = test.sign_in(JASON).await;
    let response = speaker
        .write(
            Req::new(Method::POST, &format!("/rooms/{room_id}/stage/stream"))
                .form(&[("quality", "1080p15")]),
        )
        .await;
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(
        response.location(),
        Some(format!("http://campfire.test/rooms/{room_id}").as_str())
    );
    assert_eq!(live_count(&test).await, before + 1);
    assert_eq!(live(&test, room_id).await.unwrap().user_id, JASON);
    let started_id = live(&test, room_id).await.unwrap().id;
    let mut admin = test.sign_in(KEVIN).await;
    let response = admin
        .write(
            Req::new(Method::PATCH, &format!("/rooms/stages/{room_id}")).form(&[
                ("room[name]", "Town Hall"),
                ("user_ids[]", &KEVIN.to_string()),
            ]),
        )
        .await;
    assert_eq!(response.status, StatusCode::FOUND, "{}", response.text());
    assert!(live(&test, room_id).await.is_none());
    assert_eq!(
        response.location(),
        Some(format!("http://campfire.test/rooms/{room_id}").as_str())
    );
    assert!(
        !test
            .db()
            .read(move |c| Ok(Stream::find_by_id(c, started_id)?.unwrap().live()))
            .await
            .unwrap()
    );
    assert!(
        test.db()
            .read(move |c| Ok(Membership::find_by_room_and_user(c, room_id, JASON)?.is_none()))
            .await
            .unwrap()
    );
    assert!(
        test.db()
            .read(move |c| Ok(HuddleGrant::find_by_id(c, grant_id)?.unwrap().revoked()))
            .await
            .unwrap()
    );
}
