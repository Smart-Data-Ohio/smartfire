//! "Forgot password?" over the frozen seed: availability, the uniform request, delivery, and the
//! one-time link that sets a new password and signs the user out everywhere.
use crate::controllers::presenters::test_support::{
    BENDER, DAVID, JASON, KEVIN, Reply, Req, SEED_NOW, TestApp,
};
use crate::controllers::spa::signed_out_contracts_tests::inline_boot;
use axum::http::{Method, StatusCode};
use campfire_db::TwoFactorRememberedDevice;
use serde_json::{Value, json};
use std::sync::Arc;

const MAIL: [(&str, &str); 3] = [
    ("SMTP_ADDRESS", "smtp.invalid"),
    ("MAILER_FROM", "Smartfire <alerts@example.com>"),
    ("APP_URL", "http://campfire.test"),
];
const PASSWORD: &str = "secret123456";
const NEW_PASSWORD: &str = "a brand new passphrase";
const INVALID_LINK: &str = "This reset link didn't work. It may have expired or already been used.";

async fn app(env: &[(&str, &str)]) -> (TestApp, Arc<campfire_kit::FrozenClock>) {
    let clock = Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_clock_and_env(clock.clone(), env)
        .await
        .expect("restored default seed");
    (app, clock)
}

async fn email_of(a: &TestApp, user_id: i64) -> String {
    a.db()
        .read(move |conn| Ok(campfire_db::User::find(conn, user_id)?.email_address.unwrap()))
        .await
        .unwrap()
}

/// David can reset; the rest are the accounts that can't, each with an address of its own.
async fn accounts(a: &TestApp) -> (String, Vec<String>) {
    a.db()
        .write(|tx| {
            let conn = tx.conn();
            // Google-only: no password.
            conn.execute("UPDATE users SET password_digest=NULL WHERE id=?", [KEVIN])?;
            conn.execute(
                "UPDATE users SET email_address='bender@example.com' WHERE id=?",
                [BENDER],
            )?;
            conn.execute("UPDATE users SET status=1 WHERE id=?", [JASON])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut refused = vec!["nobody@example.com".to_string()];
    for id in [KEVIN, BENDER, JASON] {
        refused.push(email_of(a, id).await);
    }
    (email_of(a, DAVID).await, refused)
}

fn request(email: &str, ip: &str) -> Req {
    Req::new(Method::POST, "/api/v1/password_resets")
        .header("accept", "application/json")
        .header("x-forwarded-for", ip)
        .json_body(&json!({ "emailAddress": email }))
}

fn submit(token: &str, password: &str) -> Req {
    Req::new(Method::PUT, &format!("/api/v1/password_resets/{token}"))
        .header("accept", "application/json")
        .header("x-forwarded-for", "198.51.100.200")
        .json_body(&json!({ "password": password }))
}

fn check(token: &str) -> Req {
    Req::new(Method::GET, &format!("/api/v1/password_resets/{token}"))
        .header("accept", "application/json")
}

/// Records the arguments of every mail delivery job as it's queued.
async fn capture_mail_jobs(a: &TestApp) {
    a.db()
        .write(|tx| {
            tx.conn().execute_batch(
                "CREATE TABLE reset_mail(args TEXT NOT NULL);
                 CREATE TRIGGER capture_reset_mail BEFORE INSERT ON background_jobs
                 WHEN NEW.job_class='Smartfire::MailDeliveryJob'
                 BEGIN INSERT INTO reset_mail VALUES(NEW.arguments); END;",
            )?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn captured_mail_jobs(a: &TestApp) -> Vec<Value> {
    a.db()
        .read(|conn| {
            let mut stmt = conn.prepare("SELECT args FROM reset_mail ORDER BY rowid")?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows
                .iter()
                .map(|args| serde_json::from_str(args).unwrap())
                .collect())
        })
        .await
        .unwrap()
}

/// The reset email for `email`, as delivery builds it for a request made at `requested_at`.
async fn reset_message(
    a: &TestApp,
    email: &str,
    requested_at: jiff::Timestamp,
) -> Option<campfire_mail::outbound::Message> {
    campfire_runtime::mail::password_reset_message(
        &a.booted.app,
        email.into(),
        campfire_db::Timestamp::from_jiff(requested_at),
    )
    .await
    .unwrap()
}

/// The link the reset email for `email` carries, as delivery builds it now for a request made now.
async fn emailed_token(a: &TestApp, email: &str) -> String {
    let message = reset_message(a, email, a.booted.app.clock.now())
        .await
        .expect("a reset email");
    link_token(&message)
}

fn link_token(message: &campfire_mail::outbound::Message) -> String {
    let url = message
        .text
        .lines()
        .find(|line| line.starts_with("http"))
        .unwrap();
    url.strip_prefix("http://campfire.test/app/passwords/")
        .expect("the SPA's reset page under APP_URL")
        .to_string()
}

async fn count(a: &TestApp, sql: &'static str, id: i64) -> i64 {
    a.db()
        .read(move |conn| Ok(conn.query_row(sql, [id], |r| r.get(0))?))
        .await
        .unwrap()
}

fn refused_link(reply: &Reply) {
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.text());
    assert_eq!(
        reply.json(),
        json!({"kind":"error","fieldErrors":{"token":[INVALID_LINK]}})
    );
}

#[tokio::test]
async fn reset_is_offered_only_when_mail_can_send_a_working_link() {
    for (env, available) in [
        (&MAIL[..], true),
        (&MAIL[..2], false),
        (&[MAIL[0], MAIL[2]][..], false),
        (&[][..], false),
    ] {
        let (a, _) = app(env).await;
        let mut b = a.anonymous();
        let page = b.get("/app/session/new").await;
        assert_eq!(
            inline_boot(&page)["signInMethods"]["passwordReset"],
            available,
            "{env:?}"
        );
        for path in ["/app/passwords/new", "/app/passwords/some-token"] {
            let page = b.get(path).await;
            assert_eq!(page.status, StatusCode::OK, "{path}");
            assert_eq!(inline_boot(&page)["kind"], "signedOut", "{path}");
        }
        if !available {
            let reply = b.write(request("david@37signals.com", "198.51.100.1")).await;
            assert_eq!(reply.status, StatusCode::NOT_FOUND);
            assert_eq!(reply.json(), json!({"kind":"error","fieldErrors":{}}));
            assert_eq!(b.send(check("token")).await.status, StatusCode::NOT_FOUND);
            assert_eq!(
                b.write(submit("token", NEW_PASSWORD)).await.status,
                StatusCode::NOT_FOUND
            );
        }
    }
}

#[tokio::test]
async fn every_request_answers_alike_and_queues_one_job() {
    let (a, _) = app(&MAIL).await;
    let (david, refused) = accounts(&a).await;
    capture_mail_jobs(&a).await;
    let mut b = a.anonymous();
    let mut answers = Vec::new();
    let emails = std::iter::once(david.clone())
        .chain(refused.iter().cloned())
        .collect::<Vec<_>>();
    for (n, email) in emails.iter().enumerate() {
        let reply = b.write(request(email, &format!("198.51.100.{n}"))).await;
        answers.push((
            reply.status,
            reply.text(),
            reply.header("content-type").map(str::to_owned),
            reply.header("cache-control").map(str::to_owned),
        ));
    }
    assert_eq!(answers[0].0, StatusCode::OK);
    assert_eq!(answers[0].1, r#"{"kind":"requested"}"#);
    assert!(answers.iter().all(|answer| *answer == answers[0]), "{answers:?}");
    assert_eq!(
        captured_mail_jobs(&a).await,
        emails
            .iter()
            .map(|email| json!({"notification":{"PasswordReset":{"email_address": email}}}))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn delivery_writes_only_to_accounts_that_can_reset() {
    let (a, _) = app(&MAIL).await;
    let (david, refused) = accounts(&a).await;
    let now = a.booted.app.clock.now();
    let message = reset_message(&a, &david, now)
        .await
        .expect("David can reset");
    assert!(message.to.ends_with(&format!("<{david}>")), "{}", message.to);
    assert_eq!(message.from, "Smartfire <alerts@example.com>");
    assert!(
        message
            .text
            .contains("http://campfire.test/app/passwords/"),
        "{}",
        message.text
    );
    for email in refused {
        assert!(reset_message(&a, &email, now).await.is_none(), "{email}");
    }
    let (unconfigured, _) = app(&MAIL[..2]).await;
    assert!(reset_message(&unconfigured, &david, now).await.is_none());
}

#[tokio::test]
async fn a_link_sets_the_password_once_and_signs_out_everywhere() {
    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    a.db()
        .write(|tx| {
            TwoFactorRememberedDevice::create_for(tx, DAVID, Some("Laptop"), None)?;
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut signed_in = a.sign_in(DAVID).await;
    assert_eq!(signed_in.get("/api/v1/boot").await.status, StatusCode::OK);
    assert!(count(&a, "SELECT count(*) FROM sessions WHERE user_id=?", DAVID).await > 0);
    let others = count(&a, "SELECT count(*) FROM sessions WHERE user_id<>?", DAVID).await;

    let token = emailed_token(&a, &david).await;
    let mut b = a.anonymous();
    let ready = b.send(check(&token)).await;
    assert_eq!(ready.status, StatusCode::OK);
    assert_eq!(ready.json(), json!({"kind":"ready"}));

    let reply = b.write(submit(&token, NEW_PASSWORD)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(
        reply.json(),
        json!({"kind":"navigate","location":"http://campfire.test/app/session/new"})
    );
    // The sign-in page it lands on says so.
    let page = b.get("/app/session/new").await;
    assert_eq!(
        inline_boot(&page)["flash"],
        json!({"kind":"notice","message":"Password updated. Sign in with your new password."})
    );

    assert_eq!(
        count(&a, "SELECT count(*) FROM sessions WHERE user_id=?", DAVID).await,
        0
    );
    assert_eq!(
        count(&a, "SELECT count(*) FROM sessions WHERE user_id<>?", DAVID).await,
        others
    );
    assert_eq!(
        count(
            &a,
            "SELECT count(*) FROM two_factor_remembered_devices WHERE user_id=?",
            DAVID
        )
        .await,
        0
    );
    let audits = a
        .db()
        .read(|conn| {
            let mut stmt = conn.prepare(
                "SELECT action,actor_id,target_id,target_type,ip_address FROM audit_logs ORDER BY id",
            )?;
            let rows = stmt
                .query_map([], |r| {
                    Ok(json!([
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<i64>>(1)?,
                        r.get::<_, Option<i64>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, Option<String>>(4)?
                    ]))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
        .await
        .unwrap();
    assert_eq!(
        audits,
        [json!(["user.password.reset", DAVID, DAVID, "User", "198.51.100.200"])]
    );
    assert_eq!(
        signed_in.get("/api/v1/boot").await.location(),
        Some("http://campfire.test/app/session/new")
    );

    // Used once: the password it was made for is gone.
    refused_link(&b.send(check(&token)).await);
    refused_link(&b.write(submit(&token, "yet another passphrase")).await);

    let old = a.anonymous().password_sign_in(&david, PASSWORD).await;
    assert_eq!(old.status, StatusCode::UNAUTHORIZED);
    let new = a.anonymous().password_sign_in(&david, NEW_PASSWORD).await;
    assert_eq!(new.status, StatusCode::OK, "{}", new.text());
    assert!(
        ["signedIn", "secondFactorRequired"].contains(&new.json()["kind"].as_str().unwrap()),
        "{}",
        new.text()
    );
}

#[tokio::test]
async fn expired_retargeted_and_foreign_links_are_refused() {
    let (a, clock) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let digest = || async {
        a.db()
            .read(|conn| Ok(campfire_db::User::find(conn, DAVID)?.password_digest))
            .await
            .unwrap()
    };
    let before = digest().await;
    let mut b = a.anonymous();

    let token = emailed_token(&a, &david).await;
    clock.advance(rails_compat::password_reset::EXPIRES_IN);
    refused_link(&b.send(check(&token)).await);
    refused_link(&b.write(submit(&token, NEW_PASSWORD)).await);

    let token = emailed_token(&a, &david).await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET email_address='david@example.org' WHERE id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    refused_link(&b.write(submit(&token, NEW_PASSWORD)).await);

    let transfer = rails_compat::signed_id::transfer_id(&a.booted.app.secrets, DAVID, a.booted.app.clock.now());
    refused_link(&b.write(submit(&transfer, NEW_PASSWORD)).await);
    refused_link(&b.write(submit("forged", NEW_PASSWORD)).await);
    assert_eq!(digest().await, before);
    let failures = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT count(*) FROM audit_logs WHERE action='session.sign_in.failure' AND details LIKE '%password_reset%'",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert!(failures >= 1, "refusals are recorded as failed sign-ins");
}

#[tokio::test]
async fn a_new_password_must_be_present_and_fit() {
    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let token = emailed_token(&a, &david).await;
    let mut b = a.anonymous();
    for (password, message) in [
        ("", "Enter a new password."),
        ("   ", "Enter a new password."),
        (
            &*"x".repeat(73),
            "That password is too long. Choose a shorter one.",
        ),
    ] {
        let reply = b.write(submit(&token, password)).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            reply.json(),
            json!({"kind":"error","fieldErrors":{"password":[message]}})
        );
    }
    // Refusing the password leaves the link working.
    assert_eq!(b.send(check(&token)).await.status, StatusCode::OK);
}

#[tokio::test]
async fn requests_and_new_passwords_are_rate_limited() {
    let (a, _) = app(&MAIL).await;
    let mut b = a.anonymous();
    let too_many = json!({"kind":"error","fieldErrors":{"base":["Too many attempts. Try again in a few minutes."]}});

    // Five per address in 15 minutes.
    for n in 0..5 {
        let reply = b
            .write(request(&format!("person{n}@example.com"), "198.51.100.7"))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{n}");
    }
    let reply = b.write(request("person5@example.com", "198.51.100.7")).await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(reply.json(), too_many);

    // Three per inbox from anywhere, however it's written.
    for (n, email) in ["inbox@example.com", "Inbox@Example.com", " inbox@example.com"]
        .iter()
        .enumerate()
    {
        let reply = b.write(request(email, &format!("198.18.0.{n}"))).await;
        assert_eq!(reply.status, StatusCode::OK, "{email}");
    }
    let reply = b.write(request("INBOX@example.com", "198.18.0.9")).await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(reply.json(), too_many);

    // Ten new-password attempts per address.
    for _ in 0..10 {
        refused_link(&b.write(submit("forged", NEW_PASSWORD)).await);
    }
    let reply = b.write(submit("forged", NEW_PASSWORD)).await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(reply.json(), too_many);
}

async fn new_digest(a: &TestApp, password: &str) -> campfire_db::PasswordDigest {
    campfire_db::PasswordDigest::hash(password.into(), a.db().env().bcrypt_cost)
        .await
        .unwrap()
}

/// David's password digest, audit rows and sessions, to show a refused reset changed none of them.
async fn reset_state(a: &TestApp) -> Value {
    a.db()
        .read(|conn| {
            let digest = campfire_db::User::find(conn, DAVID)?.password_digest;
            let audits: i64 = conn.query_row("SELECT count(*) FROM audit_logs", [], |r| r.get(0))?;
            let sessions: i64 = conn.query_row(
                "SELECT count(*) FROM sessions WHERE user_id=?",
                [DAVID],
                |r| r.get(0),
            )?;
            Ok(json!([digest, audits, sessions]))
        })
        .await
        .unwrap()
}

/// Two uses of one link can both pass the read-side check; the writer's recheck stops the second.
#[tokio::test]
async fn a_claim_checked_before_the_password_changed_resets_nothing() {
    use campfire_people::controllers::password_resets::reset_with_claim;
    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let token = emailed_token(&a, &david).await;
    let secrets = a.booted.app.secrets.clone();
    let claim = rails_compat::password_reset::verify(&secrets, &token, a.booted.app.clock.now())
        .expect("a fresh link");

    let first = new_digest(&a, NEW_PASSWORD).await;
    let (s, c) = (secrets.clone(), claim.clone());
    let reset = a
        .db()
        .write(move |tx| reset_with_claim(tx, &s, &c, first, &Default::default()))
        .await
        .unwrap();
    assert!(reset);
    let _signed_in = a.sign_in(DAVID).await;
    let before = reset_state(&a).await;

    let second = new_digest(&a, "the racing passphrase").await;
    let reset = a
        .db()
        .write(move |tx| reset_with_claim(tx, &secrets, &claim, second, &Default::default()))
        .await
        .unwrap();
    assert!(!reset);
    assert_eq!(reset_state(&a).await, before);
}

/// A link checked just before it expired changes nothing once hashing the new password has taken
/// it past its expiry: the writer checks the expiry again by its own clock.
#[tokio::test]
async fn a_link_that_expires_before_the_write_resets_nothing() {
    use campfire_people::controllers::password_resets::reset_with_claim;
    let (a, clock) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let token = emailed_token(&a, &david).await;
    let secrets = a.booted.app.secrets.clone();
    clock.advance(rails_compat::password_reset::EXPIRES_IN - jiff::SignedDuration::from_secs(1));
    let claim = rails_compat::password_reset::verify(&secrets, &token, a.booted.app.clock.now())
        .expect("a second from expiry");
    let digest = new_digest(&a, NEW_PASSWORD).await;
    let before = reset_state(&a).await;
    clock.advance(jiff::SignedDuration::from_secs(1));
    let reset = a
        .db()
        .write(move |tx| reset_with_claim(tx, &secrets, &claim, digest, &Default::default()))
        .await
        .unwrap();
    assert!(!reset);
    assert_eq!(reset_state(&a).await, before);
}

/// Delivery runs when the queue gets to it, perhaps after downtime or SMTP retries: the link still
/// expires 15 minutes after the request, nothing goes out once those are over, and nothing goes
/// out once the password has changed since the request.
#[tokio::test]
async fn late_and_superseded_deliveries_keep_the_request_s_fifteen_minutes() {
    use jiff::SignedDuration;
    use rails_compat::password_reset::{EXPIRES_IN, verify};
    let (a, clock) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let secrets = a.booted.app.secrets.clone();
    let now = a.booted.app.clock.now();

    assert!(reset_message(&a, &david, now - EXPIRES_IN).await.is_none());

    let requested = now - SignedDuration::from_mins(10);
    let token = link_token(&reset_message(&a, &david, requested).await.expect("still in time"));
    let left = SignedDuration::from_mins(5);
    assert!(verify(&secrets, &token, now + left - SignedDuration::from_secs(1)).is_some());
    assert!(verify(&secrets, &token, now + left).is_none());

    let reply = a.anonymous().write(submit(&token, NEW_PASSWORD)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert!(reset_message(&a, &david, requested).await.is_none());
    assert!(reset_message(&a, &david, now).await.is_none(), "changed at the request's instant");

    clock.advance(SignedDuration::from_secs(1));
    let later = a.booted.app.clock.now();
    assert!(reset_message(&a, &david, later).await.is_some(), "a request after the change");
}

/// A password sign-in as `POST /api/v1/session` makes one, except that with `?reset=1` the
/// password is reset (as a reset committing meanwhile would) after it is checked and before the
/// session starts. 200 when a session started, 409 when it was refused.
async fn sign_in_racing_a_reset(c: &mut campfire_kit::Ctx) -> campfire_kit::Result {
    use campfire_app::app::AppCtx;
    crate::concerns::before_actions(
        c,
        crate::concerns::Before::default().allow_unauthenticated_access(),
    )
    .await?;
    let email = c.param_str("email_address").unwrap_or_default().to_owned();
    let user = crate::concerns::authenticate_by(c, email, PASSWORD.into())
        .await?
        .expect("the seed's password");
    let proof = crate::concerns::password_proof(c, &user);
    if c.param_str("reset") == Some("1") {
        let digest = campfire_db::PasswordDigest::hash(NEW_PASSWORD.into(), 4)
            .await
            .unwrap();
        let id = user.id;
        c.app()
            .db
            .write(move |tx| {
                let mut user = campfire_db::User::find(tx.conn(), id)?;
                campfire_runtime::authentication::reset_password(
                    tx,
                    &mut user,
                    digest,
                    &Default::default(),
                )
            })
            .await
            .unwrap();
    }
    let started = crate::concerns::start_password_session_for(c, user, false, proof).await?;
    Ok(c.head(if started.is_some() {
        StatusCode::OK
    } else {
        StatusCode::CONFLICT
    }))
}

async fn race_sign_in(a: &TestApp, email: &str, reset: bool) -> StatusCode {
    use tower::ServiceExt;
    let app = &a.booted.app;
    let kit = campfire_kit::Kit::new(
        campfire_kit::KitConfig::production(app.config.disable_ssl),
        Arc::new(campfire_kit::RailsCrypto::new(app.secrets.clone())),
        app.clock.clone(),
        app.clone(),
    );
    let router = axum::Router::new()
        .route(
            "/race",
            axum::routing::get(campfire_kit::action(sign_in_racing_a_reset)),
        )
        .with_state(kit.clone())
        .layer(axum::middleware::from_fn_with_state(
            kit,
            campfire_kit::adapter::rails_middleware,
        ));
    let uri = format!(
        "/race?email_address={}&reset={}",
        email.replace('@', "%40"),
        u8::from(reset)
    );
    router
        .oneshot(
            axum::http::Request::builder()
                .uri(uri)
                .header(axum::http::header::HOST, "example.org")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

/// A password sign-in that checked the password a reset then replaced starts no session.
#[tokio::test]
async fn a_sign_in_that_checked_the_replaced_password_starts_no_session() {
    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let sessions = "SELECT count(*) FROM sessions WHERE user_id=?";
    let before = count(&a, sessions, DAVID).await;
    assert_eq!(race_sign_in(&a, &david, false).await, StatusCode::OK);
    assert_eq!(count(&a, sessions, DAVID).await, before + 1);

    assert_eq!(race_sign_in(&a, &david, true).await, StatusCode::CONFLICT);
    assert_eq!(count(&a, sessions, DAVID).await, 0);
}

/// A second step started with the old password can't finish once a reset has replaced it, even
/// with a right code: the new password is needed. With it, the second step works as before.
#[tokio::test]
async fn a_pending_second_step_from_the_old_password_ends_at_a_reset() {
    use rails_compat::{ar_encryption::ArEncryption, totp};
    let (a, clock) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let enc = ArEncryption::new(&a.booted.app.secrets);
    let secret = a
        .db()
        .read(move |conn| {
            let credential = campfire_db::TwoFactorCredential::for_user(conn, DAVID)?
                .expect("David has two-step sign-in");
            credential.secret(&enc)
        })
        .await
        .unwrap();
    let code = || totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    let sessions = "SELECT count(*) FROM sessions WHERE user_id=?";

    let mut b = a.anonymous();
    let pending = b.password_sign_in(&david, PASSWORD).await;
    assert_eq!(pending.json()["kind"], "secondFactorRequired", "{}", pending.text());
    let token = emailed_token(&a, &david).await;
    let reply = a.anonymous().write(submit(&token, NEW_PASSWORD)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());

    let reply = b.answer_challenge(&code(), false).await;
    assert_eq!(
        reply.json(),
        json!({"kind":"navigate","location":"http://campfire.test/app/session/new"}),
        "{}",
        reply.text()
    );
    assert_eq!(count(&a, sessions, DAVID).await, 0);

    clock.advance(jiff::SignedDuration::from_secs(30));
    let pending = b.password_sign_in(&david, NEW_PASSWORD).await;
    assert_eq!(pending.json()["kind"], "secondFactorRequired", "{}", pending.text());
    let reply = b.answer_challenge(&code(), false).await;
    assert_eq!(reply.json()["kind"], "signedIn", "{}", reply.text());
    assert_eq!(count(&a, sessions, DAVID).await, 1);
}

/// Asking for a link and setting a password need the page's CSRF token, as the other signed-out
/// JSON writes do: without it or with a wrong one, 422 and nothing happens.
#[tokio::test]
async fn requests_and_new_passwords_need_the_csrf_token() {
    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    capture_mail_jobs(&a).await;
    let token = emailed_token(&a, &david).await;
    let before = reset_state(&a).await;
    let mut b = a.anonymous();
    b.get("/app/passwords/new").await;
    for csrf in [None, Some("invalid")] {
        for req in [request(&david, "198.51.100.9"), submit(&token, NEW_PASSWORD)] {
            let path = req.path.clone();
            let req = match csrf {
                Some(value) => req.header(campfire_kit::csrf::HEADER, value),
                None => req,
            };
            let reply = b.send(req).await;
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{path} {csrf:?}");
        }
    }
    assert_eq!(captured_mail_jobs(&a).await, Vec::<Value>::new());
    assert_eq!(reset_state(&a).await, before);
    assert_eq!(b.send(check(&token)).await.status, StatusCode::OK);
}

#[tokio::test]
async fn a_profile_password_change_rechecks_its_session_and_digest_after_authorization() {
    use campfire_db::{NewSession, Session, User, UserChanges};
    use campfire_runtime::authentication::{reset_password, update_profile};

    for (replace_password, revoke_session) in [(true, true), (true, false), (false, true)] {
        let (a, _) = app(&MAIL).await;
        let (mut checked_user, session_id) = a
            .db()
            .write(|tx| {
                let user = User::find(tx.conn(), DAVID)?;
                let session = Session::start_with(
                    tx,
                    DAVID,
                    NewSession {
                        two_factor_verified: true,
                        ..Default::default()
                    },
                )?;
                Ok((user, session.id))
            })
            .await
            .unwrap();
        assert!(checked_user.current_password_confirmed(PASSWORD));
        let changes = UserChanges {
            email_address: Some(Some("other@example.com".into())),
            password_digest: Some(new_digest(&a, "stale write").await),
            ..Default::default()
        };

        let digest = new_digest(&a, NEW_PASSWORD).await;
        a.db()
            .write(move |tx| {
                let mut user = User::find(tx.conn(), DAVID)?;
                if !revoke_session {
                    // Change the digest without revoking the session, isolating the digest check.
                    return user.update(
                        tx,
                        UserChanges {
                            password_digest: Some(digest),
                            ..Default::default()
                        },
                    );
                }
                let previous = user.password_digest.clone();
                reset_password(tx, &mut user, digest, &Default::default())?;
                // Keep the old digest in one case to isolate the deleted-session check.
                if !replace_password {
                    tx.conn().execute(
                        "UPDATE users SET password_digest=? WHERE id=?",
                        rusqlite::params![previous, DAVID],
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();

        let saved = a
            .db()
            .write(move |tx| {
                update_profile(
                    tx,
                    &mut checked_user,
                    changes,
                    true,
                    Some(session_id),
                    false,
                    &Default::default(),
                )
            })
            .await
            .unwrap();
        assert!(!saved);
        let user = a.db().read(|conn| User::find(conn, DAVID)).await.unwrap();
        assert_eq!(user.email_address.as_deref(), Some("david@37signals.com"));
        assert!(user.authenticate(if replace_password {
            NEW_PASSWORD
        } else {
            PASSWORD
        }));
    }
}

#[tokio::test]
async fn a_whitespace_profile_password_change_is_audited_and_supersedes_reset_mail() {
    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let requested = a.booted.app.clock.now() - jiff::SignedDuration::from_secs(1);
    assert!(reset_message(&a, &david, requested).await.is_some());
    let reply = a
        .david()
        .write(
            Req::new(Method::PATCH, "/api/v1/settings/profile")
                .json_body(&json!({"password":"   "})),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let user = a
        .db()
        .read(|conn| campfire_db::User::find(conn, DAVID))
        .await
        .unwrap();
    assert!(user.authenticate("   "));
    assert_eq!(
        count(
            &a,
            "SELECT count(*) FROM audit_logs WHERE action='user.password.change' AND target_id=?",
            DAVID
        )
        .await,
        1
    );
    assert!(reset_message(&a, &david, requested).await.is_none());
}

const SESSIONS: &str = "SELECT count(*) FROM sessions WHERE user_id=?";
const REMEMBERED: &str = "SELECT count(*) FROM two_factor_remembered_devices WHERE user_id=?";
const BACKUP: &str = "23456-789ab";

async fn add_backup_code(a: &TestApp) -> i64 {
    a.db()
        .write(|tx| {
            use campfire_db::{TwoFactorBackupCode, TwoFactorCredential};
            let credential = TwoFactorCredential::for_user(tx.conn(), DAVID)?.unwrap();
            let code = TwoFactorBackupCode::create(
                tx,
                credential.id,
                &TwoFactorBackupCode::digest(BACKUP),
            )?;
            Ok(code.id)
        })
        .await
        .unwrap()
}

async fn used_backup(a: &TestApp, id: i64) -> i64 {
    count(
        a,
        "SELECT count(*) FROM two_factor_backup_codes WHERE id=? AND used_at IS NOT NULL",
        id,
    )
    .await
}

#[tokio::test]
async fn a_reset_before_the_challenge_write_leaves_the_backup_code_unused() {
    use campfire_runtime::authentication::{Challenge, ChallengeSignIn, challenge};
    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let backup = add_backup_code(&a).await;
    let old_user = a
        .db()
        .read(|conn| campfire_db::User::find(conn, DAVID))
        .await
        .unwrap();
    let secrets = a.booted.app.secrets.clone();
    let proof = rails_compat::password_reset::sign_in_proof(
        &secrets,
        old_user.password_digest.as_deref().unwrap(),
    );
    let token = emailed_token(&a, &david).await;
    assert_eq!(
        a.anonymous()
            .write(submit(&token, NEW_PASSWORD))
            .await
            .status,
        StatusCode::OK,
    );

    // A request has already checked the pending cookie, but the reset wins the writer lock.
    let encryption = a.booted.app.ar_encryption.clone();
    let outcome = a
        .db()
        .write(move |tx| {
            challenge(
                tx,
                &secrets,
                &encryption,
                ChallengeSignIn {
                    user_id: DAVID,
                    password_proof: Some(&proof),
                    code: BACKUP,
                    method: "password",
                    device_id: "reset-first",
                    remember_device: true,
                    notify_lockout: false,
                    notify_new_sign_in: false,
                },
                &Default::default(),
            )
        })
        .await
        .unwrap();
    assert!(matches!(outcome, Challenge::Gone));
    assert_eq!(used_backup(&a, backup).await, 0);
    assert_eq!(count(&a, SESSIONS, DAVID).await, 0);
    assert_eq!(count(&a, REMEMBERED, DAVID).await, 0);
}

#[tokio::test]
async fn a_reset_after_a_challenge_revokes_the_new_session_and_device() {
    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let backup = add_backup_code(&a).await;
    let token = emailed_token(&a, &david).await;
    let before = count(&a, SESSIONS, DAVID).await;
    let devices = count(&a, REMEMBERED, DAVID).await;
    let mut b = a.anonymous();
    let pending = b.password_sign_in(&david, PASSWORD).await;
    assert_eq!(
        pending.json()["kind"],
        "secondFactorRequired",
        "{}",
        pending.text()
    );
    let reply = b
        .write(
            Req::new(Method::POST, "/api/v1/two_factor/challenge")
                .header("x-forwarded-proto", "https")
                .json_body(&json!({"code":BACKUP,"rememberDevice":true})),
        )
        .await;
    assert_eq!(reply.json()["kind"], "signedIn", "{}", reply.text());
    assert!(b.cookie_header().contains("two_factor_remember="));
    assert_eq!(used_backup(&a, backup).await, 1);
    assert_eq!(count(&a, SESSIONS, DAVID).await, before + 1);
    assert_eq!(count(&a, REMEMBERED, DAVID).await, devices + 1);

    assert_eq!(
        a.anonymous()
            .write(submit(&token, NEW_PASSWORD))
            .await
            .status,
        StatusCode::OK,
    );
    assert_eq!(used_backup(&a, backup).await, 1);
    assert_eq!(count(&a, SESSIONS, DAVID).await, 0);
    assert_eq!(count(&a, REMEMBERED, DAVID).await, 0);
    assert_eq!(
        b.send(Req::new(Method::GET, "/api/v1/settings"))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn a_remembered_device_write_failure_rolls_back_the_challenge() {
    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let backup = add_backup_code(&a).await;
    let before = count(&a, SESSIONS, DAVID).await;
    let devices = count(&a, REMEMBERED, DAVID).await;
    let mut b = a.anonymous();
    let pending = b.password_sign_in(&david, PASSWORD).await;
    assert_eq!(
        pending.json()["kind"],
        "secondFactorRequired",
        "{}",
        pending.text()
    );
    a.db()
        .write(|tx| {
            tx.conn().execute_batch(
            "CREATE TRIGGER refuse_remembered_device BEFORE INSERT ON two_factor_remembered_devices
             BEGIN SELECT RAISE(ABORT, 'test remembered-device write failed'); END;",
        )?;
            Ok(())
        })
        .await
        .unwrap();

    assert_eq!(
        b.answer_challenge(BACKUP, true).await.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(used_backup(&a, backup).await, 0);
    assert_eq!(count(&a, SESSIONS, DAVID).await, before);
    assert_eq!(count(&a, REMEMBERED, DAVID).await, devices);
    assert!(!b.cookie_header().contains("session_token="));
    assert!(!b.cookie_header().contains("two_factor_remember="));
}
