//! Ports the card integration file through RoomsController, not detached view DTOs.
use super::{card_tests::Fresh, test_support::request};
use campfire_db::{Message, NewMessage, Timestamp};
use rusqlite::{params, types::Value as SqlValue};
use serde_json::{Value, json};

async fn card_fixture(fresh: &Fresh, case: &Value) {
    let case = case.clone();
    fresh.app.db.write(move|tx| {
        tx.conn().execute_batch("DELETE FROM github_pull_request_references; DELETE FROM github_pull_request_threads; DELETE FROM github_pull_requests;")?;
        if let Some(attributes)=case["attributes"].as_object() {
            let mut columns=vec!["created_at","updated_at"];
            let mut values=vec![SqlValue::Text(tx.now().to_db()),SqlValue::Text(tx.now().to_db())];
            for (key,value) in attributes {
                columns.push(key);
                values.push(match value {
                    Value::Null=>SqlValue::Null,
                    Value::Bool(value)=>SqlValue::Integer(i64::from(*value)),
                    Value::Number(value)=>SqlValue::Integer(value.as_i64().unwrap()),
                    Value::String(value)=>SqlValue::Text(if key.ends_with("_at") {Timestamp::from_jiff(value.replace(" UTC","Z").parse().unwrap()).to_db()}else{value.clone()}),
                    _=>SqlValue::Text(value.to_string()),
                });
            }
            tx.conn().execute(&format!("INSERT INTO github_pull_requests ({}) VALUES ({})",columns.join(","),vec!["?";columns.len()].join(",")),rusqlite::params_from_iter(values))?;
            tx.conn().execute("INSERT INTO github_pull_request_references (github_pull_request_id,message_id,created_at,updated_at) VALUES (816,818,?,?)",params![tx.now(),tx.now()])?;
            if case["mapped"]==true {
                crate::integrations::github::threads::PullRequestThread::create(tx,816,815,817)?;
            }
        }
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn github_room_cards_real_pages_match_pinned_rails_card_containers() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/github_room_cards.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let fresh = Fresh::new(&json!({})).await;
        card_fixture(&fresh, case).await;
        let (status, _, body) = request(&fresh, "GET", "/rooms/815", Value::Null, json!({})).await;
        assert_eq!(status, 200, "{}", case["name"]);
        assert!(
            body.contains(case["cards"].as_str().unwrap()),
            "actual room caller differs for {}",
            case["name"]
        );
        match case["name"].as_str().unwrap() {
            "public" => {
                assert_eq!(
                    body.matches("class=\"github-pr-card__discuss-form\"")
                        .count(),
                    1
                );
                assert_eq!(
                    body.matches("<button class=\"github-pr-card__discuss\"")
                        .count(),
                    1
                );
                assert!(!body.contains("<a class=\"github-pr-card__discuss\""));
                for text in [
                    "Rails/Rails",
                    "#12",
                    "Fix login",
                    "alice",
                    "Open",
                    "main ← shiny",
                    "Approved",
                    "Checks passing",
                ] {
                    assert!(body.contains(text), "missing {text}");
                }
                assert!(body.contains("rel=\"noopener noreferrer\""));
                assert!(body.contains("href=\"https://github.com/Rails/Rails/pull/12\""));
            }
            "discuss_link" => {
                assert_eq!(
                    body.matches("<a class=\"github-pr-card__discuss\"").count(),
                    1
                );
                assert!(!body.contains("class=\"github-pr-card__discuss-form\""));
            }
            "private" | "unknown" => {
                assert!(!body.contains("Secret title"));
                assert!(!body.contains("<article class=\"github-pr-card "));
                assert!(body.contains("loading=\"lazy\" class=\"github-pr-card-frame\""));
            }
            "no_cards" => assert!(!body.contains("<article class=\"github-pr-card ")),
            _ => {}
        }
        assert!(
            fresh.server.received().is_empty(),
            "room HTML must never use a viewer's credential or call GitHub"
        );
    }
}

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
        let (status, _, body) = request(&fresh, "GET", "/rooms/815", Value::Null, json!({})).await;
        assert_eq!(status, 200);
        assert_eq!(body.matches("<article class=\"github-pr-card ").count(), 4);
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
                    .uri("/rooms/815")
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
        assert!(String::from_utf8_lossy(&body).contains("Room refresh"));
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
    let (status, headers, body) =
        request(&fresh, "GET", "/rooms/815", Value::Null, json!({})).await;
    assert_eq!(status, 302);
    assert_eq!(headers["location"], "http://example.org/");
    assert!(!body.contains("Secret title") && !body.contains("github-pr-card"));
    assert!(fresh.server.received().is_empty());
}
