use super::*;

#[tokio::test]
async fn github_callback_returns_to_the_spa_integrations_whatever_was_chosen_in_every_handled_outcome() {
    use campfire_db::models::user::ui_preference::{self, UiPreference};
    for enabled in [false, true] {
        for preference in [UiPreference::Classic, UiPreference::Next] {
            for outcome in ["success", "expired", "cancelled", "rejected", "unreachable"] {
                let routes = vec![
                    Route::new("POST", "github.com", "/login/oauth/access_token", if outcome == "unreachable" { 500 } else { 200 }).body(if outcome == "rejected" { r#"{"error":"denied"}"# } else { r#"{"access_token":"fixture-app-token","refresh_token":"fixture-refresh","expires_in":28800}"# }),
                    Route::new("GET", "api.github.com", "/user", 200).body(r#"{"login":"octocat"}"#),
                ];
                let fresh = Fresh::with_routes(
                    &json!({"app_configured":true,"spa_enabled":enabled}),
                    routes,
                )
                .await;
                fresh
                    .app
                    .db
                    .write(move |tx| ui_preference::store(tx, 811, preference))
                    .await
                    .unwrap();
                let (_, headers, _) =
                    request(&fresh, "GET", "/github/app/connect", Value::Null, sudo()).await;
                let state = url::Url::parse(headers["location"].to_str().unwrap())
                    .unwrap()
                    .query_pairs()
                    .find(|(k, _)| k == "state")
                    .unwrap()
                    .1
                    .into_owned();
                let session = response_session(&fresh, &headers);
                let path = if outcome == "expired" {
                    "/github/app/callback?state=bogus".into()
                } else {
                    format!(
                        "/github/app/callback?state={}&{}",
                        crate::controllers::presenters::test_support::encode(&state),
                        if outcome == "cancelled" {
                            "error=denied"
                        } else {
                            "code=fixture-code"
                        }
                    )
                };
                let (status, headers, _) =
                    request(&fresh, "GET", &path, Value::Null, session).await;
                assert_eq!(status, 302, "{outcome}");
                assert_eq!(
                    headers["location"],
                    "http://example.org/app/settings/integrations",
                    "{outcome}"
                );
                let session = response_session(&fresh, &headers);
                assert!(session.get("github_app_oauth_state").is_none());
                let (kind, message) = match outcome {
                    "success" => ("notice", "GitHub connected as octocat."),
                    "expired" => ("alert", "GitHub connection expired. Try again."),
                    "cancelled" => ("alert", "GitHub connection was not approved."),
                    "rejected" => ("alert", "GitHub rejected the connection. Try again."),
                    _ => ("alert", "Could not reach GitHub. Try again."),
                };
                assert_eq!(session["flash"]["flashes"][kind], message, "{outcome}");
                if matches!(outcome, "expired" | "cancelled") {
                    assert!(fresh.server.received().is_empty());
                }
            }
        }
    }
}
