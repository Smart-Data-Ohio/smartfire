use super::call_channel_tests::configured;
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{Room, RoomType};

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/huddle_review_fixes.json"
    ))
    .unwrap()
}

fn unicode_oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/huddle_unicode_order.json"
    ))
    .unwrap()
}

async fn unicode_call() -> (TestApp, i64) {
    use campfire_db::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig};
    use campfire_db::{Session, User, UserChanges};
    let test = TestApp::boot_with_huddle(configured())
        .await
        .expect("seed")
        .without_job_runner()
        .await;
    let room = test
        .db()
        .write(|tx| {
            let oracle = unicode_oracle();
            for (index, id) in [JASON, KEVIN].into_iter().enumerate() {
                User::find(tx.conn(), id)?.update(
                    tx,
                    UserChanges {
                        name: Some(oracle["input_names"][index].as_str().unwrap().into()),
                        ..Default::default()
                    },
                )?;
            }
            let room = Room::create_for(
                tx,
                RoomType::Closed,
                Some("Sigma review"),
                DAVID,
                &[DAVID, JASON, KEVIN],
            )?;
            for id in [JASON, KEVIN] {
                let session = Session::start(tx, id, None, None)?;
                let member =
                    campfire_db::Membership::find_by_room_and_user(tx.conn(), room.id, id)?
                        .unwrap();
                let mut grant = HuddleGrant::issue(
                    tx,
                    session.id,
                    member.id,
                    room.id,
                    &HuddleConfig {
                        api_secret: Some("ws13-fixture-api-secret".into()),
                        admin_configured: false,
                    },
                )?;
                grant.record_seen(tx)?;
            }
            Ok(room.id)
        })
        .await
        .unwrap();
    (test, room)
}

#[tokio::test]
async fn review_unicode_sidebar_avatars_match_rails_and_the_per_room_endpoint() {
    let (test, room) = unicode_call().await;
    let mut browser = test.david();
    let per_room = browser
        .get(&format!("/rooms/{room}/huddle/participants"))
        .await;
    assert_eq!(per_room.status, StatusCode::OK);
    let names: Vec<_> = per_room
        .json()
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["name"].clone())
        .collect();
    assert_eq!(
        serde_json::json!(names),
        unicode_oracle()["participant_names"]
    );
}


#[tokio::test]
async fn review_unicode_presence_matches_rails_and_the_per_room_endpoint() {
    let (test, room) = unicode_call().await;
    let mut browser = test.david();
    let per_room = browser
        .get(&format!("/rooms/{room}/huddle/participants"))
        .await;
    assert_eq!(per_room.status, StatusCode::OK);
    let names: Vec<_> = per_room
        .json()
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["name"].clone())
        .collect();
    assert_eq!(
        serde_json::json!(names),
        unicode_oracle()["participant_names"]
    );
    let response = browser.get("/users/huddle_presence").await;
    assert_eq!(response.status, StatusCode::OK);
    let data = response.json();
    let call = data
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["room_id"] == room)
        .expect("live room");
    let participants: Vec<_> = call["participants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["name"].clone())
        .collect();
    assert_eq!(
        serde_json::json!(participants),
        unicode_oracle()["presence_names"],
        "Rails downcase keeps aggregate presence in the same order as the per-room poll"
    );
}

#[tokio::test]
async fn review_audit_failure_preserves_committed_call_room_deletion() {
    let test = TestApp::boot()
        .await
        .expect("seed")
        .without_job_runner()
        .await;
    let room = test
        .db()
        .write(|tx| {
            Room::create_for(
                tx,
                RoomType::Stage,
                Some("Review stage"),
                DAVID,
                &[DAVID, JASON, KEVIN],
            )
        })
        .await
        .unwrap();
    test.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws13_reject_room_audit BEFORE INSERT ON audit_logs WHEN NEW.action='room.destroy' BEGIN SELECT RAISE(ABORT,'WS13 rejected audit'); END")?;
        Ok(())
    }).await.unwrap();
    let response = test
        .david()
        .write(
            Req::new(Method::DELETE, &format!("/rooms/{}", room.id))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(
        response.status.as_u16(),
        oracle()["audit_failure"]["status"].as_u64().unwrap() as u16
    );
    let state = test.db().read(move |conn| Ok(serde_json::json!({"deleted": Room::find(conn,room.id)?.deleted(), "memberships": Room::find(conn,room.id)?.user_ids(conn)?.len()}))).await.unwrap();
    assert_eq!(state["deleted"], oracle()["audit_failure"]["deleted"]);
    assert_eq!(
        state["memberships"],
        oracle()["audit_failure"]["memberships"]
    );
}
