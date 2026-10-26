use crate::controllers::presenters::test_support::*;
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
async fn first_run_is_available_until_account_exists() {
    let app = app().await;
    // The retained page now sends the visitor to the SPA's setup page, until the account exists.
    let mut browser = app.anonymous();
    let page = browser.get("/first_run").await;
    assert_eq!(
        (page.status, page.location()),
        (
            StatusCode::FOUND,
            Some("http://campfire.test/app/first_run")
        )
    );
    app.db()
        .write(|tx| Account::create(tx, "Chat"))
        .await
        .unwrap();
    assert_eq!(
        app.anonymous().get("/first_run").await.location(),
        Some("http://campfire.test/")
    );
    // The retained form goes, unread, to the SPA's page; it creates nothing.
    let reply = browser
        .write(Req::new(Method::POST, "/first_run").form(&[("user[name]", "must not create")]))
        .await;
    assert_eq!(
        (reply.status, reply.location()),
        (
            StatusCode::SEE_OTHER,
            Some("http://campfire.test/app/first_run")
        )
    );
    assert_eq!(
        app.db()
            .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i64>(0))?))
            .await
            .unwrap(),
        0
    );
}
/// What first run leaves behind, in the shape of vectors/users_first_run.json's `state`.
async fn first_run_state(app: &TestApp) -> Value {
    app.db().read(|c|{
        let accounts=c.prepare("SELECT name FROM accounts ORDER BY id")?.query_map([],|r|r.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()?;
        let users=User::all(c)?.iter().map(|u|json!({"name":u.name,"email":u.email_address,"role":u.role.name(),"password_present":u.password_digest.as_deref().is_some_and(|s|!s.is_empty())})).collect::<Vec<_>>();
        let rooms=c.prepare("SELECT name,type FROM rooms ORDER BY id")?.query_map([],|r|Ok(json!([r.get::<_,String>(0)?,r.get::<_,String>(1)?])))?.collect::<Result<Vec<_>,_>>()?;
        let count=|table:&str|c.query_row(&format!("SELECT COUNT(*) FROM {table}"),[],|r|r.get::<_,i64>(0));
        Ok(json!({"account_names":accounts,"users":users,"rooms":rooms,"memberships":count("memberships")?,"sessions":count("sessions")?,"audits":count("audit_logs")?}))
    }).await.unwrap()
}

/// Each pinned Rails form case: the retained form now creates nothing and goes to the SPA's page,
/// and the same fields through the SPA's JSON contract leave Rails' state.
#[tokio::test]
async fn first_run_http_and_persisted_state_match_all_rails_cases() {
    for case in vectors()["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
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
        let empty = first_run_state(&app).await;
        let response = browser
            .write(Req::new(Method::POST, "/first_run").form(&fields))
            .await;
        assert_eq!(
            (response.status, response.location()),
            (
                StatusCode::SEE_OTHER,
                Some("http://campfire.test/app/first_run")
            ),
            "{name}"
        );
        assert_eq!(first_run_state(&app).await, empty, "{name}");

        // The same fields as the SPA submits them; `account` and `user[role]` stay ignored.
        let token = browser.get("/api/v1/first_run").await.json()["csrfToken"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut submission = case["params"]["user"].clone();
        if let Some(email) = submission
            .as_object_mut()
            .and_then(|user| user.remove("email_address"))
        {
            submission["emailAddress"] = email;
        }
        submission["account"] = case["params"]["account"].clone();
        let reply = browser
            .send(
                json_request(Method::POST, "/api/v1/first_run", submission)
                    .header("x-csrf-token", &token),
            )
            .await;
        if case["status"] == 302 {
            assert_eq!(reply.status, StatusCode::OK, "{name}: {}", reply.text());
            assert_eq!(
                reply.json(),
                json!({"kind":"signedIn","location":case["location"]}),
                "{name}"
            );
            assert_eq!(first_run_state(&app).await, case["state"], "{name}");
            assert!(browser.cookie_header().contains("session_token="));
            assert_eq!(
                browser.get("/").await.location(),
                Some("http://campfire.test/app/two_factor/setup?required=true")
            );
        } else {
            // Rails failed these (400 without `user`, 500 without a name, after committing the
            // account); the contract refuses them before creating anything.
            assert_eq!(
                reply.status,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{name}: {}",
                reply.text()
            );
            assert_eq!(first_run_state(&app).await, empty, "{name}");
        }
    }
}
#[tokio::test]
async fn five_concurrent_first_runs_create_one_administrator_and_room() {
    let app = app().await;
    let mut browsers = Vec::new();
    for _ in 0..5 {
        let mut browser = app.anonymous();
        let token = browser.get("/api/v1/first_run").await.json()["csrfToken"]
            .as_str()
            .unwrap()
            .to_owned();
        browsers.push((browser, token));
    }
    let responses = futures_util::future::join_all(browsers.into_iter().enumerate().map(
        |(i, (mut browser, token))| async move {
            let submission = json!({
                "name": format!("Attacker{i}"),
                "emailAddress": format!("attacker{i}@example.com"),
                "password": "password123",
            });
            browser
                .send(
                    json_request(Method::POST, "/api/v1/first_run", submission)
                        .header("x-csrf-token", &token),
                )
                .await
        },
    ))
    .await;
    // One sets up the workspace; the others are sent home, as the retained form's were.
    let answers = responses
        .iter()
        .map(|r| {
            assert_eq!(r.status, StatusCode::OK, "{}", r.text());
            r.json()
        })
        .collect::<Vec<_>>();
    assert!(
        answers
            .iter()
            .all(|a| a["location"] == "http://campfire.test/"),
        "{answers:?}"
    );
    assert_eq!(
        answers.iter().filter(|a| a["kind"] == "signedIn").count(),
        1,
        "{answers:?}"
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

/// Submits `case` as the retained form did: it goes, unread, to the SPA's setup page.
async fn submit_form(browser: &mut Browser<'_>, case: Case) -> Reply {
    let page = browser.get("/first_run").await;
    assert_eq!(page.location(), Some("http://campfire.test/app/first_run"));
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
}

/// Submits `case` as the SPA does; answers where it led.
async fn submit(browser: &mut Browser<'_>, case: Case) -> (Reply, String) {
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
    let reply = browser.send(request.header("x-csrf-token", &token)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let answer = reply.json();
    assert_eq!(answer["kind"], "signedIn", "{answer}");
    let location = answer["location"].as_str().unwrap().to_owned();
    (reply, location)
}

/// The retained form now creates nothing; the SPA's JSON contract sets up the workspace with the
/// rows the form wrote, and the same cookies and session whatever the submission.
#[tokio::test]
async fn json_first_run_creates_the_rows_cookies_and_session_the_form_did() {
    let mut contracts = Vec::new();
    for case in [
        Case {
            name: "New Person",
            email: Some("new@37signals.com"),
            password: Some("secret123456"),
            avatar: false,
        },
        Case {
            name: "New Person",
            email: Some("new@37signals.com"),
            password: Some("secret123456"),
            avatar: true,
        },
        Case {
            name: "",
            email: Some("new@37signals.com"),
            password: Some("secret123456"),
            avatar: false,
        },
        Case {
            name: "New Person",
            email: Some("new@37signals.com"),
            password: Some(""),
            avatar: false,
        },
        // Fields the retained form accepted when absent (vectors/users_first_run.json
        // missing_email, missing_password): no email address, no password.
        Case {
            name: "New Person",
            email: None,
            password: Some("secret123456"),
            avatar: false,
        },
        Case {
            name: "New Person",
            email: Some("new@37signals.com"),
            password: None,
            avatar: false,
        },
        Case {
            name: "New Person",
            email: None,
            password: None,
            avatar: true,
        },
    ] {
        let label = format!(
            "{}/{:?}/{:?}/{}",
            case.name, case.email, case.password, case.avatar
        );
        let app = app().await;
        let mut browser = app.anonymous();
        let before = rows(&app).await;
        let form = submit_form(&mut browser, case).await;
        assert_eq!(
            (form.status, form.location()),
            (
                StatusCode::SEE_OTHER,
                Some("http://campfire.test/app/first_run")
            ),
            "{label}"
        );
        assert_eq!(rows(&app).await, before, "{label}");
        assert!(
            !browser.cookie_header().contains("session_token="),
            "{label}"
        );

        let (reply, location) = submit(&mut browser, case).await;
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
        assert_eq!(
            home.as_deref(),
            Some("http://campfire.test/app/two_factor/setup?required=true")
        );
        let cookies = cookie_contract(&reply);
        assert!(
            cookies
                .iter()
                .any(|cookie| cookie.starts_with("session_token=set")),
            "{cookies:?}"
        );
        contracts.push((cookies, session));

        let rows = rows(&app).await;
        assert_eq!(rows["accounts"], json!([["Smartfire"]]), "{rows}");
        assert_eq!(rows["users"].as_array().unwrap().len(), 1, "{rows}");
        assert_eq!(rows["users"][0][1], json!(case.name), "{rows}");
        assert_eq!(rows["rooms"].as_array().unwrap().len(), 1, "{rows}");
        assert_eq!(rows["memberships"].as_array().unwrap().len(), 1, "{rows}");
        assert_eq!(rows["sessions"].as_array().unwrap().len(), 1, "{rows}");
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
            json!(i64::from(
                case.password.is_some_and(|password| !password.is_empty())
            )),
            "{rows}"
        );
    }
    assert!(
        contracts.windows(2).all(|pair| pair[0] == pair[1]),
        "{contracts:?}"
    );
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
    let invalid =
        |field: &str, message: &str| json!({"kind":"error","fieldErrors":{field:[message]}});
    let malformed = invalid("base", "The request body isn't valid.");
    let json = |body: &[u8]| {
        Req::new(Method::POST, "/api/v1/first_run")
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .body(body.to_vec())
    };
    for (request, answer) in [
        // Rails would commit the account and then fail on the missing name.
        (
            json(br#"{"emailAddress":"new@37signals.com","password":"secret123456"}"#),
            &malformed,
        ),
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
                        (
                            "submission",
                            r#"{"name":"A","emailAddress":"a@b.c","password":"x"}"#,
                        ),
                        ("avatar", "not-a-signed-id"),
                    ],
                    ("unused", "u.txt", "text/plain", b"u"),
                ),
            &invalid("avatar", "The avatar isn't an uploaded picture."),
        ),
    ] {
        let reply = browser.send(request.header("x-csrf-token", &token)).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert_eq!(&reply.json(), answer);
        assert_eq!(rows(&app).await, before);
    }
}

#[tokio::test]
async fn json_first_run_takes_the_header_or_body_token_and_refuses_forgeries() {
    let submission =
        json!({"name":"New Person","emailAddress":"new@37signals.com","password":"secret123456"});
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
                &[
                    ("submission", &submission.to_string()),
                    ("authenticity_token", "invalid"),
                ],
                ("avatar", "me.png", "image/png", AVATAR),
            ),
    ] {
        let reply = browser.send(rejected).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
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
                    &[
                        ("submission", &submission.to_string()),
                        ("authenticity_token", &token),
                    ],
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
    assert_eq!(
        state.json(),
        json!({"kind":"navigate","location":"http://campfire.test/"})
    );
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
    assert_eq!(
        reply.json(),
        json!({"kind":"navigate","location":"http://campfire.test/"})
    );
    assert_eq!(rows(&app).await, before);

    // Signed in on a set-up workspace, the SPA page redirects exactly as the retained one.
    let app = TestApp::boot_frozen().await.expect("restored default seed");
    let mut browser = app.sign_in(DAVID).await;
    let html = browser.get("/first_run").await;
    let spa = browser.get("/app/first_run").await;
    assert_eq!((spa.status, spa.location()), (html.status, html.location()));
}
