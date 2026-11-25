//! "Forgot password?" over the frozen seed: availability, the uniform request, delivery, and the
//! one-time link that sets a new password and signs the user out everywhere.
use crate::controllers::presenters::test_support::{
    BENDER, Browser, DAVID, JASON, KEVIN, Reply, Req, SEED_NOW, TestApp,
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
const ENROLLMENT_SETUP: &str = "/api/v1/two_factor/setup";

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
pub(super) async fn capture_mail_jobs(a: &TestApp) {
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

pub(super) async fn captured_mail_jobs(a: &TestApp) -> Vec<Value> {
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
        (
            &[MAIL[0], MAIL[1], MAIL[2], ("SMTP_USER_NAME", "user")][..],
            false,
        ),
        (
            &[
                MAIL[0],
                MAIL[1],
                MAIL[2],
                ("SMTP_USER_NAME", "user"),
                ("SMTP_PASSWORD", "password"),
                ("SMTP_AUTHENTICATION", "unsupported"),
            ][..],
            false,
        ),
        (
            &[
                MAIL[0],
                MAIL[1],
                MAIL[2],
                ("SMTP_USER_NAME", "user"),
                ("SMTP_PASSWORD", "password"),
                ("SMTP_AUTHENTICATION", "cram_md5"),
            ][..],
            true,
        ),
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
    assert!(
        answers[0]
            .3
            .as_deref()
            .is_some_and(|value| value.contains("no-store")),
        "{answers:?}"
    );
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
async fn oversized_password_reset_addresses_spend_both_limits_without_queuing() {
    let (a, _) = app(&MAIL).await;
    capture_mail_jobs(&a).await;
    let mut b = a.anonymous();
    let local = "é".repeat(121);
    let too_long = format!("  {local}x@example.com  ");
    let at_limit = format!("  {local}@example.com  ");
    assert_eq!(at_limit.trim().len(), 254);
    assert_eq!(too_long.trim().len(), 255);

    let ip = "198.51.100.20";
    for _ in 0..3 {
        let reply = b.write(request(&too_long, ip)).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.json(), json!({"kind":"requested"}));
    }
    assert_eq!(
        b.write(request(&too_long, "198.51.100.21")).await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
    for address in [&at_limit, "unknown@example.com"] {
        assert_eq!(b.write(request(address, ip)).await.status, StatusCode::OK);
    }
    assert_eq!(
        b.write(request("another@example.com", ip)).await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        captured_mail_jobs(&a).await,
        [at_limit.as_str(), "unknown@example.com"]
            .map(|email| json!({"notification":{"PasswordReset":{"email_address":email}}}))
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
    let reply = b
        .write(request("\u{a0}INBOX@example.com\t", "198.18.0.9"))
        .await;
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

/// Captures one SMTP delivery, including the original MIME body.
async fn capture_smtp_message(
    stream: tokio::net::TcpStream,
    sent: &tokio::sync::mpsc::UnboundedSender<String>,
) {
    use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
    let mut stream = BufReader::new(stream);
    stream
        .write_all(b"220 localhost test SMTP\r\n")
        .await
        .unwrap();
    loop {
        let mut command = String::new();
        if stream.read_line(&mut command).await.unwrap() == 0 {
            return;
        }
        let response: &[u8] = match command.trim_end() {
            "DATA" => {
                let mut message = String::new();
                stream
                    .write_all(b"354 End with <CRLF>.<CRLF>\r\n")
                    .await
                    .unwrap();
                loop {
                    let mut line = String::new();
                    assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
                    if line == ".\r\n" {
                        break;
                    }
                    message.push_str(&line);
                }
                sent.send(message).unwrap();
                b"250 Stored\r\n"
            }
            "QUIT" => {
                stream.write_all(b"221 Bye\r\n").await.unwrap();
                return;
            }
            command
                if command.starts_with("EHLO ")
                    || command.starts_with("MAIL FROM:")
                    || command.starts_with("RCPT TO:") =>
            {
                b"250 OK\r\n"
            }
            command => panic!("unexpected SMTP command: {command}"),
        };
        stream.write_all(response).await.unwrap();
    }
}

async fn app_with_smtp() -> (
    TestApp,
    Arc<campfire_kit::FrozenClock>,
    tokio::sync::mpsc::UnboundedReceiver<String>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    let (sent, received) = tokio::sync::mpsc::unbounded_channel();
    let smtp = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            capture_smtp_message(stream, &sent).await;
        }
    });
    let (a, clock) = app(&[
        ("SMTP_ADDRESS", "127.0.0.1"),
        ("SMTP_PORT", &port),
        ("SMTP_ENABLE_STARTTLS", "false"),
        MAIL[1],
        MAIL[2],
    ])
    .await;
    (a.without_job_runner().await, clock, received, smtp)
}

fn start_mail_runner(a: &TestApp) -> campfire_jobs::Runner {
    use campfire_jobs::{QueueConfig, Registry, RunnerConfig};

    let mut registry = Registry::new();
    campfire_runtime::mail::register(&mut registry);
    let config = RunnerConfig::new(vec![QueueConfig::new("default", 1)]);
    campfire_jobs::start(
        a.db().clone(),
        a.booted.app.jobs.queue.clone(),
        registry,
        a.booted.app.clone(),
        config,
    )
}

/// Drive the registered handler, whose Execution takes enqueued_at from the job's created_at.
async fn run_password_reset_delivery(a: &TestApp, email: &str, age: jiff::SignedDuration) {
    use campfire_jobs::{Job, JobRequest};
    use campfire_mail::jobs::{DeliveryJob, Notification};

    let queue = a.booted.app.jobs.queue.clone();
    let email_address = email.to_owned();
    let id = a
        .db()
        .write(move |tx| {
            let id = queue.enqueue(
                tx,
                &JobRequest::new(&DeliveryJob {
                    notification: Notification::PasswordReset { email_address },
                }),
            )?;
            tx.conn().execute(
                "UPDATE background_jobs SET created_at = ? WHERE id = ?",
                rusqlite::params![tx.now().since(-age), id],
            )?;
            Ok(id)
        })
        .await
        .unwrap();
    a.booted.app.jobs.queue.wake(DeliveryJob::CLASS);
    wait_for_delivery(a, id).await;
}

async fn wait_for_delivery(a: &TestApp, id: i64) {
    use std::time::Duration;

    tokio::time::timeout(Duration::from_secs(5), async {
        while count(a, "SELECT count(*) FROM background_jobs WHERE id=?", id).await != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("delivery job {id} did not complete"));
}

#[tokio::test]
async fn password_reset_delivery_job_uses_enqueue_time_and_skips_stale_requests() {
    use jiff::SignedDuration;
    use rails_compat::password_reset::{EXPIRES_IN, verify};
    use std::time::Duration;

    let (a, clock, mut received, smtp) = app_with_smtp().await;
    let runner = start_mail_runner(&a);
    let david = email_of(&a, DAVID).await;
    let secrets = &a.booted.app.secrets;
    let now = a.booted.app.clock.now();

    let age = SignedDuration::from_mins(13);
    run_password_reset_delivery(&a, &david, age).await;
    let raw = tokio::time::timeout(Duration::from_secs(5), received.recv())
        .await
        .unwrap()
        .unwrap();
    let prefix = "http://campfire.test/app/passwords/";
    let token = raw
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .expect("reset URL");
    let expires_at = now - age + EXPIRES_IN;
    assert!(verify(secrets, token, expires_at - SignedDuration::from_secs(1)).is_some());
    assert!(verify(secrets, token, expires_at).is_none());

    for age in [EXPIRES_IN, age + SignedDuration::from_secs(1)] {
        run_password_reset_delivery(&a, &david, age).await;
        assert!(
            received.try_recv().is_err(),
            "mail sent for a request aged {age}"
        );
    }

    let reply = a.anonymous().write(submit(token, NEW_PASSWORD)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    for age in [SignedDuration::from_secs(1), SignedDuration::ZERO] {
        run_password_reset_delivery(&a, &david, age).await;
        assert!(
            received.try_recv().is_err(),
            "mail sent for a superseded request"
        );
    }

    clock.advance(SignedDuration::from_secs(1));
    run_password_reset_delivery(&a, &david, SignedDuration::ZERO).await;
    let raw = tokio::time::timeout(Duration::from_secs(5), received.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(raw.lines().any(|line| line.starts_with(prefix)));
    runner.shutdown(Duration::from_secs(1)).await;
    smtp.abort();
    assert!(smtp.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn password_reset_and_sign_in_prefer_the_exact_padded_address() {
    use std::time::Duration;

    let (a, _, mut received, smtp) = app_with_smtp().await;
    let padded = " Ada@example.com ";
    let ambiguous = "ada@example.com";
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE users SET email_address = ? WHERE id = ?",
                rusqlite::params![padded, DAVID],
            )?;
            tx.conn().execute(
                "UPDATE users SET email_address = 'ADA@example.com', password_digest = NULL WHERE id = ?",
                [KEVIN],
            )?;
            campfire_db::User::find(tx.conn(), DAVID)?.reset_two_factor(tx)
        })
        .await
        .unwrap();

    let mut browser = a.anonymous();
    let refused = browser.password_sign_in(ambiguous, PASSWORD).await;
    assert_eq!(
        refused.status,
        StatusCode::UNAUTHORIZED,
        "{}",
        refused.text()
    );

    let ip = "198.51.100.20";
    let reply = browser.write(request(ambiguous, ip)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json(), json!({"kind":"requested"}));
    let id = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT id FROM background_jobs WHERE job_class = 'Smartfire::MailDeliveryJob'",
                [],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    let runner = start_mail_runner(&a);
    wait_for_delivery(&a, id).await;
    assert!(
        received.try_recv().is_err(),
        "mail sent for an ambiguous address"
    );

    let reply = browser.write(request(padded, ip)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json(), json!({"kind":"requested"}));
    let raw = tokio::time::timeout(Duration::from_secs(5), received.recv())
        .await
        .expect("reset mail for the exact padded address")
        .expect("SMTP capture is running");
    assert!(
        raw.lines()
            .any(|line| line.starts_with("To: ") && line.ends_with("<Ada@example.com>"))
    );
    let token = raw
        .lines()
        .find_map(|line| line.strip_prefix("http://campfire.test/app/passwords/"))
        .expect("reset URL");
    let claim = rails_compat::password_reset::verify(
        &a.booted.app.secrets,
        token,
        a.booted.app.clock.now(),
    )
    .expect("valid reset token");
    assert_eq!(claim.user_id, DAVID);

    runner.shutdown(Duration::from_secs(1)).await;
    smtp.abort();
    assert!(smtp.await.unwrap_err().is_cancelled());

    let signed_in = browser.password_sign_in(padded, PASSWORD).await;
    assert_eq!(signed_in.status, StatusCode::OK, "{}", signed_in.text());
    assert_eq!(signed_in.json()["kind"], "signedIn");
}

async fn reset_password_during_request(db: campfire_db::Database) {
    let digest = campfire_db::PasswordDigest::hash(NEW_PASSWORD.into(), db.env().bcrypt_cost)
        .await
        .unwrap();
    db.write(move |tx| {
        let mut user = campfire_db::User::find(tx.conn(), DAVID)?;
        campfire_runtime::authentication::reset_password(tx, &mut user, digest, &Default::default())
    })
    .await
    .unwrap();
}

/// A password sign-in that checked the password a reset then replaced starts no session.
#[tokio::test]
async fn a_sign_in_that_checked_the_replaced_password_starts_no_session() {
    use campfire_runtime::authentication::test_hooks::BEFORE_SESSION_WRITE;
    use std::cell::RefCell;

    let (a, _) = app(&MAIL).await;
    a.db()
        .write(|tx| campfire_db::User::find(tx.conn(), DAVID)?.reset_two_factor(tx))
        .await
        .unwrap();
    let david = email_of(&a, DAVID).await;
    let before = count(&a, SESSIONS, DAVID).await;
    let signed_in = a.anonymous().password_sign_in(&david, PASSWORD).await;
    assert_eq!(signed_in.json()["kind"], "signedIn");
    assert_eq!(count(&a, SESSIONS, DAVID).await, before + 1);

    let reply = BEFORE_SESSION_WRITE
        .scope(
            RefCell::new(Some(Box::pin(reset_password_during_request(
                a.db().clone(),
            )))),
            a.anonymous().password_sign_in(&david, PASSWORD),
        )
        .await;
    assert_eq!(
        reply.json(),
        json!({"kind":"navigate","location":"http://campfire.test/app/session/new"})
    );
    assert_eq!(count(&a, SESSIONS, DAVID).await, 0);
}

#[tokio::test]
async fn two_factor_enrollment_accepts_the_code_after_another_tab_reloads_setup() {
    use campfire_runtime::authentication::test_hooks::BEFORE_ENROLLMENT_CONFIRM;
    use std::cell::RefCell;

    let (a, clock) = app(&MAIL).await;
    let a = Arc::new(a);
    let (mut browser, secret, code) = prepare_enrollment(&a).await;
    let cookies = browser.cookie_header();
    let reload = {
        let a = a.clone();
        let secret = secret.clone();
        async move {
            clock.advance(jiff::SignedDuration::from_secs(1));
            let mut tab = a.anonymous();
            tab.absorb_cookie_header(&cookies);
            let reply = tab.get(ENROLLMENT_SETUP).await;
            assert_eq!(reply.status, StatusCode::OK);
            assert_eq!(reply.json()["setup"]["secret"], secret);
        }
    };
    let reply = BEFORE_ENROLLMENT_CONFIRM
        .scope(
            RefCell::new(Some(Box::pin(reload))),
            browser.write(enrollment_submission(&code)),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.json()["kind"], "recoveryCodes");
}

#[tokio::test]
async fn two_factor_enrollment_accepts_the_code_after_session_activity_refreshes() {
    use campfire_db::{Session, Timestamp, models::session::ACTIVITY_REFRESH_RATE};
    use campfire_runtime::authentication::test_hooks::BEFORE_ENROLLMENT_CONFIRM;
    use std::cell::RefCell;

    let (a, clock) = app(&MAIL).await;
    let (mut browser, _, code) = prepare_enrollment(&a).await;
    let session_id = a
        .db()
        .write(|tx| {
            let conn = tx.conn();
            let id: i64 = conn.query_row(
                "SELECT session_id FROM two_factor_setup_secrets
                 JOIN sessions ON sessions.id = session_id WHERE user_id = ?",
                [DAVID],
                |row| row.get(0),
            )?;
            conn.execute(
                "UPDATE sessions SET last_active_at = ?1 WHERE id = ?2",
                rusqlite::params![tx.now().ago(ACTIVITY_REFRESH_RATE), id],
            )?;
            Ok(id)
        })
        .await
        .unwrap();
    let db = a.db().clone();
    let refresh = async move {
        clock.advance(jiff::SignedDuration::from_secs(1));
        db.write(move |tx| {
            let mut session = Session::find(tx.conn(), session_id)?;
            assert!(session.needs_resume(tx.now()));
            session.resume(tx, None, None)?;
            Ok(())
        })
        .await
        .unwrap();
    };
    let reply = BEFORE_ENROLLMENT_CONFIRM
        .scope(
            RefCell::new(Some(Box::pin(refresh))),
            browser.write(enrollment_submission(&code)),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.json()["kind"], "recoveryCodes");
    let now = Timestamp::from_jiff(a.booted.app.clock.now());
    assert_eq!(
        a.db()
            .read(move |conn| Ok(Session::find(conn, session_id)?.last_active_at))
            .await
            .unwrap(),
        now
    );
}

async fn prepare_enrollment(a: &TestApp) -> (Browser<'_>, String, String) {
    a.db()
        .write(|tx| campfire_db::User::find(tx.conn(), DAVID)?.reset_two_factor(tx))
        .await
        .unwrap();
    let mut browser = a.sign_in(DAVID).await;
    let reply = browser.get(ENROLLMENT_SETUP).await;
    assert_eq!(reply.status, StatusCode::OK);
    let secret = reply.json()["setup"]["secret"].as_str().unwrap().to_owned();
    let code = rails_compat::totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    (browser, secret, code)
}

fn enrollment_submission(code: &str) -> Req {
    Req::new(Method::POST, ENROLLMENT_SETUP).json_body(&json!({"code":code}))
}

#[tokio::test]
async fn a_password_reset_before_enrollment_confirm_leaves_two_factor_disabled() {
    use campfire_db::TwoFactorCredential;
    use campfire_runtime::authentication::test_hooks::BEFORE_ENROLLMENT_CONFIRM;
    use std::cell::RefCell;

    let (a, _) = app(&MAIL).await;
    let (mut browser, _, code) = prepare_enrollment(&a).await;
    let reply = BEFORE_ENROLLMENT_CONFIRM
        .scope(
            RefCell::new(Some(Box::pin(reset_password_during_request(
                a.db().clone(),
            )))),
            browser.write(enrollment_submission(&code)),
        )
        .await;
    assert_eq!(count(&a, SESSIONS, DAVID).await, 0);
    assert!(
        !a.db()
            .read(|conn| Ok(
                TwoFactorCredential::for_user(conn, DAVID)?.is_some_and(|c| c.enabled())
            ))
            .await
            .unwrap()
    );
    assert_eq!(
        count(
            &a,
            "SELECT count(*) FROM two_factor_backup_codes
                   WHERE two_factor_credential_id IN
                   (SELECT id FROM two_factor_credentials WHERE user_id=?)",
            DAVID
        )
        .await,
        0
    );
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.text());
    assert_eq!(
        reply.json(),
        json!({"kind":"navigate","location":"http://campfire.test/app/session/new"})
    );
}

#[tokio::test]
async fn transfer_requires_a_challenge_if_the_remembered_device_is_revoked_before_session_write() {
    use campfire_runtime::authentication::test_hooks::BEFORE_SESSION_WRITE;
    use std::cell::RefCell;

    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let mut browser = a.anonymous();
    assert_eq!(
        browser.password_sign_in(&david, PASSWORD).await.json()["kind"],
        "secondFactorRequired"
    );
    add_backup_code(&a).await;
    assert_eq!(
        browser
            .write(
                Req::new(Method::POST, "/api/v1/two_factor/challenge")
                    .header("x-forwarded-proto", "https")
                    .json_body(&json!({"code":BACKUP,"rememberDevice":true})),
            )
            .await
            .json()["kind"],
        "signedIn"
    );
    let remember = browser
        .cookie_header()
        .split("; ")
        .find(|pair| pair.starts_with("two_factor_remember="))
        .unwrap()
        .to_owned();
    let token = crate::controllers::presenters::accounts::transfer_id(
        &a.booted.app.secrets,
        DAVID,
        a.booted.app.clock.now(),
    );
    let transfer = || {
        Req::new(Method::PUT, &format!("/api/v1/session/transfers/{token}"))
            .header("x-forwarded-proto", "https")
    };
    let mut browser = a.anonymous();
    browser.absorb_cookie_header(&remember);
    assert_eq!(browser.write(transfer()).await.json()["kind"], "signedIn");

    let mut browser = a.anonymous();
    browser.absorb_cookie_header(&remember);
    let db = a.db().clone();
    let revoke = async move {
        db.write(|tx| {
            TwoFactorRememberedDevice::revoke_all(tx, DAVID)?;
            Ok(())
        })
        .await
        .unwrap();
    };
    let before = count(&a, SESSIONS, DAVID).await;
    let reply = BEFORE_SESSION_WRITE
        .scope(
            RefCell::new(Some(Box::pin(revoke))),
            browser.write(transfer()),
        )
        .await;
    assert_eq!(
        reply.json()["kind"],
        "secondFactorRequired",
        "{}",
        reply.text()
    );
    assert_eq!(count(&a, SESSIONS, DAVID).await, before);
    assert_eq!(count(&a, REMEMBERED, DAVID).await, 0);
}

/// A second step started with the old password can't finish once a reset has replaced it, even
/// with a right code: the new password is needed. With it, the second step works as before.
#[tokio::test]
async fn a_pending_second_step_from_the_old_password_ends_at_a_reset() {
    use rails_compat::{ar_encryption::ArEncryption, totp};
    let (a, clock) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let secondary = "linked@example.test";
    confirmed_secondary(&a, secondary).await;
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
    let pending = b.password_sign_in(secondary, PASSWORD).await;
    assert_eq!(
        pending.json()["kind"],
        "secondFactorRequired",
        "{}",
        pending.text()
    );
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
    let pending = b.password_sign_in(secondary, NEW_PASSWORD).await;
    assert_eq!(
        pending.json()["kind"],
        "secondFactorRequired",
        "{}",
        pending.text()
    );
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
async fn password_reset_addresses_preserve_spelling_and_require_an_unambiguous_match() {
    let (a, _) = app(&MAIL).await;
    let a = a.without_job_runner().await;
    let david = email_of(&a, DAVID).await;
    let now = a.booted.app.clock.now();
    let mut b = a.anonymous();
    capture_mail_jobs(&a).await;

    for email in ["David@37signals.com", " david@37signals.com "] {
        assert_eq!(
            b.write(request(email, "198.51.100.20")).await.status,
            StatusCode::OK
        );
    }
    let jobs = captured_mail_jobs(&a).await;
    for (job, expected) in jobs
        .iter()
        .zip(["David@37signals.com", " david@37signals.com "])
    {
        let address = job["notification"]["PasswordReset"]["email_address"]
            .as_str()
            .unwrap();
        assert_eq!(address, expected);
        let message = reset_message(&a, address, now).await.unwrap();
        assert!(message.to.ends_with(&format!("<{david}>")));
    }
    assert_eq!(jobs.len(), 2);

    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET email_address = 'DAVID@37signals.com' WHERE id = ?",
                [JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        reset_message(&a, "David@37signals.com", now)
            .await
            .is_none()
    );
    let exact = reset_message(&a, &david, now).await.unwrap();
    assert!(exact.to.ends_with(&format!("<{david}>")));
}

async fn assert_reset_delivery_matches(stored: &'static str, submitted: &str, recipient: &str) {
    let (a, _) = app(&MAIL).await;
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE users SET email_address=?1 WHERE id=?2",
                rusqlite::params![stored, DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let message = reset_message(&a, submitted, a.booted.app.clock.now())
        .await
        .unwrap_or_else(|| panic!("expected a reset email to {stored:?}"));
    assert_eq!(
        campfire_mail::outbound::first_address(&message.to)
            .unwrap()
            .to_string(),
        recipient
    );
    a.db()
        .write(|tx| campfire_db::User::find(tx.conn(), DAVID)?.reset_two_factor(tx))
        .await
        .unwrap();
    let mut browser = a.anonymous();
    let token = link_token(&message);
    // The link must still verify against the stored address, with its case and spaces intact.
    assert_eq!(browser.send(check(&token)).await.status, StatusCode::OK);
    let reset = browser.write(submit(&token, NEW_PASSWORD)).await;
    assert_eq!(reset.status, StatusCode::OK, "{}", reset.text());
    let sign_in = browser.password_sign_in(submitted, NEW_PASSWORD).await;
    assert_eq!(sign_in.status, StatusCode::OK, "{}", sign_in.text());
    assert_eq!(sign_in.json()["kind"], "signedIn");
    assert_eq!(count(&a, SESSIONS, DAVID).await, 1);
}

#[tokio::test]
async fn password_reset_and_sign_in_match_padded_stored_addresses() {
    assert_reset_delivery_matches(
        "  david@37signals.com  ",
        "david@37signals.com",
        "david@37signals.com",
    )
    .await;
}

#[tokio::test]
async fn password_reset_and_sign_in_match_unicode_case() {
    assert_reset_delivery_matches("JÖRG@x.com", "jörg@x.com", "JÖRG@x.com").await;
}

#[tokio::test]
async fn password_reset_delivery_refuses_addresses_that_collide_after_trimming() {
    let (a, _) = app(&MAIL).await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET email_address='  DAVID@37SIGNALS.COM  ' WHERE id=?",
                [JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let now = a.booted.app.clock.now();
    assert!(
        reset_message(&a, "David@37signals.com", now)
            .await
            .is_none()
    );
    assert!(
        reset_message(&a, "david@37signals.com", now)
            .await
            .is_some()
    );
}

#[tokio::test]
async fn password_reset_and_sign_in_refuse_a_normalized_collision_with_google_only() {
    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let google_only = "DAVID@37SIGNALS.COM";
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE users SET email_address = ?, password_digest = NULL WHERE id = ?",
                rusqlite::params![google_only, KEVIN],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let now = a.booted.app.clock.now();
    let mut browser = a.anonymous();
    let ambiguous = "David@37signals.com";
    let refused = browser.password_sign_in(ambiguous, PASSWORD).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
    assert!(reset_message(&a, ambiguous, now).await.is_none());

    let exact = browser.password_sign_in(&david, PASSWORD).await;
    assert_eq!(exact.json()["kind"], "secondFactorRequired");
    let message = reset_message(&a, &david, now).await.unwrap();
    assert!(message.to.ends_with(&format!("<{david}>")));

    let exact_google = a.anonymous().password_sign_in(google_only, PASSWORD).await;
    assert_eq!(exact_google.status, StatusCode::UNAUTHORIZED);
    assert!(reset_message(&a, google_only, now).await.is_none());
}

#[tokio::test]
async fn password_sign_in_refuses_ambiguous_normalized_addresses() {
    let (a, _) = app(&MAIL).await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET email_address='  DAVID@37SIGNALS.COM  ',
                    password_digest=(SELECT password_digest FROM users WHERE id=?1)
                 WHERE id=?2",
                [DAVID, JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let exact = a
        .anonymous()
        .password_sign_in("david@37signals.com", PASSWORD)
        .await;
    assert_eq!(exact.json()["kind"], "secondFactorRequired");
    let mut browser = a.anonymous();
    let missing = browser
        .password_sign_in("unknown@example.com", PASSWORD)
        .await;
    let ambiguous = browser
        .password_sign_in("David@37signals.com", PASSWORD)
        .await;
    assert_eq!(ambiguous.status, StatusCode::UNAUTHORIZED);
    assert_eq!(ambiguous.json(), missing.json());
}

#[tokio::test]
async fn a_password_reset_link_is_refused_after_the_member_is_banned() {
    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let token = emailed_token(&a, &david).await;
    let mut b = a.anonymous();
    assert_eq!(b.send(check(&token)).await.status, StatusCode::OK);

    a.db()
        .write(|tx| {
            // Keep the request out of the IP-ban gate so it reaches the account check.
            tx.conn().execute(
                "UPDATE sessions SET ip_address=NULL WHERE user_id=?",
                [DAVID],
            )?;
            campfire_db::User::find(tx.conn(), DAVID)?.ban(tx)
        })
        .await
        .unwrap();
    let before = reset_state(&a).await;
    refused_link(&b.write(submit(&token, NEW_PASSWORD)).await);
    assert_eq!(reset_state(&a).await[0], before[0]);
    assert_eq!(
        count(
            &a,
            "SELECT count(*) FROM audit_logs WHERE action='user.password.reset' AND target_id=?",
            DAVID
        )
        .await,
        0
    );
}

async fn profile_authorization(a: &TestApp) -> (campfire_db::User, i64) {
    use campfire_db::{NewSession, Session, User};

    a.db()
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
        .unwrap()
}

#[tokio::test]
async fn a_profile_password_change_rechecks_its_session_and_digest_after_authorization() {
    use campfire_db::{User, UserChanges};
    use campfire_runtime::authentication::{reset_password, update_profile};

    for (replace_password, revoke_session) in [(true, true), (true, false), (false, true)] {
        let (a, _) = app(&MAIL).await;
        let (mut checked_user, session_id) = profile_authorization(&a).await;
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
async fn a_case_only_profile_email_edit_cannot_write_after_password_reset() {
    let (a, _) = app(&MAIL).await;
    let (mut user, session_id) = profile_authorization(&a).await;
    let email = user.email_address.clone().unwrap();
    let submitted = email.to_ascii_uppercase();
    assert_ne!(email, submitted);
    let email_change_requested = user.email_change_requested(&submitted);
    assert!(!email_change_requested);
    let token = emailed_token(&a, &email).await;
    let reset = a.anonymous().write(submit(&token, NEW_PASSWORD)).await;
    assert_eq!(reset.status, StatusCode::OK, "{}", reset.text());

    let saved = a
        .db()
        .write(move |tx| {
            campfire_runtime::authentication::update_profile(
                tx,
                &mut user,
                campfire_db::UserChanges {
                    email_address: Some(Some(submitted)),
                    ..Default::default()
                },
                email_change_requested,
                Some(session_id),
                false,
                &Default::default(),
            )
        })
        .await
        .unwrap();
    assert!(!saved);
    assert_eq!(email_of(&a, DAVID).await, email);
}

#[tokio::test]
async fn a_stale_profile_email_edit_cannot_restore_the_previous_address() {
    let (a, _) = app(&MAIL).await;
    let mut browser = a.david();
    let (mut user, session_id) = profile_authorization(&a).await;
    let submitted = user.email_address.clone();
    let replacement = "new@example.com";
    let updated = browser
        .write(
            Req::new(Method::PATCH, "/api/v1/settings/profile")
                .json_body(&json!({"emailAddress": replacement, "currentPassword": PASSWORD})),
        )
        .await;
    assert_eq!(updated.status, StatusCode::OK, "{}", updated.text());

    let saved = a
        .db()
        .write(move |tx| {
            campfire_runtime::authentication::update_profile(
                tx,
                &mut user,
                campfire_db::UserChanges {
                    email_address: Some(submitted),
                    ..Default::default()
                },
                false,
                Some(session_id),
                false,
                &Default::default(),
            )
        })
        .await
        .unwrap();
    assert!(!saved);
    assert_eq!(email_of(&a, DAVID).await, replacement);
}

#[tokio::test]
async fn make_primary_rechecks_the_session_digest_and_email_snapshot() {
    use campfire_db::{User, UserChanges};

    for (revoke_session, replace_password, replace_email) in
        [(true, false, false), (false, true, false), (false, false, true)]
    {
        let (a, _) = app(&MAIL).await;
        let secondary = confirmed_secondary(&a, "linked@example.test").await;
        let (mut user, session_id) = profile_authorization(&a).await;
        let digest = if replace_password {
            Some(new_digest(&a, NEW_PASSWORD).await)
        } else {
            None
        };
        let email = replace_email.then(|| Some("replacement@example.test".into()));
        a.db()
            .write(move |tx| {
                if revoke_session {
                    tx.conn()
                        .execute("DELETE FROM sessions WHERE id=?", [session_id])?;
                }
                User::find(tx.conn(), DAVID)?.update(
                    tx,
                    UserChanges {
                        password_digest: digest,
                        email_address: email,
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();

        let expected_email = email_of(&a, DAVID).await;
        capture_mail_jobs(&a).await;
        let saved = a
            .db()
            .write(move |tx| {
                campfire_runtime::authentication::promote_email_address(
                    tx,
                    &mut user,
                    secondary.id,
                    Some(session_id),
                    &Default::default(),
                )
            })
            .await
            .unwrap();
        assert!(!saved);
        assert_eq!(email_of(&a, DAVID).await, expected_email);
        assert_eq!(
            count(
                &a,
                "SELECT count(*) FROM user_email_addresses WHERE user_id=?",
                DAVID,
            )
            .await,
            1
        );
        assert!(captured_mail_jobs(&a).await.is_empty());
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
    use campfire_runtime::authentication::test_hooks::BEFORE_SESSION_WRITE;
    use std::cell::RefCell;

    let (a, _) = app(&MAIL).await;
    let david = email_of(&a, DAVID).await;
    let backup = add_backup_code(&a).await;
    let mut browser = a.anonymous();
    let pending = browser.password_sign_in(&david, PASSWORD).await;
    assert_eq!(pending.json()["kind"], "secondFactorRequired");

    let reply = BEFORE_SESSION_WRITE
        .scope(
            RefCell::new(Some(Box::pin(reset_password_during_request(
                a.db().clone(),
            )))),
            browser.answer_challenge(BACKUP, true),
        )
        .await;
    assert_eq!(
        reply.json(),
        json!({"kind":"navigate","location":"http://campfire.test/app/session/new"})
    );
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

#[tokio::test]
async fn a_secondary_signs_in_and_receives_reset_until_that_confirmation_is_removed() {
    use campfire_db::user::email_addresses::{Confirmation, EmailAddress};

    let (a, _) = app(&MAIL).await;
    let address = "Linked@example.test";
    let secondary = confirmed_secondary(&a, address).await;
    let signed_in = a
        .anonymous()
        .password_sign_in(&address.to_ascii_lowercase(), PASSWORD)
        .await;
    assert_eq!(signed_in.status, StatusCode::OK, "{}", signed_in.text());
    assert!(
        ["signedIn", "secondFactorRequired"].contains(&signed_in.json()["kind"].as_str().unwrap())
    );

    capture_mail_jobs(&a).await;
    let mut visitor = a.anonymous();
    let reply = visitor
        .write(request(&address.to_ascii_lowercase(), "198.51.100.1"))
        .await;
    assert_eq!(reply.json(), json!({"kind":"requested"}));
    let queued = captured_mail_jobs(&a).await;
    assert_eq!(
        queued,
        [json!({"notification":{"PasswordReset":{"email_address":address.to_ascii_lowercase()}}})]
    );
    let now = a.booted.app.clock.now();
    let requested_at = now - jiff::SignedDuration::from_mins(10);
    let message = reset_message(&a, &address.to_ascii_lowercase(), requested_at)
        .await
        .unwrap();
    assert!(
        message.to.ends_with(&format!("<{address}>")),
        "{}",
        message.to
    );
    let token = link_token(&message);
    assert!(
        rails_compat::password_reset::verify(
            &a.booted.app.secrets,
            &token,
            now + jiff::SignedDuration::from_mins(5),
        )
        .is_none()
    );
    assert_eq!(
        visitor.send(check(&token)).await.json(),
        json!({"kind":"ready"})
    );

    a.db()
        .write(move |tx| {
            EmailAddress::remove(tx, DAVID, secondary.id)?;
            Ok(())
        })
        .await
        .unwrap();
    refused_link(&visitor.send(check(&token)).await);
    refused_link(&visitor.write(submit(&token, NEW_PASSWORD)).await);
    assert!(reset_message(&a, address, now).await.is_none());
    a.db()
        .write(move |tx| {
            let user = campfire_db::User::find(tx.conn(), DAVID)?;
            assert_eq!(
                EmailAddress::confirm(tx, &user, address)?,
                Confirmation::Added
            );
            Ok(())
        })
        .await
        .unwrap();
    refused_link(&visitor.send(check(&token)).await);
}

async fn confirmed_secondary(
    a: &TestApp,
    address: &'static str,
) -> campfire_db::user::email_addresses::EmailAddress {
    use campfire_db::user::email_addresses::{Confirmation, EmailAddress};

    a.db()
        .write(move |tx| {
            let user = campfire_db::User::find(tx.conn(), DAVID)?;
            assert_eq!(
                EmailAddress::confirm(tx, &user, address)?,
                Confirmation::Added
            );
            Ok(EmailAddress::by_address(tx.conn(), address)?.unwrap().1)
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn password_reset_delivers_a_padded_mixed_case_request_to_the_confirmed_secondary() {
    let (a, _) = app(&MAIL).await;
    let a = a.without_job_runner().await;
    let address = "Linked@example.test";
    let secondary = confirmed_secondary(&a, address).await;
    capture_mail_jobs(&a).await;

    let mut visitor = a.anonymous();
    let submitted = "\u{a0} lInKeD@example.test\t";
    let reply = visitor.write(request(submitted, "198.51.100.21")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let jobs = captured_mail_jobs(&a).await;
    assert_eq!(jobs.len(), 1);
    let queued = jobs[0]["notification"]["PasswordReset"]["email_address"]
        .as_str()
        .unwrap();
    assert_eq!(queued, submitted);

    let now = a.booted.app.clock.now();
    let message = reset_message(&a, queued, now).await.unwrap();
    assert!(
        message.to.ends_with(&format!("<{address}>")),
        "{}",
        message.to
    );
    let token = link_token(&message);
    let claim = rails_compat::password_reset::verify(&a.booted.app.secrets, &token, now).unwrap();
    assert_eq!(claim.user_id, DAVID);
    assert_eq!(claim.secondary_id, Some(secondary.id));
    assert_eq!(visitor.send(check(&token)).await.status, StatusCode::OK);
}

#[tokio::test]
async fn password_reset_refuses_a_normalized_collision_between_primary_and_secondary() {
    let (a, _) = app(&MAIL).await;
    let address = "Linked@example.test";
    let legacy_primary = "LINKED@example.test";
    let secondary = confirmed_secondary(&a, address).await;
    let now = a.booted.app.clock.now();
    assert!(
        reset_message(&a, "linked@example.test", now)
            .await
            .is_some()
    );

    a.db()
        .write(move |tx| {
            // Legacy or imported rows can predate the cross-table claim check.
            tx.conn().execute(
                "UPDATE users SET email_address=? WHERE id=?",
                rusqlite::params![legacy_primary, JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        reset_message(&a, "linked@example.test", now)
            .await
            .is_none()
    );

    for (input, user_id, secondary_id) in [
        (legacy_primary, JASON, None),
        (address, DAVID, Some(secondary.id)),
    ] {
        let exact = reset_message(&a, input, now).await.unwrap();
        let claim =
            rails_compat::password_reset::verify(&a.booted.app.secrets, &link_token(&exact), now)
                .unwrap();
        assert_eq!(claim.user_id, user_id);
        assert_eq!(claim.secondary_id, secondary_id);
    }
}

#[tokio::test]
async fn password_reset_and_sign_in_require_an_exact_address_in_a_cross_account_collision() {
    let (a, _) = app(&MAIL).await;
    let secondary = "Straße@example.test";
    let primary = "STRASSE@example.test";
    let ambiguous = "straße@example.test";
    confirmed_secondary(&a, secondary).await;
    let digest = new_digest(&a, NEW_PASSWORD).await;
    a.db()
        .write(move |tx| {
            // Import a legacy primary with the same Unicode case fold as David's secondary.
            tx.conn().execute(
                "UPDATE users SET email_address=?, password_digest=? WHERE id=?",
                rusqlite::params![primary, digest.into_string(), JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();

    let mut browser = a.anonymous();
    let missing = browser
        .password_sign_in("unknown@example.test", PASSWORD)
        .await;
    let refused = browser.password_sign_in(ambiguous, PASSWORD).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
    assert_eq!(refused.json(), missing.json());
    let now = a.booted.app.clock.now();
    assert!(reset_message(&a, ambiguous, now).await.is_none());

    for (address, password, user_id) in
        [(secondary, PASSWORD, DAVID), (primary, NEW_PASSWORD, JASON)]
    {
        let reply = a.anonymous().password_sign_in(address, password).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        assert!(
            ["signedIn", "secondFactorRequired"].contains(&reply.json()["kind"].as_str().unwrap())
        );
        let exact = reset_message(&a, address, now).await.unwrap();
        assert!(exact.to.ends_with(&format!("<{address}>")), "{}", exact.to);
        let claim =
            rails_compat::password_reset::verify(&a.booted.app.secrets, &link_token(&exact), now)
                .unwrap();
        assert_eq!(claim.user_id, user_id);
    }
}
