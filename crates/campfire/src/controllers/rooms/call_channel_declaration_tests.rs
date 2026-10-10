//! Remaining request assertions in stages_controller_test.rb and the shared Voice paths.
use super::call_channel_broadcast_tests::{Socket, next, socket};
use super::call_channel_tests::configured;
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Membership, Room, RoomType, StageRole};
const JZ: i64 = 773523953;
async fn fixture(test: &TestApp, kind: RoomType, users: &[i64]) -> Room {
    let users = users.to_vec();
    test.db()
        .write(move |tx| Room::create_for(tx, kind, Some("Town Hall"), DAVID, &users))
        .await
        .unwrap()
}
async fn drain(test: &TestApp, client: &mut Socket, user: i64) -> Vec<String> {
    test.booted.app.broadcasts.channel(&format!("user_{user}_reads"), &serde_json::json!({"barrier": true}));
    let mut frames = Vec::new();
    loop {
        let frame = next(client).await;
        if frame["message"]["barrier"] == true { break; }
        frames.push(frame["message"].to_string());
    }
    frames
}

#[tokio::test]
async fn call_channel_members_and_outsiders_cannot_edit_read_messages_or_receive_denial_frames() {
    for (kind, namespace) in [(RoomType::Stage, "stages"), (RoomType::Voice, "voices")] {
        let Some(test) = TestApp::boot_with_huddle(configured()).await else {
            return;
        };
        let room = fixture(&test, kind, &[DAVID, JZ]).await;
        let room_id = room.id;
        let listener = crate::channels::tests::support::bind_listener().await;
        let addr = listener.local_addr().unwrap();
        let (stop, stopping) = tokio::sync::oneshot::channel();
        let router = test.booted.router.clone();
        let serving = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = stopping.await;
                })
                .await
                .unwrap()
        });
        let mut member_frames = socket(&test, addr, JZ).await;
        let mut browser = test.sign_in(JZ).await;
        let response = browser
            .write(
                Req::new(Method::PUT, &format!("/rooms/{namespace}/{room_id}"))
                    .form(&[("room[name]", "New Name")]),
            )
            .await;
        assert_eq!(response.status, StatusCode::FORBIDDEN);
        assert!(drain(&test, &mut member_frames, JZ).await.is_empty());
        assert_eq!(
            test.db()
                .read(move |c| Room::find(c, room_id))
                .await
                .unwrap()
                .name
                .as_deref(),
            Some("Town Hall")
        );
        let secret = fixture(&test, kind, &[DAVID]).await;
        let secret_id = secret.id;
        test.db()
            .write(move |tx| {
                campfire_db::Message::create(
                    tx,
                    campfire_db::NewMessage {
                        room_id: secret_id,
                        creator_id: DAVID,
                        body: Some("Secret call chat".into()),
                        ..Default::default()
                    },
                )?;
                Ok(())
            })
            .await
            .unwrap();
        for path in [
            format!("/rooms/{secret_id}"),
            format!("/rooms/{namespace}/{secret_id}"),
            format!("/rooms/{namespace}/{secret_id}/edit"),
        ] {
            let response = browser.get(&path).await;
            assert_eq!(
                response.status,
                StatusCode::FOUND,
                "{path}: {}",
                response.text()
            );
            assert_eq!(response.location(), Some("http://campfire.test/"));
            assert!(!response.text().contains("Secret call chat"));
        }
        let response = browser.get(&format!("/rooms/{secret_id}/messages")).await;
        assert_eq!(response.status, StatusCode::NOT_FOUND);
        assert!(!response.text().contains("Secret call chat"));
        member_frames.close(None).await.unwrap();
        let _ = stop.send(());
        serving.await.unwrap();
    }
}
#[tokio::test]
async fn call_channel_unknown_icon_creation_renders_errors_without_creating_any_room() {
    for namespace in ["stages", "voices"] {
        let Some(test) = TestApp::boot_with_huddle(configured()).await else {
            return;
        };
        let before = test
            .db()
            .read(|c| {
                Ok(c.query_row_cached("SELECT COUNT(*) FROM rooms", [], |r| r.get::<_, i64>(0))?)
            })
            .await
            .unwrap();
        let mut browser = test.sign_in(DAVID).await;
        let response = browser
            .write(
                Req::new(Method::POST, &format!("/rooms/{namespace}")).form(&[
                    ("room[name]", "Iconic"),
                    ("room[icon_name]", ":notanicon:"),
                    ("user_ids[]", &DAVID.to_string()),
                ]),
            )
            .await;
        assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(response.text().contains("Icon name is not a known icon"));
        assert_eq!(
            test.db()
                .read(|c| Ok(
                    c.query_row_cached("SELECT COUNT(*) FROM rooms", [], |r| r.get::<_, i64>(0))?
                ))
                .await
                .unwrap(),
            before
        );
    }
}
#[tokio::test]
async fn stage_channel_revisions_keep_hosts_and_assign_only_new_members_as_listeners() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let room = fixture(&test, RoomType::Stage, &[DAVID, JASON, JZ]).await;
    let room_id = room.id;
    let mut browser = test.sign_in(DAVID).await;
    for add in [false, true] {
        let mut fields = vec![
            ("room[name]", "New Name".to_owned()),
            ("user_ids[]", DAVID.to_string()),
            ("user_ids[]", JASON.to_string()),
        ];
        if add {
            fields.push(("user_ids[]", KEVIN.to_string()));
        }
        let params: Vec<_> = fields.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let response = browser
            .write(Req::new(Method::PUT, &format!("/rooms/stages/{room_id}")).form(&params))
            .await;
        assert_eq!(response.status, StatusCode::FOUND);
        assert_eq!(
            response.location(),
            Some(format!("http://campfire.test/rooms/{room_id}").as_str())
        );
        let (room, members) = test
            .db()
            .read(move |c| Ok((Room::find(c, room_id)?, Membership::for_room(c, room_id)?)))
            .await
            .unwrap();
        assert_eq!(room.name.as_deref(), Some("New Name"));
        assert!(!members.iter().any(|m| m.user_id == JZ));
        assert_eq!(
            members
                .iter()
                .find(|m| m.user_id == DAVID)
                .unwrap()
                .stage_role,
            Some(StageRole::Host)
        );
        if add {
            assert_eq!(
                members
                    .iter()
                    .find(|m| m.user_id == KEVIN)
                    .unwrap()
                    .stage_role,
                Some(StageRole::Listener)
            );
        }
    }
}
#[tokio::test]
async fn call_channel_self_removal_and_stage_empty_revision_keep_the_correct_room_state() {
    for kind in [RoomType::Stage, RoomType::Voice] {
        let Some(test) = TestApp::boot_with_huddle(configured()).await else {
            return;
        };
        let room = fixture(&test, kind, &[DAVID, JASON]).await;
        let room_id = room.id;
        let namespace = if kind == RoomType::Stage {
            "stages"
        } else {
            "voices"
        };
        if kind == RoomType::Stage {
            test.db()
                .write(move |tx| {
                    tx.conn().execute_cached(
                        "UPDATE memberships SET stage_role='host' WHERE room_id=? AND user_id=?",
                        rusqlite::params![room_id, JASON],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        let before = test
            .db()
            .read(|c| Ok(Room::for_user(c, DAVID)?.len()))
            .await
            .unwrap();
        let mut browser = test.sign_in(DAVID).await;
        let response = browser
            .write(
                Req::new(Method::PUT, &format!("/rooms/{namespace}/{room_id}")).form(&[
                    ("room[name]", "Town Hall"),
                    ("user_ids[]", &JASON.to_string()),
                ]),
            )
            .await;
        assert_eq!(response.status, StatusCode::FOUND);
        assert_eq!(
            response.location(),
            Some(format!("http://campfire.test/rooms/{room_id}").as_str())
        );
        assert_eq!(
            test.db()
                .read(|c| Ok(Room::for_user(c, DAVID)?.len()))
                .await
                .unwrap()
                + 1,
            before
        );
        let response = browser.get(&format!("/rooms/{room_id}")).await;
        assert_eq!(response.status, StatusCode::FOUND);
        assert_eq!(response.location(), Some("http://campfire.test/"));
    }
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let room = fixture(&test, RoomType::Stage, &[DAVID, JASON]).await;
    let room_id = room.id;
    let mut browser = test.sign_in(DAVID).await;
    // A missing list is the same empty revision as Rails' `user_ids: []` form encoding.
    let response = browser
        .write(
            Req::new(Method::PUT, &format!("/rooms/stages/{room_id}"))
                .form(&[("room[name]", "Town Hall")]),
        )
        .await;
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(
        response.location(),
        Some(format!("http://campfire.test/rooms/{room_id}").as_str())
    );
    let (members, notes) = test
        .db()
        .read(move |c| {
            Ok((
                Room::find(c, room_id)?.user_ids(c)?,
                c.query_row_cached(
                    "SELECT COUNT(*) FROM messages WHERE room_id=? AND system_note=1",
                    [room_id],
                    |r| r.get::<_, i64>(0),
                )?,
            ))
        })
        .await
        .unwrap();
    assert!(members.is_empty());
    assert_eq!(notes, 0);
}
#[tokio::test]
async fn stage_channel_page_and_edit_render_with_no_hosts() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let room = fixture(&test, RoomType::Stage, &[DAVID, JASON, KEVIN]).await;
    let room_id = room.id;
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE memberships SET stage_role='listener' WHERE room_id=?",
                [room_id],
            )?;
            tx.conn()
                .execute_cached("UPDATE users SET role=1 WHERE id=?", [KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    let hosts = test
        .db()
        .read(move |c| {
            Ok(Membership::for_room(c, room_id)?
                .into_iter()
                .filter(|m| m.stage_role == Some(StageRole::Host))
                .count())
        })
        .await
        .unwrap();
    assert_eq!(hosts, 0);
    let mut browser = test.sign_in(KEVIN).await;
    let response = browser.get(&format!("/rooms/{room_id}")).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("Hosts · 0"));
    let response = browser.get(&format!("/rooms/stages/{room_id}/edit")).await;
    assert_eq!(response.status, StatusCode::OK);
}
