use super::card_tests::Fresh;
use crate::{
    controllers::presenters::test_support::masked_session_token,
    integrations::github::subscriptions::RepositorySubscription,
};
use axum::{
    body::Body,
    http::{HeaderMap, Request},
};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn csrf(fresh: &Fresh) -> (String, String) {
    let response = fresh
        .router
        .clone()
        .oneshot(
            Request::get("/app/")
                .header("Host", "example.org")
                .header("Cookie", &fresh.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let raw = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .find_map(|v| {
            v.to_str()
                .ok()?
                .split(';')
                .next()?
                .strip_prefix("_campfire_session=")
        })
        .unwrap();
    let token = masked_session_token(&fresh.app.secrets, raw).unwrap();
    (format!("{}; _campfire_session={raw}", fresh.cookie), token)
}
async fn request(
    fresh: &Fresh,
    method: &str,
    id: Option<i64>,
    body: Value,
) -> (u16, HeaderMap, String) {
    let (cookie, token) = csrf(fresh).await;
    let path = format!(
        "/rooms/815/github_subscriptions{}",
        if method == "POST" {
            String::new()
        } else {
            id.map(|id| format!("/{id}")).unwrap_or_default()
        }
    );
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("Host", "example.org")
        .header("Cookie", cookie)
        .header("X-CSRF-Token", token)
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = fresh.router.clone().oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, headers, String::from_utf8(body.to_vec()).unwrap())
}
#[tokio::test]
async fn github_subscription_http_security_rejects_nonmembers_plain_members_direct_deleted_and_cross_room()
 {
    for (input, status) in [
        (json!({"member":false,"linked":true}), 404),
        (json!({"role":0,"room_creator":812,"linked":true}), 403),
        (json!({"kind":"Rooms::Direct","linked":true}), 404),
        (json!({"deleted":true,"linked":true}), 404),
    ] {
        let fresh = Fresh::new(&input).await;
        let sub = fresh
            .app
            .db
            .write(|tx| {
                RepositorySubscription::create(
                    tx,
                    825,
                    "rails",
                    "rails",
                    Value::Null,
                    Some(811),
                    true,
                )
            })
            .await;
        let id = sub.ok().map(|s| s.id).unwrap_or(9999);
        for method in ["POST", "PATCH", "DELETE"] {
            let (actual,headers,_)=request(&fresh,method,if method=="POST"{None}else{Some(id)},json!({"github_repository_subscription":{"full_name":"rails/rails","events":["merged"],"skip_access_check":"1"}})).await;
            assert_eq!(actual, status, "{input} {method}");
            if status == 403 || input["kind"] == "Rooms::Direct" {
                assert_eq!(headers.get("content-type").unwrap(), "text/html");
            }
        }
        assert!(fresh.server.received().is_empty());
    }
    let fresh = Fresh::new(&json!({"linked":true})).await;
    let sub = fresh
        .app
        .db
        .write(|tx| {
            RepositorySubscription::create(tx, 825, "rails", "rails", Value::Null, Some(811), true)
        })
        .await
        .unwrap();
    for method in ["PATCH", "DELETE"] {
        assert_eq!(
            request(
                &fresh,
                method,
                Some(sub.id),
                json!({"github_repository_subscription":{"events":["merged"]}})
            )
            .await
            .0,
            404
        );
    }
    assert!(fresh.server.received().is_empty());
}

#[tokio::test]
async fn github_subscription_http_status_flash_token_events_and_membership_match_rails() {
    use campfire_kit::Crypto;
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/github_subscription_http.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let fresh = Fresh::new(case).await;
        let input = case.clone();
        let ids = fresh
            .app
            .db
            .write(move |tx| {
                let mut ids = Vec::new();
                for name in input["preset"].as_array().into_iter().flatten() {
                    let (owner, repo) = name.as_str().unwrap().split_once('/').unwrap();
                    let s = RepositorySubscription::create(
                        tx,
                        input["preset_room"].as_i64().unwrap_or(815),
                        owner,
                        repo,
                        Value::Null,
                        Some(811),
                        true,
                    )?;
                    ids.push(s.id);
                }
                Ok(ids)
            })
            .await
            .unwrap();
        let method = case["method"].as_str().unwrap_or("POST");
        let (status, headers, body) = request(
            &fresh,
            method,
            ids.first().copied(),
            case["request_body"].clone(),
        )
        .await;
        assert_eq!(status, case["status"], "{}", case["name"]);
        assert_eq!(
            headers.get("location").and_then(|h| h.to_str().ok()),
            case["location"].as_str(),
            "{}",
            case["name"]
        );
        assert_eq!(
            headers.get("content-type").unwrap(),
            case["content_type"].as_str().unwrap(),
            "{}",
            case["name"]
        );
        if let Some(expected) = case["body"].as_str() {
            assert_eq!(body, expected, "{}", case["name"]);
        }
        let flashes = headers
            .get_all("set-cookie")
            .iter()
            .find_map(|v| {
                v.to_str()
                    .ok()?
                    .split(';')
                    .next()?
                    .strip_prefix("_campfire_session=")
            })
            .and_then(|raw| {
                campfire_kit::RailsCrypto::new(fresh.app.secrets.clone()).decrypt_cookie(
                    "_campfire_session",
                    &percent_encoding::percent_decode_str(raw).decode_utf8_lossy(),
                    "2026-01-01T12:00:00Z".parse().unwrap(),
                )
            })
            .and_then(|value| value.get("flash").and_then(|v| v.get("flashes")).cloned())
            .unwrap_or(json!({}));
        assert_eq!(flashes, case["flash"], "{}", case["name"]);
        let (rows,bot_member,disconnected)=fresh.app.db.read(|conn| {
   let rows=conn.prepare("SELECT room_id,owner,repo,events,created_by_id,reader_verified FROM github_repository_subscriptions ORDER BY owner,repo")?.query_map([],|r|Ok(json!({"room_id":r.get::<_,i64>(0)?,"owner":r.get::<_,String>(1)?,"repo":r.get::<_,String>(2)?,"events":serde_json::from_str::<Value>(&r.get::<_,String>(3)?).unwrap(),"created_by_id":r.get::<_,Option<i64>>(4)?,"reader_verified":r.get::<_,bool>(5)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
   let bot=conn.query_row("SELECT EXISTS(SELECT 1 FROM memberships WHERE room_id=815 AND user_id IN (SELECT id FROM users WHERE role=2 AND status=0 AND name='GitHub'))",[],|r|r.get::<_,bool>(0))?;
   let disconnected=crate::integrations::github::accounts::Account::for_user(conn,811)?.and_then(|a|a.disconnected_reason);Ok((rows,bot,disconnected))
  }).await.unwrap();
        assert_eq!(
            serde_json::to_value(rows).unwrap(),
            case["rows"],
            "{}",
            case["name"]
        );
        assert_eq!(bot_member, case["bot_member"], "{}", case["name"]);
        assert_eq!(
            serde_json::to_value(disconnected).unwrap(),
            case["disconnected"],
            "{}",
            case["name"]
        );
        let requests = fresh.server.received();
        assert_eq!(
            requests.len(),
            case["requests"].as_array().unwrap().len(),
            "{}",
            case["name"]
        );
        for (actual, expected) in requests.iter().zip(case["requests"].as_array().unwrap()) {
            assert_eq!(
                actual.target,
                format!(
                    "/repos/{}/{}",
                    expected[0].as_str().unwrap(),
                    expected[1].as_str().unwrap()
                )
            );
            assert!(
                actual
                    .headers
                    .iter()
                    .any(|(key, value)| key.eq_ignore_ascii_case("authorization")
                        && value == &format!("Bearer {}", expected[2].as_str().unwrap()))
            );
        }
    }
}
