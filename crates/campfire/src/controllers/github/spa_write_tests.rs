//! `/api/v1` pull-request writes, over the same fixture and fake GitHub client as the classic
//! write tests. The SPA is enabled so the JSON routes are mounted; nothing here calls the network.

use super::{
    card_tests::Fresh,
    test_support::{request_with_accept, sudo},
};
use crate::integrations::test_support::Route;
use serde_json::{Value, json};

const ACTIONS: &str = "/api/v1/rooms/815/github/pull_requests/816/actions";
const COMMENTS: &str = "/api/v1/rooms/815/github/pull_requests/816/comments";
const REVIEWS: &str = "/api/v1/rooms/815/github/pull_requests/816/reviews";
const REVIEWERS: &str = "/api/v1/rooms/815/github/pull_requests/816/review_requests";

fn spa(extra: Value) -> Value {
    let mut input = json!({"spa_enabled": true, "mapping": true});
    if let (Some(object), Some(more)) = (input.as_object_mut(), extra.as_object()) {
        for (key, value) in more {
            object.insert(key.clone(), value.clone());
        }
    }
    input
}

async fn call(fresh: &Fresh, method: &str, path: &str, body: Value) -> (u16, Value) {
    let (status, _, text) =
        request_with_accept(fresh, method, path, body, sudo(), "application/json").await;
    assert!(
        !text.contains("fixture-viewer-token"),
        "a token leaked into the response: {text}"
    );
    let parsed = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).unwrap_or(Value::String(text))
    };
    (status, parsed)
}

fn route(method: &str, path: &str, status: u16) -> Route {
    Route::new(method, "api.github.com", path, status)
        .body(json!({"id": 12, "message": "no"}).to_string())
}

#[tokio::test]
async fn actions_follow_the_classic_account_gate_including_a_closed_pull_request() {
    let linked = Fresh::with_routes(&spa(json!({"linked": true})), vec![]).await;
    let (status, body) = call(&linked, "GET", ACTIONS, json!({})).await;
    assert_eq!(status, 200);
    assert_eq!(
        body,
        json!({
            "login": "oracle",
            "account": "connected",
            "canComment": true,
            "canReview": true,
            "canRequestReviewers": true,
            "status": "open",
        })
    );

    let closed = Fresh::with_routes(&spa(json!({"linked": true})), vec![]).await;
    closed
        .app
        .db
        .write(|tx| {
            tx.conn().execute(
                "UPDATE github_pull_requests SET state='closed' WHERE id=816",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let (status, body) = call(&closed, "GET", ACTIONS, json!({})).await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], "closed");
    assert_eq!(body["canComment"], true);
    assert_eq!(body["canReview"], true);

    let unlinked = Fresh::with_routes(&spa(json!({})), vec![]).await;
    let (status, body) = call(&unlinked, "GET", ACTIONS, json!({})).await;
    assert_eq!(status, 200);
    assert_eq!(body["account"], "none");
    assert_eq!(body["login"], Value::Null);
    assert_eq!(body["canComment"], false);
    assert_eq!(body["canReview"], false);
    assert_eq!(body["canRequestReviewers"], false);

    let rejected = Fresh::with_routes(
        &spa(json!({"linked": true, "initially_disconnected": true})),
        vec![],
    )
    .await;
    let (status, body) = call(&rejected, "GET", ACTIONS, json!({})).await;
    assert_eq!(status, 200);
    assert_eq!(body["account"], "rejected");
    assert_eq!(body["login"], "oracle");
    assert_eq!(body["canComment"], false);

    for input in [
        json!({"member": false, "linked": true}),
        json!({"mapping": false, "linked": true}),
        json!({"deleted": true, "linked": true}),
    ] {
        let fresh = Fresh::with_routes(&spa(input.clone()), vec![]).await;
        let (status, _) = call(&fresh, "GET", ACTIONS, json!({})).await;
        assert_eq!(status, 404, "{input}");
        assert!(fresh.server.received().is_empty(), "{input}");
    }
}

#[tokio::test]
async fn comments_reviews_and_review_requests_post_as_the_viewer_and_keep_no_local_message() {
    let cases = [
        (
            COMMENTS,
            json!({"body": "  Looks good.  "}),
            "/repos/rails/rails/issues/12/comments",
            json!({"body": "Looks good."}),
            "Comment posted on GitHub as @oracle.",
        ),
        (
            REVIEWS,
            json!({"event": "approve", "body": "Ship it"}),
            "/repos/rails/rails/pulls/12/reviews",
            json!({"event": "APPROVE", "body": "Ship it"}),
            "Approved on GitHub as @oracle.",
        ),
        (
            REVIEWS,
            json!({"event": "request_changes", "body": "Rename it"}),
            "/repos/rails/rails/pulls/12/reviews",
            json!({"event": "REQUEST_CHANGES", "body": "Rename it"}),
            "Requested changes on GitHub as @oracle.",
        ),
        (
            REVIEWS,
            json!({"event": "comment", "body": "One question"}),
            "/repos/rails/rails/pulls/12/reviews",
            json!({"event": "COMMENT", "body": "One question"}),
            "Left a review comment on GitHub as @oracle.",
        ),
        (
            REVIEWERS,
            json!({"reviewers": "Alice, @Bob"}),
            "/repos/rails/rails/pulls/12/requested_reviewers",
            json!({"reviewers": ["alice", "bob"]}),
            "Requested review from @alice, @bob on GitHub as @oracle.",
        ),
    ];
    for (path, request_body, api_path, github_body, notice) in cases {
        let fresh = Fresh::with_routes(
            &spa(json!({"linked": true})),
            vec![route("POST", api_path, 201)],
        )
        .await;
        let before = message_count(&fresh).await;
        let (status, body) = call(&fresh, "POST", path, request_body).await;
        assert_eq!(status, 200, "{path} {body}");
        assert_eq!(body["notice"], notice);
        let requests = fresh.server.received();
        assert_eq!(requests.len(), 1, "{path}");
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].target, api_path);
        assert_eq!(
            serde_json::from_slice::<Value>(&requests[0].body).unwrap(),
            github_body
        );
        assert_eq!(
            requests[0].header("authorization"),
            Some("Bearer fixture-viewer-token")
        );
        assert_eq!(message_count(&fresh).await, before);
    }
}

#[tokio::test]
async fn writes_map_blank_input_refusal_and_a_rejected_token_without_calling_ahead() {
    let blank = Fresh::with_routes(&spa(json!({"linked": true})), vec![]).await;
    let (status, body) = call(&blank, "POST", COMMENTS, json!({"body": "  "})).await;
    assert_eq!(status, 422);
    assert_eq!(body["error"]["message"], "Write a comment first.");
    assert_eq!(body["error"]["fields"]["body"][0], "Write a comment first.");
    assert!(blank.server.received().is_empty());

    let note = Fresh::with_routes(&spa(json!({"linked": true})), vec![]).await;
    let (status, body) = call(
        &note,
        "POST",
        REVIEWS,
        json!({"event": "request_changes", "body": ""}),
    )
    .await;
    assert_eq!(status, 422);
    assert_eq!(
        body["error"]["message"],
        "Add a note describing the requested changes."
    );
    assert!(note.server.received().is_empty());

    let review_comment = Fresh::with_routes(&spa(json!({"linked": true})), vec![]).await;
    let (status, body) = call(
        &review_comment,
        "POST",
        REVIEWS,
        json!({"event": "comment"}),
    )
    .await;
    assert_eq!(status, 422);
    assert_eq!(
        body["error"]["message"],
        "Add a note for the review comment."
    );

    let names = Fresh::with_routes(&spa(json!({"linked": true})), vec![]).await;
    let (status, body) = call(
        &names,
        "POST",
        REVIEWERS,
        json!({"reviewers": "no spaces!!"}),
    )
    .await;
    assert_eq!(status, 422);
    assert_eq!(
        body["error"]["message"],
        "Enter GitHub usernames separated by commas."
    );
    assert!(names.server.received().is_empty());

    let unlinked = Fresh::with_routes(&spa(json!({})), vec![]).await;
    let (status, body) = call(&unlinked, "POST", COMMENTS, json!({"body": "hi"})).await;
    assert_eq!(status, 422);
    assert_eq!(
        body["error"]["message"],
        "Connect GitHub to comment and review from here as yourself."
    );
    assert!(unlinked.server.received().is_empty());

    let refused = Fresh::with_routes(
        &spa(json!({"linked": true})),
        vec![route("POST", "/repos/rails/rails/issues/12/comments", 403)],
    )
    .await;
    let before_refusal = message_count(&refused).await;
    let (status, body) = call(&refused, "POST", COMMENTS, json!({"body": "hi"})).await;
    assert_eq!(status, 422, "{body}");
    assert_eq!(body["error"]["message"], "GitHub refused: no");
    assert_eq!(message_count(&refused).await, before_refusal);

    let rejected = Fresh::with_routes(
        &spa(json!({"linked": true})),
        vec![
            Route::new(
                "POST",
                "api.github.com",
                "/repos/rails/rails/pulls/12/reviews",
                401,
            )
            .body("{}"),
        ],
    )
    .await;
    let (status, body) = call(&rejected, "POST", REVIEWS, json!({"event": "approve"})).await;
    assert_eq!(status, 422, "{body}");
    assert_eq!(
        body["error"]["message"],
        "GitHub rejected your token. Reconnect to post."
    );
    let reason = rejected
        .app
        .db
        .read(|conn| {
            Ok(
                crate::integrations::github::accounts::Account::for_user(conn, 811)?
                    .and_then(|account| account.disconnected_reason),
            )
        })
        .await
        .unwrap();
    assert_eq!(
        reason.as_deref(),
        Some("GitHub rejected the linked token (401)")
    );

    for input in [
        json!({"member": false, "linked": true}),
        json!({"mapping": false, "linked": true}),
    ] {
        let fresh = Fresh::with_routes(&spa(input.clone()), vec![]).await;
        let (status, _) = call(&fresh, "POST", COMMENTS, json!({"body": "hi"})).await;
        assert_eq!(status, 404, "{input}");
        assert!(fresh.server.received().is_empty());
    }
}

#[tokio::test]
async fn bot_credentials_are_forbidden_and_never_reach_github() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let fresh = Fresh::with_routes(&spa(json!({"linked": true})), vec![]).await;
    let key = fresh
        .app
        .db
        .write(|tx| {
            Ok(
                campfire_db::User::create_bot(tx, "Write test machine", None)?
                    .plain_bot_key()
                    .unwrap(),
            )
        })
        .await
        .unwrap();
    for path in [COMMENTS, REVIEWS, REVIEWERS] {
        let response = fresh
            .router
            .clone()
            .oneshot(
                Request::post(format!(
                    "{path}?{}",
                    url::form_urlencoded::Serializer::new(String::new())
                        .append_pair("bot_key", &key)
                        .finish()
                ))
                .header("Host", "example.org")
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .body(Body::from(
                    json!({"body": "hi", "event": "approve", "reviewers": "alice"}).to_string(),
                ))
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 403, "{path}");
        let text = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(!text.contains("fixture-viewer-token"), "{text}");
        assert!(!text.contains(&key), "{path}");
    }
    assert!(fresh.server.received().is_empty());
}

async fn message_count(fresh: &Fresh) -> i64 {
    fresh
        .app
        .db
        .read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))?))
        .await
        .unwrap()
}
