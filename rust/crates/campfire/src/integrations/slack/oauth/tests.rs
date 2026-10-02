use super::*;
use crate::integrations::{slack::client::tests::fake, test_support::Route};
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../../../vectors/slack/oauth.json")).unwrap()
}
#[test]
fn slack_oauth_manifest_urls_and_state_match_real_rails() {
    let v = vectors();
    assert_eq!(manifest("https://example.invalid"), v["manifest"]);
    for case in v["authorize"].as_array().unwrap() {
        assert_eq!(
            authorize_url(
                "client +&",
                "https://example.invalid/slack/oauth/callback",
                "raw +state",
                case["team"].as_str()
            ),
            case["url"]
        );
    }
    let secrets = Secrets::new(v["state"]["secret_key_base"].as_str().unwrap());
    let raw = v["state"]["raw"].as_str().unwrap();
    let signed = sign_state(&secrets, raw);
    assert_eq!(signed, v["state"]["signed"]);
    let now = "2026-03-02T16:00:00Z".parse().unwrap();
    let stored = json!({"state":raw,"user_id":811});
    assert!(valid_state(&secrets, &signed, Some(&stored), 811, now));
    for stored in [
        json!({"state":"other","user_id":811}),
        json!({"state":raw,"user_id":812}),
        json!({"state":9,"user_id":811}),
        json!({}),
    ] {
        assert!(!valid_state(&secrets, &signed, Some(&stored), 811, now));
    }
    let mut bad = signed.clone();
    bad.push('x');
    assert!(!valid_state(&secrets, &bad, Some(&stored), 811, now));
    for (name, purpose, expiry) in [
        ("github_app_oauth_state", None, None),
        ("slack_oauth_state", Some("other"), None),
        ("slack_oauth_state", None, Some(now)),
    ] {
        let signed =
            rails_compat::app_verifier(&secrets, name).generate(&json!(raw), purpose, expiry);
        assert!(!valid_state(&secrets, &signed, Some(&stored), 811, now));
    }
}
#[tokio::test]
async fn slack_oauth_exchange_revoke_and_team_info_match_rails_through_real_tls() {
    for case in vectors()["cases"].as_array().unwrap() {
        if case["input"].get("exception").is_some() {
            continue;
        }
        let i = &case["input"];
        let status = i["status"].as_u64().unwrap() as u16;
        let body = i["body"].as_str().unwrap();
        let (server, network) = fake(vec![
            Route::new("POST", "slack.com", "/api/oauth.v2.access", status).body(body),
            Route::new("POST", "slack.com", "/api/auth.revoke", status).body(body),
            Route::new("GET", "slack.com", "/api/team.info", status).body(body),
        ])
        .await;
        let oauth = OAuth { network };
        let result = match oauth
            .exchange_code(
                "fixture-client",
                "fixture-secret",
                "fixture-code",
                "https://example.invalid/slack/oauth/callback",
            )
            .await
        {
            Ok(v) => json!({"body":v}),
            Err(e) => json!({"error":e.to_string()}),
        };
        assert_eq!(result, case["expected"]["exchange"], "{i}");
        assert_eq!(
            oauth.revoke("fixture-user-grant").await,
            case["expected"]["revoke"]
        );
        assert_eq!(
            oauth
                .team_info("fixture-user-grant")
                .await
                .unwrap_or(Value::Null),
            case["expected"]["team_info"]
        );
        let requests = server.received();
        assert_eq!(requests.len(), 3);
        for (actual, expected) in requests.iter().zip(case["requests"].as_array().unwrap()) {
            assert_eq!(actual.method, expected["method"]);
            assert_eq!(actual.target, expected["path"]);
            assert_eq!(actual.header("accept"), Some("application/json"));
            assert_eq!(actual.header("connection"), None);
            if actual.method == "POST" {
                assert_eq!(
                    actual.header("content-type"),
                    Some("application/x-www-form-urlencoded")
                );
                let form: serde_json::Map<String, Value> =
                    url::form_urlencoded::parse(&actual.body)
                        .map(|(k, v)| (k.into_owned(), json!(v)))
                        .collect();
                assert_eq!(Value::Object(form), expected["form"]);
                assert!(actual.header("authorization").is_none());
            } else {
                assert_eq!(
                    actual.header("authorization"),
                    Some(format!("Bearer {}", "fixture-user-grant").as_str())
                );
            }
        }
    }
}
#[test]
fn slack_oauth_transport_errors_never_include_details() {
    use std::io;
    let errors = [
        ("Net::OpenTimeout", HttpError::OpenTimeout),
        ("Net::ReadTimeout", HttpError::ReadTimeout),
        ("Net::WriteTimeout", HttpError::WriteTimeout),
        (
            "SocketError",
            HttpError::Unresolvable("fixture-secret".into()),
        ),
        (
            "OpenSSL::SSL::SSLError",
            HttpError::Tls("fixture-secret".into()),
        ),
        ("EOFError", HttpError::ConnectionClosed),
        (
            "Errno::ECONNREFUSED",
            HttpError::Io(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "fixture-secret",
            )),
        ),
    ];
    for (name, error) in errors {
        let v = vectors();
        let expected = v["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["input"]["exception"] == name)
            .unwrap();
        assert_eq!(
            transport_error(error).to_string(),
            expected["expected"]["exchange"]["error"]
        );
    }
}
