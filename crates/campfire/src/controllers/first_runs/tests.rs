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

// The SPA's JSON contract (`/api/v1/first_run`) against the retained form, each on a fresh
// first-run seed: the same rows, cookies and session for the same submission.

use crate::controllers::spa::signed_out_contracts_tests::{
    body_token, browser_session, cookie_contract, inline_boot, json_request,
};

const AVATAR: &[u8] = include_bytes!("../../../../../fixtures/files/workspace_icons/square_64.png");

/// Every row first run writes, without ids, keys and times that differ between two runs.
async fn rows(app: &TestApp) -> Value {
    app.db()
        .read(|c| {
            let all = |sql: &str, width: usize| -> rusqlite::Result<Vec<Value>> {
                c.prepare(sql)?
                    .query_map([], |r| {
                        (0..width)
                            .map(|i| {
                                Ok(match r.get_ref(i)? {
                                    rusqlite::types::ValueRef::Null => Value::Null,
                                    rusqlite::types::ValueRef::Integer(n) => json!(n),
                                    rusqlite::types::ValueRef::Real(n) => json!(n),
                                    rusqlite::types::ValueRef::Text(t) => {
                                        json!(String::from_utf8_lossy(t))
                                    }
                                    rusqlite::types::ValueRef::Blob(b) => json!(b.len()),
                                })
                            })
                            .collect::<rusqlite::Result<Vec<_>>>()
                            .map(Value::Array)
                    })?
                    .collect()
            };
            Ok(json!({
                "accounts": all("SELECT name FROM accounts ORDER BY id", 1)?,
                "users": all("SELECT id,name,email_address,role,status,password_digest IS NOT NULL AND password_digest != '' FROM users ORDER BY id", 6)?,
                "rooms": all("SELECT id,name,type,creator_id FROM rooms ORDER BY id", 4)?,
                "memberships": all("SELECT room_id,user_id,involvement FROM memberships ORDER BY id", 3)?,
                "sessions": all("SELECT user_id,user_agent,ip_address,two_factor_verified_at FROM sessions ORDER BY id", 4)?,
                "attachments": all("SELECT a.name,a.record_type,a.record_id,b.filename,b.content_type,b.byte_size,b.checksum FROM active_storage_attachments a JOIN active_storage_blobs b ON b.id=a.blob_id ORDER BY a.id", 7)?,
                "audits": all("SELECT action FROM audit_logs ORDER BY id", 1)?,
            }))
        })
        .await
        .unwrap()
}

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    /// `None` leaves the field out of the submission altogether.
    email: Option<&'static str>,
    password: Option<&'static str>,
    avatar: bool,
}

/// Submits `case` as the retained form does, or as the SPA does; answers where it led.
async fn submit(browser: &mut Browser<'_>, case: Case, json: bool) -> (Reply, String) {
    let reply = if json {
        let state = browser.get("/api/v1/first_run").await;
        assert_eq!(state.status, StatusCode::OK);
        let state = state.json();
        assert_eq!(state["kind"], "pending");
        let token = state["csrfToken"].as_str().unwrap().to_owned();
        let mut submission = json!({"name": case.name});
        if let Some(email) = case.email {
            submission["emailAddress"] = json!(email);
        }
        if let Some(password) = case.password {
            submission["password"] = json!(password);
        }
        let request = if case.avatar {
            Req::new(Method::POST, "/api/v1/first_run")
                .header("accept", "application/json")
                .multipart(
                    &[("submission", &submission.to_string())],
                    ("avatar", "me.png", "image/png", AVATAR),
                )
        } else {
            json_request(Method::POST, "/api/v1/first_run", submission)
        };
        browser.send(request.header("x-csrf-token", &token)).await
    } else {
        assert_eq!(browser.get("/first_run").await.status, StatusCode::OK);
        let fields = [
            Some(("user[name]", case.name)),
            case.email.map(|email| ("user[email_address]", email)),
            case.password.map(|password| ("user[password]", password)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let request = if case.avatar {
            Req::new(Method::POST, "/first_run")
                .multipart(&fields, ("user[avatar]", "me.png", "image/png", AVATAR))
        } else {
            Req::new(Method::POST, "/first_run").form(&fields)
        };
        browser.write(request).await
    };
    let location = if json {
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let answer = reply.json();
        assert_eq!(answer["kind"], "signedIn", "{answer}");
        answer["location"].as_str().unwrap().to_owned()
    } else {
        assert_eq!(reply.status, StatusCode::FOUND, "{}", reply.text());
        reply.location().unwrap().to_owned()
    };
    (reply, location)
}

#[tokio::test]
async fn json_first_run_creates_the_same_rows_cookies_and_session_as_the_form() {
    for case in [
        Case { name: "New Person", email: Some("new@37signals.com"), password: Some("secret123456"), avatar: false },
        Case { name: "New Person", email: Some("new@37signals.com"), password: Some("secret123456"), avatar: true },
        Case { name: "", email: Some("new@37signals.com"), password: Some("secret123456"), avatar: false },
        Case { name: "New Person", email: Some("new@37signals.com"), password: Some(""), avatar: false },
        // Fields the retained form accepts when absent (vectors/users_first_run.json
        // missing_email, missing_password): no email address, no password.
        Case { name: "New Person", email: None, password: Some("secret123456"), avatar: false },
        Case { name: "New Person", email: Some("new@37signals.com"), password: None, avatar: false },
        Case { name: "New Person", email: None, password: None, avatar: true },
    ] {
        let mut outcomes = Vec::new();
        for json in [false, true] {
            let app = app().await;
            let mut browser = app.anonymous();
            let (reply, location) = submit(&mut browser, case, json).await;
            assert_eq!(location, "http://campfire.test/");
            let mut session = browser_session(&app, &browser)
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            session.sort();
            // Signed in: home goes on to the administrator's two-step setup, as after the form.
            let home = browser.get("/").await.location().map(str::to_owned);
            assert_eq!(home.as_deref(), Some("http://campfire.test/two_factor_setup"));
            outcomes.push((cookie_contract(&reply), session, rows(&app).await));
        }
        let rows = &outcomes[0].2;
        assert_eq!(rows["users"].as_array().unwrap().len(), 1, "{rows}");
        assert_eq!(
            rows["attachments"].as_array().unwrap().len(),
            usize::from(case.avatar),
            "{rows}"
        );
        // Omitted stays NULL, never an empty string standing in for it.
        assert_eq!(
            rows["users"][0][2],
            case.email.map_or(Value::Null, |email| json!(email)),
            "{rows}"
        );
        assert_eq!(
            rows["users"][0][5],
            json!(i64::from(case.password.is_some_and(|password| !password.is_empty()))),
            "{rows}"
        );
        assert_eq!(outcomes[0], outcomes[1], "{}/{:?}/{:?}", case.name, case.email, case.password);
    }
}

#[tokio::test]
async fn json_first_run_refuses_malformed_submissions_before_creating_anything() {
    let app = app().await;
    let mut browser = app.anonymous();
    let token = browser.get("/api/v1/first_run").await.json()["csrfToken"]
        .as_str()
        .unwrap()
        .to_owned();
    let before = rows(&app).await;
    let invalid = |field: &str, message: &str| json!({"kind":"error","fieldErrors":{field:[message]}});
    let malformed = invalid("base", "The request body isn't valid.");
    let json = |body: &[u8]| {
        Req::new(Method::POST, "/api/v1/first_run")
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .body(body.to_vec())
    };
    for (request, answer) in [
        // Rails would commit the account and then fail on the missing name.
        (json(br#"{"emailAddress":"new@37signals.com","password":"secret123456"}"#), &malformed),
        (json(b"{not json"), &malformed),
        (json(b""), &malformed),
        (
            Req::new(Method::POST, "/api/v1/first_run")
                .header("accept", "application/json")
                .multipart(&[], ("avatar", "me.png", "image/png", AVATAR)),
            &malformed,
        ),
        (
            Req::new(Method::POST, "/api/v1/first_run")
                .header("accept", "application/json")
                .multipart(
                    &[
                        ("submission", r#"{"name":"A","emailAddress":"a@b.c","password":"x"}"#),
                        ("avatar", "not-a-signed-id"),
                    ],
                    ("unused", "u.txt", "text/plain", b"u"),
                ),
            &invalid("avatar", "The avatar isn't an uploaded picture."),
        ),
    ] {
        let reply = browser.send(request.header("x-csrf-token", &token)).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", reply.text());
        assert_eq!(&reply.json(), answer);
        assert_eq!(rows(&app).await, before);
    }
}

#[tokio::test]
async fn json_first_run_takes_the_header_or_body_token_and_refuses_forgeries() {
    let submission = json!({"name":"New Person","emailAddress":"new@37signals.com","password":"secret123456"});
    let app = app().await;
    let mut browser = app.anonymous();
    let token = browser.get("/api/v1/first_run").await.json()["csrfToken"]
        .as_str()
        .unwrap()
        .to_owned();
    let before = rows(&app).await;
    let request = json_request(Method::POST, "/api/v1/first_run", submission.clone());
    for rejected in [
        request.clone(),
        body_token(request.clone(), true, "invalid"),
        request.clone().header("x-csrf-token", "invalid"),
        body_token(request.clone(), true, &token).header("origin", "https://attacker.test"),
        body_token(request.clone(), true, &token).header("origin", "null"),
        Req::new(Method::POST, "/api/v1/first_run")
            .header("accept", "application/json")
            .multipart(
                &[("submission", &submission.to_string()), ("authenticity_token", "invalid")],
                ("avatar", "me.png", "image/png", AVATAR),
            ),
    ] {
        let reply = browser.send(rejected).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", reply.text());
        assert_eq!(rows(&app).await, before);
    }
    // The Rails body token alone, as the retained form posts it.
    let reply = browser.send(body_token(request, true, &token)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.json()["kind"], "signedIn");

    // A multipart form's own token field.
    let app = self::app().await;
    let mut browser = app.anonymous();
    let token = browser.get("/api/v1/first_run").await.json()["csrfToken"]
        .as_str()
        .unwrap()
        .to_owned();
    let reply = browser
        .send(
            Req::new(Method::POST, "/api/v1/first_run")
                .header("accept", "application/json")
                .multipart(
                    &[("submission", &submission.to_string()), ("authenticity_token", &token)],
                    ("avatar", "me.png", "image/png", AVATAR),
                ),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(rows(&app).await["attachments"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn once_set_up_every_first_run_path_goes_home_and_changes_nothing() {
    let app = app().await;
    let mut browser = app.anonymous();
    let reply = browser.get("/app/first_run").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(inline_boot(&reply)["firstRunPending"], true);
    let token = browser.get("/api/v1/first_run").await.json()["csrfToken"]
        .as_str()
        .unwrap()
        .to_owned();
    app.db()
        .write(|tx| Account::create(tx, "Chat"))
        .await
        .unwrap();
    let before = rows(&app).await;
    let home = Some("http://campfire.test/");
    assert_eq!(browser.get("/first_run").await.location(), home);
    assert_eq!(browser.get("/app/first_run").await.location(), home);
    let state = browser.get("/api/v1/first_run").await;
    assert_eq!(state.status, StatusCode::OK);
    assert_eq!(state.json(), json!({"kind":"navigate","location":"http://campfire.test/"}));
    let reply = browser
        .send(
            json_request(
                Method::POST,
                "/api/v1/first_run",
                json!({"name":"must not create","emailAddress":"x@example.com","password":"secret123456"}),
            )
            .header("x-csrf-token", &token),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json(), json!({"kind":"navigate","location":"http://campfire.test/"}));
    assert_eq!(rows(&app).await, before);

    // Signed in on a set-up workspace, the SPA page redirects exactly as the retained one.
    let app = TestApp::boot_frozen().await.expect("restored default seed");
    let mut browser = app.sign_in(DAVID).await;
    let html = browser.get("/first_run").await;
    let spa = browser.get("/app/first_run").await;
    assert_eq!((spa.status, spa.location()), (html.status, html.location()));
}
