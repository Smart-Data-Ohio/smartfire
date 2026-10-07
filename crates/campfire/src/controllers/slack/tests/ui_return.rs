use super::*;

#[tokio::test]
async fn slack_oauth_returns_to_validated_spa_screen_only_for_next_ui() {
    use campfire_db::models::user::ui_preference::{self, UiPreference};
    for preference in [UiPreference::Classic, UiPreference::Next] {
        for (role, classic, next) in [
            (1, "/account/slack_import", "/app/admin/slack"),
            (0, "/slack/imports", "/app/settings/slack"),
        ] {
            for outcome in [
                "success",
                "expired",
                "cancelled",
                "exchange-failed",
                "unconfigured",
            ] {
                let exchange = json!({"ok":true,"team":{"id":"TFIXTURE","name":"Fixture"},"authed_user":{"id":"UFIXTURE","access_token":"fixture-user-grant","scope":crate::integrations::slack::oauth::USER_SCOPES.join(",")}});
                let f = Fresh::with_spa(
                    role,
                    vec![
                        Route::new(
                            "POST",
                            "slack.com",
                            "/api/oauth.v2.access",
                            if outcome == "exchange-failed" {
                                500
                            } else {
                                200
                            },
                        )
                        .body(exchange.to_string()),
                    ],
                    true,
                )
                .await;
                f.app
                    .db
                    .write(move |tx| ui_preference::store(tx, 811, preference))
                    .await
                    .unwrap();
                if outcome != "unconfigured" {
                    f.workspace().await;
                }
                let start = format!(
                    "/slack/oauth/start?return_to={}",
                    crate::controllers::presenters::test_support::encode(classic)
                );
                let (_, headers, _) = request(&f, "GET", &start, Value::Null, sudo()).await;
                let destination = if preference == UiPreference::Next {
                    next
                } else {
                    classic
                };
                let session = if outcome == "unconfigured" {
                    assert_eq!(
                        headers["location"],
                        format!("http://example.org{destination}")
                    );
                    response_session(&f, &headers)
                } else {
                    let state = url::Url::parse(headers["location"].to_str().unwrap())
                        .unwrap()
                        .query_pairs()
                        .find(|(k, _)| k == "state")
                        .unwrap()
                        .1
                        .into_owned();
                    let saved = response_session(&f, &headers);
                    assert_eq!(saved["slack_oauth_return_to"], classic);
                    let path = if outcome == "expired" {
                        "/slack/oauth/callback?state=bogus".into()
                    } else {
                        format!(
                            "/slack/oauth/callback?state={}&{}",
                            crate::controllers::presenters::test_support::encode(&state),
                            if outcome == "cancelled" {
                                "error=denied"
                            } else {
                                "code=fixture-code"
                            }
                        )
                    };
                    let (status, headers, _) = request(&f, "GET", &path, Value::Null, saved).await;
                    assert_eq!(status, 302);
                    assert_eq!(
                        headers["location"],
                        format!("http://example.org{destination}")
                    );
                    response_session(&f, &headers)
                };
                assert!(session.get("slack_oauth_state").is_none());
                assert!(session.get("slack_oauth_return_to").is_none());
                let (kind, message) = match outcome {
                    "success" => ("notice", "Slack connected."),
                    "expired" => ("alert", "Slack connection expired. Try again."),
                    "cancelled" => ("alert", "Slack connection was not approved."),
                    "unconfigured" => ("alert", "Set up the Slack app credentials first."),
                    _ => ("alert", "Could not connect Slack. Try again."),
                };
                assert_eq!(session["flash"]["flashes"][kind], message, "{outcome}");
                if matches!(outcome, "expired" | "cancelled" | "unconfigured") {
                    assert!(f.server.received().is_empty());
                }
            }
        }
    }
}
