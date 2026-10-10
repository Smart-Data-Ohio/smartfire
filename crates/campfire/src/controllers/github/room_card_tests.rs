//! Ports the card integration file through RoomsController, not detached view DTOs.
use super::{card_tests::Fresh, test_support::request};
use campfire_db::{Message, NewMessage, Timestamp};
use rusqlite::params;
use serde_json::{Value, json};

async fn fetch_count(fresh: &Fresh) -> i64 {
    fresh.app.db.read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'",[],|r|r.get(0))?)).await.unwrap()
}
#[tokio::test]
async fn github_room_refresh_claims_dedupe_many_messages_and_different_viewers() {
    use campfire_kit::Crypto;
    use tower::ServiceExt;
    for age in [Some(60), Some(660), None] {
        let fresh = Fresh::new(&json!({"private":false})).await;
        let token=fresh.app.db.write(move|tx| {
            for index in 0..3 {
                Message::create_markdown(tx,NewMessage{room_id:815,creator_id:811,client_message_id:Some(format!("room-dedupe-{index}")),..Default::default()},"https://github.com/rails/rails/pull/12")?;
            }
            let fetched=age.map(|seconds|Timestamp::from_jiff(tx.now().jiff().checked_sub(std::time::Duration::from_secs(seconds)).unwrap()));
            tx.conn().execute("UPDATE github_pull_requests SET title='Room refresh',fetched_at=?,fetch_requested_at=NULL,private=0 WHERE id=816",[fetched])?;
            tx.conn().execute("DELETE FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'",[])?;
            tx.conn().execute("INSERT INTO memberships (room_id,user_id,created_at,updated_at) VALUES (815,812,?,?)",params![tx.now(),tx.now()])?;
            Ok(campfire_db::Session::start_with(tx,812,campfire_db::NewSession{two_factor_verified:true,..Default::default()})?.token)
        }).await.unwrap();
        fresh.app.fragment_cache.clear();
        assert_eq!(fetch_count(&fresh).await, 0);
        let (status, _, _body) = request(&fresh, "GET", "/api/v1/rooms/815/messages", Value::Null, json!({})).await;
        assert_eq!(status, 200);

        let expected = i64::from(age != Some(60));
        assert_eq!(fetch_count(&fresh).await, expected, "age={age:?}");
        let signed = campfire_kit::RailsCrypto::new(fresh.app.secrets.clone()).sign_cookie(
            "session_token",
            &token,
            None,
        );
        let response = fresh
            .router
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/rooms/815/messages")
                    .header("Host", "example.org")
                    .header(
                        "Cookie",
                        format!("session_token={}", campfire_kit::cookies::escape(&signed)),
                    )
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let messages: Value = serde_json::from_slice(&body).unwrap();
        assert!(messages["messages"].as_array().unwrap().iter().any(|message| {
            message["cards"].as_array().is_some_and(|cards| {
                cards.iter().any(|card| card["kind"] == "github" && card["data"]["pullRequestId"] == 816)
            })
        }));
        assert_eq!(
            fetch_count(&fresh).await,
            expected,
            "second viewer must share the same refresh window"
        );
        fresh
            .app
            .db
            .read(move |conn| {
                let claimed: Option<Timestamp> = conn.query_row(
                    "SELECT fetch_requested_at FROM github_pull_requests WHERE id=816",
                    [],
                    |r| r.get(0),
                )?;
                assert_eq!(claimed.is_some(), expected == 1);
                Ok(())
            })
            .await
            .unwrap();
        assert!(fresh.server.received().is_empty());
    }
}

#[tokio::test]
async fn github_room_cards_security_redirects_nonmembers_without_card_data() {
    let fresh = Fresh::new(&json!({"member":false,"private":false})).await;
    let (status, _, body) =
        request(&fresh, "GET", "/api/v1/rooms/815/messages", Value::Null, json!({})).await;
    assert_eq!(status, 404);
    assert!(!body.contains("Secret title") && !body.contains("github-pr-card"));
    assert!(fresh.server.received().is_empty());
}
