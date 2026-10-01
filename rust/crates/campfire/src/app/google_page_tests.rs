//! Complete configured HTTP pages captured by the pinned Rails application.
use super::google_api_tests::{self as support, Recorded};
use crate::{
    controllers::presenters::test_support::{DAVID, TestApp, with_fixed_render_secrets},
    integrations::google::{api, sign_in},
};
use campfire_db::{Timestamp, models::google_identity::GoogleIdentity};
use campfire_kit::FrozenClock;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
#[tokio::test]
async fn google_complete_login_pages_and_configured_profile_components_match_rails() {
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/google_full_pages.json")).unwrap();
    let mut differences = vec![];
    for row in oracle["rows"].as_array().unwrap() {
        let spec = &row["spec"];
        let name = spec["name"].as_str().unwrap();
        let configured = spec["config"] != "missing_credentials";
        let domains = if spec["config"] == "empty_domains" {
            vec![]
        } else {
            vec!["smartdata.net".into(), "cnbssoftware.com".into()]
        };
        let at: jiff::Timestamp = oracle["now"].as_str().unwrap().parse().unwrap();
        let mut a = TestApp::boot_with_clock_and_env(
            Arc::new(FrozenClock::new(at)),
            &[(
                "VAPID_PUBLIC_KEY",
                oracle["vapid_public_key"].as_str().unwrap_or_default(),
            )],
        )
        .await
        .unwrap();
        a.booted.jobs.stop(Duration::from_secs(5)).await;
        let r = Recorded::new(vec![]);
        let mut config = support::config();
        if !configured {
            config.client_id.clear();
            config.client_secret.clear();
        }
        a.booted
            .app
            .google
            .install_api(api::Api::new(config, r.clone()));
        a.booted.app.google.install(sign_in::SignIn::with_client(
            sign_in::Config {
                client_id: if configured {
                    "test-client-id".into()
                } else {
                    String::new()
                },
                client_secret: if configured {
                    "FAKE-page-secret".into()
                } else {
                    String::new()
                },
                domains,
            },
            r.clone(),
        ));
        a.db()
            .write(|tx| {
                tx.conn()
                    .execute("DELETE FROM google_accounts WHERE user_id=?", [DAVID])?;
                tx.conn()
                    .execute("DELETE FROM google_identities WHERE user_id=?", [DAVID])?;
                Ok(())
            })
            .await
            .unwrap();
        if let Some(scope) = row["scope"].as_str() {
            support::grant(
                &a,
                DAVID,
                Timestamp::from_jiff(at).since(jiff::SignedDuration::from_hours(1)),
                false,
            )
            .await;
            let scope = scope.to_owned();
            let disconnected = spec["account"] == "disconnected";
            a.db().write(move |tx| {tx.conn().execute("UPDATE google_accounts SET email='david@smartdata.net',scopes=?,disconnected_reason=? WHERE user_id=?",rusqlite::params![scope,disconnected.then_some("revoked"),DAVID])?;Ok(())}).await.unwrap();
        }
        if spec["account"] == "linked" {
            a.db().write(|tx| GoogleIdentity::link_to_user(tx,json!({"sub":"page-fixture","email":"david@smartdata.net","hd":"smartdata.net"}).as_object().unwrap(),DAVID)).await.unwrap();
        }
        let profile = spec["page"] == "profile";
        let mut browser = if profile { a.david() } else { a.anonymous() };
        let path = if profile {
            "/users/me/profile"
        } else {
            "/session/new"
        };
        let response = with_fixed_render_secrets(browser.get(path)).await;
        assert_eq!(
            response.status.as_u16() as u64,
            row["status"].as_u64().unwrap(),
            "{name}"
        );
        let actual = response.text();
        let expected = row["body"].as_str().unwrap();
        let matches = if profile {
            let panels = ["google-calendar-title", "google-sign-in-title"]
                .into_iter()
                .map(|id| {
                    regex::Regex::new(&format!(
                        r#"(?s)<section[^>]*aria-labelledby="{id}".*?</section>"#
                    ))
                    .unwrap()
                    .find(&actual)
                    .map(|m| m.as_str())
                })
                .collect::<Vec<_>>();
            let settings = actual
                .lines()
                .filter(|l| {
                    l.contains("<p")
                        && (l.contains("Google Calendar")
                            || l.contains("Google calendar")
                            || l.contains("Reconnect Google"))
                })
                .map(str::trim)
                .collect::<Vec<_>>();
            assert_eq!(
                json!(panels),
                row["google_panels"],
                "{name}: complete Google panels"
            );
            assert_eq!(
                json!(settings),
                row["google_settings"],
                "{name}: complete Google settings notices"
            );
            true
        } else {
            super::asset_goldens::compare(name, &actual, expected)
        };
        if !matches {
            if let Ok(dir) = std::env::var("WS14G_PAGE_DIFF_DIR") {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(format!("{dir}/{name}.actual"), &actual).unwrap();
                std::fs::write(format!("{dir}/{name}.expected"), expected).unwrap();
            }
            differences.push(name.to_owned());
        }
        assert!(
            r.calls.lock().unwrap().is_empty(),
            "{name}: pages must never call Google"
        );
    }
    assert!(
        differences.is_empty(),
        "complete Google page differences: {differences:?}"
    );
    println!(
        "Pinned Rails Google HTML: 3 complete login pages and 16 complete profile panels exercised; 0 skipped; 8 full profile pages owner-blocked"
    );
}
