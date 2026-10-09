use crate::controllers::presenters::test_support::*;
use askama::Template;
use axum::http::{Method, StatusCode};
use campfire_db::{Account, User};
use serde_json::{Value, json};
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/users_first_run.json")).unwrap()
}
async fn app() -> TestApp {
    TestApp::boot_seed_with_env(
        "first_run",
        std::sync::Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap())),
        &[],
    )
    .await
    .expect("seed required")
}
#[tokio::test]
async fn signup_body_matches_rails_and_is_available_until_account_exists() {
    let app = app().await;
    let actual = crate::controllers::users::people_tests::render_with(
        &app,
        |ctx| {
            ctx.current_user = None;
        },
        |ctx| {
            let ctx = crate::controllers::users::people_tests::retained(ctx);
            campfire_retained::first_runs::Show { ctx: &ctx }
                .as_content()
                .render()
                .unwrap()
        },
    );
    crate::form_contracts::assert_forms(
        "first-run-body",
        &actual,
        vectors()["page"]["html"].as_str().unwrap(),
    );
    let mut browser = app.anonymous();
    assert_eq!(browser.get("/first_run").await.status, StatusCode::OK);
    app.db()
        .write(|tx| Account::create(tx, "Chat"))
        .await
        .unwrap();
    assert_eq!(
        app.anonymous().get("/first_run").await.location(),
        Some("http://campfire.test/")
    );
    assert_eq!(
        browser
            .write(Req::new(Method::POST, "/first_run").form(&[("user[name]", "must not create")]))
            .await
            .location(),
        Some("http://campfire.test/")
    );
}
#[tokio::test]
async fn first_run_http_and_persisted_state_match_all_rails_cases() {
    for case in vectors()["cases"].as_array().unwrap() {
        let app = app().await;
        let mut browser = app.anonymous();
        let mut fields = Vec::new();
        for (object, attrs) in case["params"].as_object().unwrap() {
            for (k, v) in attrs.as_object().unwrap() {
                fields.push((format!("{object}[{k}]"), v.as_str().unwrap().to_owned()));
            }
        }
        let fields = fields
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>();
        let response = browser
            .write(Req::new(Method::POST, "/first_run").form(&fields))
            .await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{}",
            case["name"]
        );
        assert_eq!(response.location(), case["location"].as_str());
        let state=app.db().read(|c|{
            let accounts=c.prepare("SELECT name FROM accounts ORDER BY id")?.query_map([],|r|r.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()?;
            let users=User::all(c)?.iter().map(|u|json!({"name":u.name,"email":u.email_address,"role":u.role.name(),"password_present":u.password_digest.as_deref().is_some_and(|s|!s.is_empty())})).collect::<Vec<_>>();
            let rooms=c.prepare("SELECT name,type FROM rooms ORDER BY id")?.query_map([],|r|Ok(json!([r.get::<_,String>(0)?,r.get::<_,String>(1)?])))?.collect::<Result<Vec<_>,_>>()?;
            let count=|table:&str|c.query_row(&format!("SELECT COUNT(*) FROM {table}"),[],|r|r.get::<_,i64>(0));
            Ok(json!({"account_names":accounts,"users":users,"rooms":rooms,"memberships":count("memberships")?,"sessions":count("sessions")?,"audits":count("audit_logs")?}))
        }).await.unwrap();
        assert_eq!(state, case["state"], "{}", case["name"]);
        if response.status == StatusCode::FOUND {
            assert!(browser.cookie_header().contains("session_token="));
            assert_eq!(
                browser.get("/").await.location(),
                Some("http://campfire.test/two_factor_setup")
            );
        }
    }
}
#[tokio::test]
async fn five_concurrent_first_runs_create_one_administrator_and_room() {
    let app = app().await;
    let mut browsers = Vec::new();
    for _ in 0..5 {
        let mut browser = app.anonymous();
        browser.get("/first_run").await;
        browsers.push(browser);
    }
    let responses = futures_util::future::join_all(browsers.into_iter().enumerate().map(
        |(i, mut browser)| async move {
            let name = format!("Attacker{i}");
            let email = format!("attacker{i}@example.com");
            browser
                .write(Req::new(Method::POST, "/first_run").form(&[
                    ("user[name]", &name),
                    ("user[email_address]", &email),
                    ("user[password]", "password123"),
                ]))
                .await
        },
    ))
    .await;
    assert!(
        responses
            .iter()
            .all(|r| r.location() == Some("http://campfire.test/"))
    );
    app.db()
        .read(|c| {
            for table in ["accounts", "users", "rooms"] {
                assert_eq!(
                    c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                        .get::<_, i64>(0))?,
                    1,
                    "{table}"
                );
            }
            assert_eq!(User::all(c)?[0].role, campfire_db::Role::Administrator);
            Ok(())
        })
        .await
        .unwrap();
}
