//! The retained sign-in, two-step, sudo, join, first-run and public page URLs after the cutover:
//! each sends the browser to its SPA page, keeping the query and any waiting flash, and a write
//! to one changes nothing.
use crate::controllers::presenters::test_support::{Browser, DAVID, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{Account, Session, User};
use serde_json::{Value, json};

use super::signed_out_contracts_tests::inline_boot;

async fn app() -> TestApp {
    TestApp::boot_frozen().await.expect("restored default seed")
}

async fn join_code(app: &TestApp) -> String {
    app.db()
        .read(|conn| Ok(Account::first(conn)?.expect("seeded account").join_code))
        .await
        .unwrap()
}

/// GET and HEAD of `path?query` both redirect with `status` to `target?query`.
async fn assert_redirect(browser: &mut Browser<'_>, path: &str, status: StatusCode, target: &str) {
    let query = "utm_source=mail&note=a%20b";
    for method in [Method::GET, Method::HEAD] {
        let reply = browser.send(Req::new(method.clone(), &format!("{path}?{query}"))).await;
        assert_eq!(
            (reply.status, reply.location()),
            (status, Some(format!("http://campfire.test{target}?{query}").as_str())),
            "{method} {path}: {}",
            reply.text()
        );
    }
}

#[tokio::test]
async fn retained_pages_redirect_to_their_spa_pages_keeping_the_query() {
    let app = app().await;
    let code = join_code(&app).await;
    let mut visitor = app.anonymous();
    for (path, target) in [
        ("/session/new".to_string(), "/app/session/new".to_string()),
        ("/session/transfers/some-id".into(), "/app/session/transfers/some-id".into()),
        (format!("/join/{code}"), format!("/app/join/{code}")),
        ("/join/wrong-code".into(), "/app/join/wrong-code".into()),
        ("/invite/some-token".into(), "/app/invite/some-token".into()),
    ] {
        assert_redirect(&mut visitor, &path, StatusCode::FOUND, &target).await;
    }
    for page in ["about", "privacy", "terms"] {
        assert_redirect(&mut visitor, &format!("/{page}"), StatusCode::MOVED_PERMANENTLY, &format!("/app/{page}"))
            .await;
    }

    let mut david = app.sign_in(DAVID).await;
    for (path, target) in [
        ("/sudo/new", "/app/sudo/new"),
        ("/two_factor_challenge", "/app/two_factor/challenge"),
        ("/about", "/app/about"),
    ] {
        let status = if path == "/about" { StatusCode::MOVED_PERMANENTLY } else { StatusCode::FOUND };
        assert_redirect(&mut david, path, status, target).await;
    }
    // (`/two_factor_setup` is the enrollment gate's test: an enrolled member goes to the profile.)
    // Signed in, the join page still sends a member home first.
    assert_eq!(david.get(&format!("/join/{code}")).await.location(), Some("http://campfire.test/"));
}

#[tokio::test]
async fn retained_forms_redirect_with_see_other_and_change_nothing() {
    let app = app().await;
    let code = join_code(&app).await;
    let facts = || async {
        app.db()
            .read(|conn| {
                let count = |sql: &str| conn.query_row(sql, [], |row| row.get::<_, i64>(0));
                Ok(json!({
                    "sessions": Session::for_user(conn, DAVID)?.len(),
                    "users": count("SELECT COUNT(*) FROM users")?,
                    "credentials": count("SELECT COUNT(*) FROM two_factor_credentials")?,
                    "backup_codes": conn.query_row(
                        "SELECT group_concat(id) FROM (SELECT id FROM two_factor_backup_codes ORDER BY id)",
                        [],
                        |row| row.get::<_, Option<String>>(0),
                    )?,
                    "audits": count("SELECT COUNT(*) FROM audit_logs")?,
                }))
            })
            .await
            .unwrap()
    };
    let mut david = app.sign_in(DAVID).await;
    let before = facts().await;

    // A stale form posts without a live authenticity token; it still lands on the SPA page.
    let mut visitor = app.anonymous();
    for (method, path, form, target) in [
        (Method::POST, "/session", vec![("email_address", "david@37signals.com"), ("password", "secret123456")], "/app/session/new"),
        (Method::POST, "/two_factor_challenge", vec![("code", "123456")], "/app/two_factor/challenge"),
        (Method::PUT, "/session/transfers/some-id", vec![], "/app/session/transfers/some-id"),
        (Method::POST, "/first_run", vec![("user[name]", "Someone")], "/app/first_run"),
        (
            Method::POST,
            &format!("/join/{code}"),
            vec![("user[name]", "New"), ("user[email_address]", "new@example.com"), ("user[password]", "secret123456")],
            &format!("/app/join/{code}"),
        ),
    ] {
        let reply = visitor.send(Req::new(method.clone(), path).form(&form)).await;
        assert_eq!(
            (reply.status, reply.location()),
            (StatusCode::SEE_OTHER, Some(format!("http://campfire.test{target}").as_str())),
            "{method} {path}: {}",
            reply.text()
        );
        assert!(reply.header("set-cookie").is_none_or(|cookie| !cookie.contains("session_token")), "{path}");
    }

    for (method, path, form, target) in [
        (Method::POST, "/sudo", vec![("password", "secret123456")], "/app/sudo/new"),
        (Method::POST, "/sudo/google", vec![], "/app/sudo/new"),
        (Method::POST, "/two_factor_setup", vec![("code", "123456")], "/app/settings/security"),
        (Method::DELETE, "/two_factor_setup", vec![("password", "secret123456")], "/app/settings/security"),
        (Method::POST, "/two_factor_backup_codes", vec![("password", "secret123456")], "/app/settings/security"),
        (Method::DELETE, "/two_factor_remembered_devices", vec![], "/app/settings/security"),
    ] {
        let reply = david.send(Req::new(method.clone(), path).form(&form)).await;
        assert_eq!(
            (reply.status, reply.location()),
            (StatusCode::SEE_OTHER, Some(format!("http://campfire.test{target}").as_str())),
            "{method} {path}: {}",
            reply.text()
        );
    }
    // The password went nowhere: sudo is still unconfirmed.
    assert_eq!(
        david.get("/account/audit_log.csv").await.location(),
        Some("http://campfire.test/sudo/new")
    );
    assert_eq!(facts().await, before);
}

#[tokio::test]
async fn a_waiting_flash_survives_the_redirect_to_the_spa_page() {
    use crate::integrations::google::sign_in::{Config, SignIn};
    let app = app().await;
    app.booted.app.google.install(SignIn::new(
        Config {
            client_id: "fixture-client".into(),
            client_secret: "fixture-secret".into(),
            domains: vec!["smartdata.net".into()],
        },
        crate::net::Network::system(),
    ));
    let alert = json!({
        "kind": "alert",
        "message": "Google sign-in expired. Try again or sign in with email and password."
    });
    let mut browser = app.anonymous();
    for retained in ["/session/new?from=bookmark", "/about"] {
        // A stale Google callback leaves an alert for the next page load.
        browser.get("/session/google/callback?state=stale&code=fixture").await;
        let reply = browser.get(retained).await;
        assert!(reply.status.is_redirection(), "{retained}: {}", reply.status);
        let page = browser.get("/app/session/new").await;
        assert_eq!(inline_boot(&page)["flash"], alert, "{retained}");
        assert_eq!(inline_boot(&browser.get("/app/session/new").await)["flash"], Value::Null, "{retained}");
    }
}

#[tokio::test]
async fn the_enrollment_gate_sends_pages_to_the_spa_setup_without_a_loop() {
    let app = app().await;
    app.db().write(|tx| User::find(tx.conn(), DAVID)?.reset_two_factor(tx)).await.unwrap();
    let mut browser = app.anonymous();
    let signed_in = browser.password_sign_in("david@37signals.com", "secret123456").await;
    assert_eq!(signed_in.status, StatusCode::OK, "{}", signed_in.text());

    let setup = "http://campfire.test/app/two_factor/setup?required=true";
    for path in ["/", "/app/", "/users/me/profile"] {
        assert_eq!(browser.get(path).await.location(), Some(setup), "{path}");
    }
    // The page the gate names is exempt from it, so following the gate ends there.
    let mut path = "/".to_string();
    for _ in 0..3 {
        let reply = browser.get(&path).await;
        match reply.location() {
            Some(location) => path = location.strip_prefix("http://campfire.test").unwrap().to_string(),
            None => {
                assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.text());
                break;
            }
        }
    }
    assert_eq!(path, "/app/two_factor/setup?required=true");
    for method in [Method::GET, Method::HEAD] {
        let page = browser.send(Req::new(method.clone(), "/app/two_factor/setup")).await;
        assert_eq!(page.status, StatusCode::OK, "{method}");
    }
    // The retained URL goes to the same page, and the page's own read stays open.
    assert_eq!(
        browser.get("/two_factor_setup?required=true").await.location(),
        Some(setup)
    );
    let state = browser
        .send(Req::new(Method::GET, "/api/v1/two_factor/setup").header("accept", "application/json"))
        .await;
    assert_eq!(state.status, StatusCode::OK, "{}", state.text());
}

/// A JSON request for `path`.
fn json_get(path: &str) -> Req {
    Req::new(Method::GET, path).header("accept", "application/json")
}

#[tokio::test]
async fn gates_send_only_page_navigations_to_the_spa_pages() {
    let app = app().await;
    // Signed out, a page goes to the SPA's sign-in. A JSON request goes to the retained route,
    // which still answers it with a 401, never the SPA's HTML.
    let mut visitor = app.anonymous();
    assert_eq!(visitor.get("/app/").await.location(), Some("http://campfire.test/app/session/new"));
    assert_eq!(
        visitor.send(json_get("/app/")).await.location(),
        Some("http://campfire.test/session/new")
    );
    assert_eq!(visitor.send(json_get("/session/new")).await.status, StatusCode::UNAUTHORIZED);

    // Waiting on a second factor: the SPA's challenge for a page, the retained route (which
    // answers only HTML) for anything else.
    let mut pending = app.anonymous();
    let signed_in = pending.password_sign_in("david@37signals.com", "secret123456").await;
    assert_eq!(signed_in.json()["kind"], "secondFactorRequired", "{}", signed_in.text());
    assert_eq!(
        pending.get("/app/").await.location(),
        Some("http://campfire.test/app/two_factor/challenge")
    );
    assert_eq!(
        pending.send(json_get("/app/")).await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
    let retained = pending.send(json_get("/two_factor_challenge")).await;
    assert_eq!(retained.status, StatusCode::NOT_ACCEPTABLE, "{}", retained.text());

    // The sudo gate: a page or form goes to the SPA's confirmation, a download (CSV) to the
    // retained route, which a browser follows on to the SPA's page and anything else is refused.
    let mut david = app.sign_in(DAVID).await;
    assert_eq!(
        david.write(Req::new(Method::POST, "/account/join_code")).await.location(),
        Some("http://campfire.test/app/sudo/new")
    );
    assert_eq!(
        david.get("/account/audit_log.csv").await.location(),
        Some("http://campfire.test/sudo/new")
    );
    assert_eq!(david.get("/sudo/new").await.location(), Some("http://campfire.test/app/sudo/new"));
    let csv = david.send(Req::new(Method::GET, "/sudo/new").header("accept", "text/csv")).await;
    assert_eq!(csv.status, StatusCode::NOT_ACCEPTABLE, "{}", csv.text());
}

#[tokio::test]
async fn before_first_run_only_a_page_goes_to_the_spa_setup() {
    let app = TestApp::boot_seed("first_run").await.expect("first-run seed");
    let mut visitor = app.anonymous();
    assert_eq!(
        visitor.get("/session/new").await.location(),
        Some("http://campfire.test/app/first_run")
    );
    assert_eq!(
        visitor.send(json_get("/session/new")).await.location(),
        Some("http://campfire.test/first_run")
    );
    let retained = visitor.send(json_get("/first_run")).await;
    assert_eq!(retained.status, StatusCode::NOT_ACCEPTABLE, "{}", retained.text());
}
