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
        // SPA cutover: the retained sign-in page now redirects to the SPA's, whose inline boot
        // carries the Google configuration the retained page drew; the public links and the
        // Google form contract stay asserted on the retained renderer above.
        let mut browser = app.anonymous();
        let retained = browser.get("/session/new").await;
        assert_eq!(retained.status, axum::http::StatusCode::FOUND);
        assert_eq!(retained.location(), Some("http://campfire.test/app/session/new"));
        let page = browser.get("/app/session/new").await;
        assert_eq!(page.status, axum::http::StatusCode::OK);
        let methods = crate::controllers::spa::signed_out_contracts_tests::inline_boot(&page)
            ["signInMethods"]
            .clone();
        let configured = case["configured"].as_bool().unwrap();
        assert_eq!(
            methods["google"].as_bool(),
            Some(configured),
            "{}: configuration",
            case["name"]
        );
        assert_eq!(
            actual.contains("Sign in with Google"),
            configured,
            "{}: retained configuration",
            case["name"]
        );
        if configured {
            assert!(actual.contains("action=\"/session/google\""));
            assert_eq!(methods["googleDomains"], case["domains"], "{}: domains", case["name"]);
            for domain in case["domains"].as_array().unwrap() {
                assert!(actual.contains(&format!("@{}", domain.as_str().unwrap())));
            }
        }
        super::tests::assert_public_links(&actual);
    }
}
