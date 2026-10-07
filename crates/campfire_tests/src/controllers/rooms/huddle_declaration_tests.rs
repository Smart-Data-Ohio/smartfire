//! Remaining HTTP/Cable assertions from rooms/huddles_controller_test.rb.
//! These tests use the existing WS13b domain interfaces without porting its tests.
use super::call_channel_tests::configured;
use crate::controllers::presenters::test_support::{
    ALL_TALK, DAVID, DIRECT_DAVID_JASON, JASON, KEVIN, Req, TestApp,
};
use axum::http::{Method, StatusCode};
use campfire_db::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig};
use campfire_db::{CachedStatements, Membership, Room, RoomType, Session};
use serde_json::{Value, json};
const JZ: i64 = 773523953;
fn domain() -> HuddleConfig {
    HuddleConfig {
        api_secret: Some("ws13-fixture-api-secret".into()),
        admin_configured: true,
    }
}
async fn active(test: &TestApp, room_id: i64, user: i64, session: Option<i64>) -> HuddleGrant {
    test.db()
        .write(move |tx| {
            let session = match session {
                Some(id) => id,
                None => Session::start(tx, user, None, None)?.id,
            };
            let member = Membership::find_by_room_and_user(tx.conn(), room_id, user)?.unwrap();
            let grant = HuddleGrant::issue(tx, session, member.id, room_id, &domain())?;
            // Same fixture setup as Rails' issue-in-call helper, without first-seen jobs.
            tx.conn().execute_cached(
                "UPDATE huddle_grants SET last_seen_at=? WHERE id=?",
                rusqlite::params![tx.now(), grant.id],
            )?;
            Ok(HuddleGrant::find_by_id(tx.conn(), grant.id)?.unwrap())
        })
        .await
        .unwrap()
}
async fn current_session(test: &TestApp, user: i64) -> i64 {
    test.db()
        .read(move |c| {
            Ok(c.query_row_cached(
                "SELECT id FROM sessions WHERE user_id=? ORDER BY id DESC LIMIT 1",
                [user],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap()
}
fn denial(
    reply: &crate::controllers::presenters::test_support::Reply,
    status: StatusCode,
    message: &str,
) {
    assert_eq!(reply.status, status, "{}", reply.text());
    assert_eq!(reply.json(), json!({"error":message}));
    assert_eq!(reply.header("cache-control"), Some("no-store"));
}
#[tokio::test]
async fn huddle_controller_reuses_the_actual_opaque_grant_across_post_requests() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let mut browser = test.sign_in(DAVID).await;
    let first = browser
        .write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/huddle")))
        .await;
    let second = browser
        .write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/huddle")))
        .await;
    for reply in [&first, &second] {
        assert_eq!(reply.status, StatusCode::OK);
    }
    let first = first.json();
    let second = second.json();
    assert_eq!(first["grant_id"], second["grant_id"]);
    assert_eq!(first["identity"], second["identity"]);
    let id = first["grant_id"].as_i64().unwrap();
    let grant = test
        .db()
        .read(move |c| Ok(HuddleGrant::find_by_id(c, id)?.unwrap()))
        .await
        .unwrap();
    let shape = regex::Regex::new(r"\Acampfire-participant-[0-9a-f]{64}\z").unwrap();
    assert!(shape.is_match(&grant.identity));
    assert_ne!(
        grant.room_name.strip_prefix("campfire-room-"),
        grant.identity.strip_prefix("campfire-participant-")
    );
    for reply in [&first, &second] {
        let coordinates = rails_compat::jwt::livekit::verify(
            reply["token"].as_str().unwrap(),
            "ws13-fixture-api-key",
            "ws13-fixture-api-secret",
            test.booted.app.db.env().now().as_second(),
        )
        .unwrap();
        assert_eq!(coordinates.room_name, grant.room_name);
        assert_eq!(coordinates.identity, grant.identity);
    }
}
#[tokio::test]
async fn huddle_controller_posts_capture_invitation_jobs_and_exact_recipients() {
    for kind in [RoomType::Closed, RoomType::Direct] {
        for group in [false, true] {
            if kind == RoomType::Closed && group {
                continue;
            }
            let Some(test) = TestApp::boot_with_huddle(configured()).await else {
                return;
            };
            let room = test
                .db()
                .write(move |tx| {
                    Room::create_for(
                        tx,
                        kind,
                        if kind == RoomType::Direct {
                            None
                        } else {
                            Some("Channel")
                        },
                        DAVID,
                        if group {
                            &[DAVID, JASON, KEVIN]
                        } else {
                            &[DAVID, JASON]
                        },
                    )
                })
                .await
                .unwrap();
            test.db().write(|tx| {
                // Capture actual committed enqueues even when a worker has already consumed its row.
                tx.conn().execute_batch("CREATE TABLE ws13_enqueued_jobs(job_class TEXT, arguments TEXT); CREATE TRIGGER ws13_capture_jobs AFTER INSERT ON background_jobs BEGIN INSERT INTO ws13_enqueued_jobs VALUES(NEW.job_class,NEW.arguments); END;")?;
                Ok(())
            }).await.unwrap();
            let before = test
                .db()
                .read(|c| {
                    Ok(
                        c.query_row_cached("SELECT COUNT(*) FROM activity_items", [], |r| {
                            r.get::<_, i64>(0)
                        })?,
                    )
                })
                .await
                .unwrap();
            let mut browser = test.sign_in(DAVID).await;
            let response = browser
                .write(Req::new(
                    Method::POST,
                    &format!("/rooms/{}/huddle", room.id),
                ))
                .await;
            assert_eq!(response.status, StatusCode::OK, "{}", response.text());
            let grant_id = response.json()["grant_id"].as_i64().unwrap();
            let room_id = room.id;
            let (grant,items,jobs,after)=test.db().read(move |c| {
                let items=c.prepare_cached("SELECT id,user_id,source_id,event_type FROM activity_items WHERE source_type='HuddleGrant' AND source_id=? ORDER BY user_id")?.query_map([grant_id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?,r.get::<_,String>(3)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
                let jobs=c.prepare_cached("SELECT arguments FROM ws13_enqueued_jobs WHERE job_class='Huddle::PushInvitationJob'")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
                let grant=HuddleGrant::find_by_id(c,grant_id)?.unwrap();
                assert_eq!(grant.room_id,room_id); assert_eq!(grant.user_id,DAVID);
                Ok((grant,items,jobs,c.query_row_cached("SELECT COUNT(*) FROM activity_items",[],|r|r.get::<_,i64>(0))?))
            }).await.unwrap();
            assert_eq!(grant.id, grant_id);
            let expected = if kind == RoomType::Closed {
                vec![]
            } else if group {
                vec![JASON, KEVIN]
            } else {
                vec![JASON]
            };
            assert_eq!(items.iter().map(|i| i.1).collect::<Vec<_>>(), expected);
            assert_eq!(after - before, expected.len() as i64);
            assert_eq!(jobs.len(), expected.len());
            for item in items {
                assert_ne!(item.1, DAVID);
                assert_eq!(item.2, grant_id);
                assert_eq!(item.3, "huddle_started");
                assert!(jobs.iter().any(|arguments| {
                    serde_json::from_str::<Value>(arguments).unwrap()["activity_item_id"] == item.0
                }));
            }
        }
    }
}
#[tokio::test]
async fn huddle_controller_nonmembers_and_removed_callers_fail_closed_and_leave_participants() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let group = test
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Direct, None, DAVID, &[DAVID, JASON, KEVIN]))
        .await
        .unwrap();
    let grant = active(&test, group.id, DAVID, None).await;
    let mut outsider = test.sign_in(JZ).await;
    for room in [ALL_TALK, DIRECT_DAVID_JASON, group.id] {
        let response = outsider
            .write(Req::new(Method::POST, &format!("/rooms/{room}/huddle")))
            .await;
        denial(
            &response,
            StatusCode::NOT_FOUND,
            "Room not found or inaccessible",
        );
    }
    let voice = test
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Voice, Some("Lounge"), DAVID, &[DAVID, JASON]))
        .await
        .unwrap();
    let voice_grant = active(&test, voice.id, DAVID, None).await;
    let mut browser = test.sign_in(JASON).await;
    let response = browser
        .get(&format!("/rooms/{}/huddle/participants", voice.id))
        .await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response
            .json()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["id"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        vec![DAVID]
    );
    for (room_id, grant_id) in [(group.id, grant.id), (voice.id, voice_grant.id)] {
        test.db()
            .write(move |tx| {
                Membership::find_by_room_and_user(tx.conn(), room_id, DAVID)?
                    .unwrap()
                    .destroy(tx)
            })
            .await
            .unwrap();
        let revoked = test
            .db()
            .read(move |c| Ok(HuddleGrant::find_by_id(c, grant_id)?.unwrap().revoked()))
            .await
            .unwrap();
        assert!(revoked);
    }
    let response = browser
        .get(&format!("/rooms/{}/huddle/participants", voice.id))
        .await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json(), json!([]));
    let mut david = test.sign_in(DAVID).await;
    let response = david
        .write(Req::new(
            Method::POST,
            &format!("/rooms/{}/huddle", group.id),
        ))
        .await;
    denial(
        &response,
        StatusCode::NOT_FOUND,
        "Room not found or inaccessible",
    );
    let mut kevin = test.sign_in(KEVIN).await;
    let response = kevin
        .write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/huddle")))
        .await;
    denial(
        &response,
        StatusCode::NOT_FOUND,
        "Room not found or inaccessible",
    );
    for room in [voice.id, DIRECT_DAVID_JASON] {
        let response = outsider
            .get(&format!("/rooms/{room}/huddle/participants"))
            .await;
        denial(
            &response,
            StatusCode::NOT_FOUND,
            "Room not found or inaccessible",
        );
    }
    let response = outsider.write(Req::new(Method::DELETE, "/session")).await;
    assert_eq!(response.status, StatusCode::FOUND);
    let response = outsider
        .get(&format!("/rooms/{}/huddle/participants", voice.id))
        .await;
    denial(
        &response,
        StatusCode::UNAUTHORIZED,
        "Authentication required",
    );
}
#[tokio::test]
async fn huddle_controller_signout_revokes_the_current_grant_and_persists_cleanup() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let mut browser = test.sign_in(DAVID).await;
    let session = current_session(&test, DAVID).await;
    let grant = active(&test, ALL_TALK, DAVID, Some(session)).await;
    let response = browser
        .write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/huddle")))
        .await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json()["grant_id"], grant.id);
    let response = browser.write(Req::new(Method::DELETE, "/session")).await;
    assert_eq!(response.status, StatusCode::FOUND);
    let response = browser.get(&format!("/rooms/{ALL_TALK}/huddle")).await;
    denial(
        &response,
        StatusCode::UNAUTHORIZED,
        "Authentication required",
    );
    let id = grant.id;
    let (revoked,cleanups)=test.db().read(move |c| Ok((HuddleGrant::find_by_id(c,id)?.unwrap().revoked(),c.query_row_cached("SELECT COUNT(*) FROM huddle_cleanups WHERE operation='remove_participant' AND huddle_grant_id=?",[id],|r|r.get::<_,i64>(0))?))).await.unwrap();
    assert!(revoked);
    assert_eq!(cleanups, 1);
}
#[tokio::test]
async fn huddle_controller_aliased_public_url_denies_before_issuing_any_grant() {
    let mut config = configured();
    config.public_url = Some("wss://internal.example.test:7880/client/path".into());
    let Some(test) = TestApp::boot_with_huddle(config).await else {
        return;
    };
    let mut browser = test.sign_in(DAVID).await;
    let response = browser
        .write(Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/huddle")))
        .await;
    denial(
        &response,
        StatusCode::SERVICE_UNAVAILABLE,
        "Huddles are not configured",
    );
    let count = test
        .db()
        .read(|c| {
            Ok(
                c.query_row_cached("SELECT COUNT(*) FROM huddle_grants", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(count, 0);
}
#[tokio::test]
async fn huddle_controller_leave_keeps_the_other_device_and_sends_one_room_refresh() {
    use super::call_channel_broadcast_tests::{next, socket};
    use crate::channels::broadcasts::Stream as Channel;
    use futures_util::SinkExt;
    use tokio_tungstenite::tungstenite::Message;
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let mut browser = test.sign_in(DAVID).await;
    let session = current_session(&test, DAVID).await;
    let grant = active(&test, ALL_TALK, DAVID, Some(session)).await;
    let other = active(&test, ALL_TALK, DAVID, None).await;
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
    let mut client = socket(&test, addr, DAVID).await;
    let room = test.db().read(|c| Room::find(c, ALL_TALK)).await.unwrap();
    let room_stream = Channel::room_messages(&room);
    let identifier=json!({"channel":"RoomMessagesChannel","signed_stream_name":rails_compat::turbo::signed_stream_name(&test.booted.app.secrets,&room_stream.streamables())}).to_string();
    client
        .send(Message::Text(
            json!({"command":"subscribe","identifier":identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(next(&mut client).await["type"], "confirm_subscription");
    let response = browser
        .write(Req::new(
            Method::POST,
            &format!("/rooms/{ALL_TALK}/huddle/leave"),
        ))
        .await;
    assert_eq!(response.status, StatusCode::NO_CONTENT);
    assert!(response.body.is_empty());
    assert_eq!(response.header("cache-control"), Some("no-store"));
    test.booted
        .app
        .broadcasts
        .replace(&Channel::user_rooms(DAVID), "ws13_leave_barrier", "");
    let mut room_frames = Vec::new();
    loop {
        let frame = next(&mut client).await;
        let html = frame["message"].as_str().unwrap();
        if html.contains("target=\"ws13_leave_barrier\"") {
            break;
        }
        if frame["identifier"] == identifier {
            room_frames.push(html.to_owned());
        }
    }
    assert_eq!(room_frames.len(), 1, "{room_frames:?}");
    assert!(room_frames[0].contains(&format!(
        "target=\"header_voice_participants_rooms_closed_{ALL_TALK}\""
    )));
    let now = test.booted.app.db.env().now();
    let (grant, other, participants) = test
        .db()
        .read(move |c| {
            Ok((
                HuddleGrant::find_by_id(c, grant.id)?.unwrap(),
                HuddleGrant::find_by_id(c, other.id)?.unwrap(),
                HuddleGrant::participants_for(c, ALL_TALK, now)?,
            ))
        })
        .await
        .unwrap();
    assert!(grant.last_seen_at.is_none());
    assert!(!grant.revoked());
    assert!(other.last_seen_at.is_some());
    assert_eq!(
        participants.iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![DAVID]
    );
    client.close(None).await.unwrap();
    let _ = stop.send(());
    serving.await.unwrap();
}
