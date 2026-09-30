//! Request assertions from test/controllers/rooms/stage_view_test.rb.
use super::call_channel_tests::configured;
use crate::controllers::presenters::test_support::{DAVID, KEVIN, TestApp};
use axum::http::StatusCode;
use campfire_db::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig, stream::Stream};
use campfire_db::{CachedStatements, Membership, Room, RoomType};
const JZ: i64 = 773523953;
async fn fixture(test: &TestApp, creator: i64, other: i64) -> Room {
    test.db()
        .write(move |tx| {
            Room::create_for(
                tx,
                RoomType::Stage,
                Some("Town Hall"),
                creator,
                &[creator, other],
            )
        })
        .await
        .unwrap()
}
async fn membership(test: &TestApp, room_id: i64, user: i64) -> Membership {
    test.db()
        .read(move |c| Ok(Membership::find_by_room_and_user(c, room_id, user)?.unwrap()))
        .await
        .unwrap()
}
fn row(html: String, membership: i64) -> String {
    let start = html
        .find(&format!("<li id=\"stage_row_membership_{membership}\""))
        .expect("stage row");
    let end = html[start..].find("</li>").unwrap() + start;
    html[start..end].to_string()
}
#[tokio::test]
async fn stage_page_join_hints_role_forms_and_go_live_follow_the_viewer_role() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let room = fixture(&test, DAVID, KEVIN).await;
    let room_id = room.id;
    for (actor, role, publish, manage) in [
        (KEVIN, "listener", false, false),
        (DAVID, "host", true, true),
        (KEVIN, "speaker", true, false),
    ] {
        let role = role.to_string();
        test.db()
            .write(move |tx| {
                tx.conn().execute_cached(
                    "UPDATE memberships SET stage_role=? WHERE room_id=? AND user_id=?",
                    rusqlite::params![role, room_id, actor],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = test.sign_in(actor).await;
        let response = browser.get(&format!("/rooms/{room_id}")).await;
        assert_eq!(response.status, StatusCode::OK);
        let html = response.text();
        assert!(html.contains(&format!("data-huddle-can-publish-param=\"{publish}\"")));
        assert!(html.contains("id=\"huddle_role_events\""));
        if publish {
            assert!(html.contains("Stream quality"));
        }
        assert_eq!(html.contains("Go live"), publish);
        assert_eq!(html.contains("Invite to speak"), manage);
        assert_eq!(html.contains("Make host"), manage);
        if !publish {
            assert!(html.contains("You are in the audience"));
            for label in ["Move to audience", "Move to speakers"] {
                assert!(!html.contains(label));
            }
        }
    }
    let voice = test
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Voice, Some("Lounge"), DAVID, &[DAVID, KEVIN]))
        .await
        .unwrap();
    let mut browser = test.sign_in(KEVIN).await;
    let response = browser.get(&format!("/rooms/{}", voice.id)).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("Join voice"));
    assert!(!response.text().contains("data-huddle-can-publish-param"));
}
#[tokio::test]
async fn stage_page_administrator_rows_have_exact_moderation_controls() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let room = fixture(&test, JZ, DAVID).await;
    let room_id = room.id;
    let admin = membership(&test, room_id, DAVID).await.id;
    let host = membership(&test, room_id, JZ).await.id;
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE memberships SET stage_role='speaker' WHERE id=?",
                [admin],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut browser = test.sign_in(JZ).await;
    let response = browser.get(&format!("/rooms/{room_id}")).await;
    assert_eq!(response.status, StatusCode::OK);
    let target = row(response.text(), admin);
    assert!(target.contains("/stage/roles/"));
    assert!(!target.contains("/call_moderation/"));
    let mut browser = test.sign_in(DAVID).await;
    let response = browser.get(&format!("/rooms/{room_id}")).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(row(response.text(), host).contains("/call_moderation/"));
    test.db()
        .write(move |tx| {
            tx.conn().execute_cached(
                "UPDATE memberships SET server_muted_at=? WHERE id=?",
                rusqlite::params![tx.now(), admin],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let response = browser.get(&format!("/rooms/{room_id}")).await;
    assert_eq!(response.status, StatusCode::OK);
    let target = row(response.text(), admin);
    assert_eq!(target.matches("/call_moderation/").count(), 1);
    assert!(target.contains(">Unmute</button>"));
}
#[tokio::test]
async fn stage_page_live_identity_stop_permissions_and_stream_id_follow_the_presenter() {
    let Some(test) = TestApp::boot_with_huddle(configured()).await else {
        return;
    };
    let room = fixture(&test, DAVID, KEVIN).await;
    let room_id = room.id;
    let host = membership(&test, room_id, DAVID).await.id;
    let mut listener = test.sign_in(KEVIN).await;
    let response = listener.get(&format!("/rooms/{room_id}")).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(!response.text().contains("Live: David"));
    assert!(!response.text().contains("Go live"));
    let identity = test
        .db()
        .write(move |tx| {
            let session = campfire_db::Session::start(tx, DAVID, None, None)?;
            let grant = HuddleGrant::issue(
                tx,
                session.id,
                host,
                room_id,
                &HuddleConfig {
                    api_secret: Some("ws13-fixture-api-secret".into()),
                    admin_configured: false,
                },
            )?;
            Stream::create(tx, room_id, host, DAVID, "1080p15", None)?;
            Ok(grant.identity)
        })
        .await
        .unwrap();
    let response = listener.get(&format!("/rooms/{room_id}")).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("Live: David"));
    assert!(
        response
            .text()
            .contains(&format!("data-presenter-id=\"{identity}\""))
    );
    assert!(!response.text().contains("Stop stream"));
    let mut browser = test.sign_in(DAVID).await;
    let response = browser.get(&format!("/rooms/{room_id}")).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(!response.text().contains("Go live"));
    assert!(response.text().contains("Live: David"));
    assert!(response.text().contains("Stop stream"));
    let speaker = membership(&test, room_id, KEVIN).await.id;
    test.db()
        .write(move |tx| {
            Stream::live_for_room(tx.conn(), room_id)?
                .unwrap()
                .end(tx, None)?;
            tx.conn().execute_cached(
                "UPDATE memberships SET stage_role='speaker' WHERE id=?",
                [speaker],
            )?;
            Stream::create(tx, room_id, speaker, KEVIN, "1080p15", None)?;
            Ok(())
        })
        .await
        .unwrap();
    for actor in [KEVIN, DAVID] {
        let mut browser = test.sign_in(actor).await;
        let response = browser.get(&format!("/rooms/{room_id}")).await;
        assert_eq!(response.status, StatusCode::OK);
        assert!(response.text().contains("Stop stream"));
    }
}
