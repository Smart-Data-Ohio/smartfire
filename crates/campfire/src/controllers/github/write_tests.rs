use super::{
    card_tests::Fresh,
    test_support::{request, sudo},
};
use serde_json::{Value, json};
#[tokio::test]
async fn github_write_security_checks_room_and_mapping_before_own_token_access() {
    for input in [
        json!({"member":false,"linked":true}),
        json!({"mapping":false,"linked":true}),
        json!({"deleted":true,"linked":true}),
    ] {
        let fresh = Fresh::new(&input).await;
        for path in [
            "/rooms/815/github/pull_request_comments",
            "/rooms/815/github/pull_request_reviews",
            "/rooms/815/github/pull_request_review_requests",
            "/rooms/815/github/pull_request_write_actions/816",
        ] {
            assert_eq!(request(&fresh,if path.contains("pull_request_write_actions"){"GET"}else{"POST"},path,json!({"pull_request_id":816,"body":"hi","event":"APPROVE","reviewers":"alice"}),sudo()).await.0,404,"{input} {path}");
        }
        assert!(fresh.server.received().is_empty());
    }
}
#[tokio::test]
async fn github_write_http_results_payloads_own_token_prompts_retry_text_and_streams_match_rails() {
    use crate::integrations::test_support::Route;
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/github_write_http.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let action = case["action"].as_str().unwrap();
        let path = case["path"].as_str().unwrap();
        let api_path = match action {
            "comments" => "/repos/rails/rails/issues/12/comments",
            "reviews" => "/repos/rails/rails/pulls/12/reviews",
            _ => "/repos/rails/rails/pulls/12/requested_reviewers",
        };
        let mut input = case.clone();
        input["mapping"] = case.get("mapping").cloned().unwrap_or(json!(true));
        let fresh = Fresh::with_routes(
            &input,
            vec![
                Route::new(
                    "POST",
                    "api.github.com",
                    api_path,
                    case["api_status"].as_u64().unwrap_or(201) as u16,
                )
                .body(
                    json!({"message":"Denied <review> @[Member]\nTry again","id":12}).to_string(),
                ),
            ],
        )
        .await;
        let count = fresh
            .app
            .db
            .read(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?)
            })
            .await
            .unwrap();
        let (status, headers, body) = super::test_support::request_with_accept(
            &fresh,
            if action == "show" { "GET" } else { "POST" },
            path,
            case["request_body"].clone(),
            sudo(),
            if case["stream"] == true {
                "text/vnd.turbo-stream.html"
            } else {
                "text/html"
            },
        )
        .await;
        assert_eq!(status, case["status"], "{} {body}", case["name"]);
        if status != 404 {
            assert_eq!(
                headers["content-type"],
                case["content_type"].as_str().unwrap(),
                "{}",
                case["name"]
            );
            let data: campfire_views::github::write_actions::WriteActions =
                serde_json::from_value(case["render_data"].clone()).unwrap();
            let fragment = data.render();
            let expected = if case["stream"] == true {
                format!(
                    "<turbo-stream action=\"replace\" target=\"github_write_actions_channel_thread_817\"><template>{fragment}</template></turbo-stream>"
                )
            } else {
                fragment
            };
            assert_eq!(
                expected,
                case["body"].as_str().unwrap(),
                "{} detached exact bytes",
                case["name"]
            );
            // HTTP forms contain real request-local CSRF tokens; prompts/errors and input preservation match.
            for value in [
                data.notice.as_deref(),
                data.alert.as_deref(),
                data.comment_body.as_deref(),
                data.review_body.as_deref(),
                data.reviewers_body.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                assert!(
                    body.contains(&campfire_views::helpers::escape(value)),
                    "{} {value}",
                    case["name"]
                );
            }
            assert_eq!(body.contains("github-pr-write__comment"), data.usable);
            assert_eq!(body.contains("name=\"authenticity_token\""), data.usable);
            assert_eq!(
                body.contains("Reconnect GitHub"),
                data.linked && !data.usable
            );
            assert_eq!(body.contains(">Connect GitHub<"), !data.linked);
            if data.usable {
                // The cleared/retained fields from the actual HTTP body equal the oracle's form fields.
                for (name, value) in [
                    ("comment", data.comment_body.as_deref()),
                    ("review", data.review_body.as_deref()),
                ] {
                    let class = if name == "comment" {
                        "github-pr-write__comment"
                    } else {
                        "github-pr-write__review"
                    };
                    let form = body
                        .split(&format!("class=\"{class}\""))
                        .nth(1)
                        .unwrap()
                        .split("</form>")
                        .next()
                        .unwrap();
                    let area = form
                        .split("</textarea>")
                        .next()
                        .unwrap()
                        .rsplit('>')
                        .next()
                        .unwrap();
                    assert_eq!(
                        area.strip_prefix('\n').unwrap_or(area),
                        campfire_views::helpers::escape(value.unwrap_or("")),
                        "{} {name}",
                        case["name"]
                    );
                }
            }
        }
        let requests = fresh.server.received();
        assert_eq!(
            requests.len(),
            case["requests"].as_array().unwrap().len(),
            "{}",
            case["name"]
        );
        for (actual, expected) in requests.iter().zip(case["requests"].as_array().unwrap()) {
            assert_eq!(actual.method, expected[0]);
            assert_eq!(actual.target, expected[1]);
            assert_eq!(
                serde_json::from_slice::<Value>(&actual.body).unwrap(),
                expected[2]
            );
            assert!(
                actual
                    .headers
                    .iter()
                    .any(|(k, v)| k.eq_ignore_ascii_case("authorization")
                        && v == &format!("Bearer {}", expected[3].as_str().unwrap()))
            );
        }
        let (disconnected, after) = fresh
            .app
            .db
            .read(|conn| {
                Ok((
                    crate::integrations::github::accounts::Account::for_user(conn, 811)?
                        .and_then(|a| a.disconnected_reason),
                    conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?,
                ))
            })
            .await
            .unwrap();
        assert_eq!(
            json!(disconnected),
            case["disconnected"],
            "{}",
            case["name"]
        );
        assert_eq!(after, count);
        assert!(!body.contains("fixture-viewer-token"));
    }
}
#[test]
fn github_review_logins_normalization_matches_rails_odd_shapes_and_boundaries() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/github_review_logins.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        assert_eq!(
            json!(crate::integrations::github::actions::normalize_reviewers(
                &case["input"]
            )),
            case["expected"],
            "{}",
            case["input"]
        );
    }
}
#[tokio::test]
async fn github_write_bot_credentials_are_forbidden_and_never_reach_github() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let fresh = Fresh::new(&json!({})).await;
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
    for path in [
        "/rooms/815/github/pull_request_comments",
        "/rooms/815/github/pull_request_reviews",
        "/rooms/815/github/pull_request_review_requests",
    ] {
        let response=fresh.router.clone().oneshot(Request::post(format!("{path}?{}",url::form_urlencoded::Serializer::new(String::new()).append_pair("bot_key",&key).finish())).header("Host","example.org").header("Content-Type","application/json").body(Body::from(json!({"pull_request_id":816,"body":"hi","event":"APPROVE","reviewers":"alice"}).to_string())).unwrap()).await.unwrap();
        assert_eq!(response.status(), 403, "{path}");
    }
    assert!(fresh.server.received().is_empty());
}
