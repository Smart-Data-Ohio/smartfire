//! The configured public-links criterion, through WS9's actual sign-in renderer.
use crate::controllers::presenters::{self, test_support::*};
use askama::Template;
use campfire_retained::sessions;
use serde_json::Value;

#[tokio::test]
async fn configured_sign_in_keeps_public_links_and_rails_form_contracts() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_sign_in_google.json"
    ))
    .unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        let environment = case["environment"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str().unwrap()))
            .collect::<Vec<_>>();
        let app = TestApp::boot_with_clock_and_env(
            std::sync::Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap())),
            &environment,
        )
        .await
        .expect("seed required");
        let (account, help) = app
            .db()
            .read(|conn| {
                Ok((
                    campfire_db::Account::first(conn)?,
                    presenters::accounts::help_contact(conn)?,
                ))
            })
            .await
            .unwrap();
        let actual = crate::controllers::users::people_tests::render_with(
            &app,
            |ctx| {
                ctx.current_user = None;
                ctx.account = presenters::view_context::account_summary(account.as_ref(), false);
            },
            |ctx| {
                let ctx = crate::controllers::users::people_tests::retained(ctx);
                sessions::New {
                    ctx: &ctx,
                    email_address: None,
                    help_contact: help,
                    google_sign_in_domains: app.booted.app.config.sign_in_google_domains.clone(),
                }
                .as_content()
                .render()
                .unwrap()
            },
        );
        crate::form_contracts::assert_forms(case["name"].as_str().unwrap(), &actual, case["body"].as_str().unwrap());
        crate::form_contracts::assert_text(&actual, &account.as_ref().unwrap().name);
        let page = app.anonymous().get("/session/new").await;
        assert_eq!(page.status, axum::http::StatusCode::OK);
        let body = page.text();
        let configured = case["configured"].as_bool().unwrap();
        assert_eq!(
            body.contains("Sign in with Google"),
            configured,
            "{}: configuration",
            case["name"]
        );
        if configured {
            assert!(body.contains("action=\"/session/google\""));
            for domain in case["domains"].as_array().unwrap() {
                assert!(body.contains(&format!("@{}", domain.as_str().unwrap())));
            }
        }
        super::tests::assert_public_links(&body);
    }
}
