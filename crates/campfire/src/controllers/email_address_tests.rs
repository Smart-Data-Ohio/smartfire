use super::password_reset_tests::{capture_mail_jobs, captured_mail_jobs};
use super::signed_out_contracts_tests::inline_boot;
use crate::controllers::presenters::test_support::{
    Browser, DAVID, KEVIN, Reply, Req, SEED_NOW, TestApp,
};
use axum::http::{Method, StatusCode};
use campfire_db::user::email_addresses::{Confirmation, EmailAddress};
use campfire_db::{PasswordDigest, Timestamp, User, UserChanges, models::audit_log};
use campfire_kit::FrozenClock;
use campfire_mail::jobs::{DeliveryJob, EmailAddressConfirmation, Notification};
use jiff::SignedDuration;
use rails_compat::email_address_confirmation;
use serde_json::{Value, json};
use std::sync::Arc;

const ADDRESSES: &str = "/api/v1/email_addresses";
const LINKED: &str = "Linked@example.test";
const MAIL: &[(&str, &str)] = &[
    ("SMTP_ADDRESS", "smtp.invalid"),
    ("MAILER_FROM", "Smartfire <alerts@example.com>"),
    ("APP_URL", "http://campfire.test"),
];

async fn app(env: &[(&str, &str)]) -> TestApp {
    app_with_clock(env).await.0
}

async fn app_with_clock(env: &[(&str, &str)]) -> (TestApp, Arc<FrozenClock>) {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_clock_and_env(clock.clone(), env)
        .await
        .expect("restored default seed");
    (app, clock)
}

fn add(address: &str) -> Req {
    Req::new(Method::POST, ADDRESSES).json_body(&json!({ "emailAddress": address }))
}

fn confirmation(method: Method, token: &str) -> Req {
    Req::new(method, &format!("{ADDRESSES}/confirmations/{token}"))
}

fn promote(id: i64) -> Req {
    Req::new(Method::POST, &format!("{ADDRESSES}/{id}/primary"))
}

async fn reset_password(a: &TestApp) {
    let digest = PasswordDigest::hash("a new test passphrase".into(), 4)
        .await
        .unwrap();
    a.db()
        .write(move |tx| {
            let mut user = User::find(tx.conn(), DAVID)?;
            campfire_runtime::authentication::reset_password(
                tx,
                &mut user,
                digest,
                &audit_log::Context::default(),
            )
        })
        .await
        .unwrap();
}

async fn write_during_password_reset(a: &TestApp, request: Req) -> Reply {
    let mut owner = a.sign_in(DAVID).await;
    owner.grant_sudo().await;
    capture_mail_jobs(a).await;
    let hold = campfire_api::test_hooks::hold_before_email_address_write(a.db().path(), DAVID);
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let (reply, ()) = tokio::join!(owner.write(request), async {
            hold.reached.wait().await;
            reset_password(a).await;
            hold.release.wait().await;
        });
        reply
    })
    .await
    .expect("the request reached its write after authorization")
}

#[tokio::test]
async fn password_reset_between_authorization_and_add_queues_nothing() {
    let a = app(MAIL).await.without_job_runner().await;
    let reply = write_during_password_reset(&a, add(LINKED)).await;
    assert!(captured_mail_jobs(&a).await.is_empty());
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.text());
    assert_eq!(reply.json()["error"]["_tag"], "Unauthorized");
}

#[tokio::test]
async fn password_reset_between_authorization_and_remove_preserves_the_address() {
    let a = app(MAIL).await.without_job_runner().await;
    let owned = link(&a, DAVID, LINKED).await;
    let request = Req::new(Method::DELETE, &format!("{ADDRESSES}/{}", owned.id));
    let reply = write_during_password_reset(&a, request).await;
    assert!(
        a.db()
            .read(move |conn| EmailAddress::find_owned(conn, DAVID, owned.id))
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.text());
    assert_eq!(reply.json()["error"]["_tag"], "Unauthorized");
}

#[tokio::test]
async fn password_reset_between_authorization_and_promotion_returns_json_unauthorized() {
    let a = app(MAIL).await.without_job_runner().await;
    let previous = primary_of(&a, DAVID).await;
    let owned = link(&a, DAVID, LINKED).await;
    let reply = write_during_password_reset(&a, promote(owned.id)).await;
    assert_eq!(primary_of(&a, DAVID).await, previous);
    assert!(captured_mail_jobs(&a).await.is_empty());
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.text());
    assert_eq!(reply.json()["error"]["_tag"], "Unauthorized");
}

async fn token(a: &TestApp, user_id: i64, email: &str, age: SignedDuration) -> String {
    let (digest, audit_id) = a
        .db()
        .read_snapshot(move |conn| {
            Ok((
                User::find(conn, user_id)?
                    .password_digest
                    .unwrap_or_default(),
                audit_log::AuditLog::latest_id(conn)?,
            ))
        })
        .await
        .unwrap();
    email_address_confirmation::token(
        &a.booted.app.secrets,
        user_id,
        email,
        &digest,
        a.booted.app.clock.now() - age,
        audit_id,
    )
}

async fn confirmation_message(
    a: &TestApp,
    user_id: i64,
    address: &str,
    requested_at: Timestamp,
) -> Option<campfire_mail::outbound::Message> {
    campfire_runtime::mail::email_address_confirmation_message(
        &a.booted.app,
        confirmation_job(a, user_id, address).await,
        requested_at,
    )
    .await
    .unwrap()
}

async fn confirmation_job(a: &TestApp, user_id: i64, address: &str) -> EmailAddressConfirmation {
    let address = address.to_owned();
    a.db()
        .read(move |conn| {
            let user = User::find(conn, user_id)?;
            let audit_id = audit_log::AuditLog::latest_id(conn)?;
            Ok(EmailAddressConfirmation::new(&user, address, audit_id))
        })
        .await
        .unwrap()
}

async fn queued_confirmation(a: &TestApp) -> EmailAddressConfirmation {
    let [job] = <[Value; 1]>::try_from(captured_mail_jobs(a).await).expect("one queued job");
    let job: DeliveryJob = serde_json::from_value(job).unwrap();
    let Notification::EmailAddressConfirmation(confirmation) = job.notification else {
        panic!("expected a confirmation job, got {job:?}");
    };
    confirmation
}

#[tokio::test]
async fn queued_confirmation_does_not_send_after_reset_even_if_the_clock_goes_back() {
    let (a, clock) = app_with_clock(MAIL).await;
    let a = a.without_job_runner().await;
    let mut owner = a.sign_in(DAVID).await;
    owner.grant_sudo().await;
    capture_mail_jobs(&a).await;
    let requested_at = a.db().env().now();
    assert_eq!(owner.write(add(LINKED)).await.status, StatusCode::OK);
    let job = queued_confirmation(&a).await;

    clock.advance(SignedDuration::from_secs(-1));
    reset_password(&a).await;
    clock.advance(SignedDuration::from_secs(2));

    assert!(
        campfire_runtime::mail::email_address_confirmation_message(
            &a.booted.app,
            job,
            requested_at
        )
        .await
        .unwrap()
        .is_none()
    );
}

fn assert_address_snapshot(log: &[String]) {
    let begin = log
        .iter()
        .position(|sql| sql.starts_with("BEGIN"))
        .expect("opens a snapshot");
    let end = log
        .iter()
        .position(|sql| sql.starts_with("ROLLBACK"))
        .expect("closes it");
    let snapshot = &log[begin + 1..end];
    for table in ["users", "user_email_addresses"] {
        assert!(
            snapshot
                .iter()
                .any(|sql| sql.contains(&format!("FROM {table}"))
                    || sql.contains(&format!("FROM \"{table}\""))),
            "missing {table}: {snapshot:?}"
        );
    }
}

#[tokio::test]
async fn list_and_confirmation_delivery_read_both_address_sources_in_a_snapshot() {
    let a = app(MAIL).await.without_job_runner().await;
    let mut owner = a.sign_in(DAVID).await;
    let log = a.db().capture_read_queries();
    assert_eq!(owner.get(ADDRESSES).await.status, StatusCode::OK);
    a.db().stop_capturing_read_queries();
    let list_queries = log.lock().unwrap().clone();

    let log = a.db().capture_read_queries();
    assert!(
        confirmation_message(&a, DAVID, LINKED, a.db().env().now())
            .await
            .is_some()
    );
    a.db().stop_capturing_read_queries();
    let delivery_queries = log.lock().unwrap().clone();
    assert_address_snapshot(&list_queries);
    assert_address_snapshot(&delivery_queries);
}

fn link_token(message: &campfire_mail::outbound::Message) -> &str {
    message
        .text
        .lines()
        .find_map(|line| line.strip_prefix("http://campfire.test/app/email_addresses/confirm/"))
        .expect("confirmation link")
}

async fn primary_of(a: &TestApp, user_id: i64) -> String {
    a.db()
        .read(move |conn| Ok(User::find(conn, user_id)?.email_address.unwrap()))
        .await
        .unwrap()
}

async fn link(a: &TestApp, user_id: i64, address: &'static str) -> EmailAddress {
    a.db()
        .write(move |tx| {
            let user = User::find(tx.conn(), user_id)?;
            assert_eq!(
                EmailAddress::confirm(tx, &user, address)?,
                Confirmation::Added
            );
            Ok(EmailAddress::by_address(tx.conn(), address)?.unwrap().1)
        })
        .await
        .unwrap()
}

fn assert_sudo(reply: Reply) {
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());
    assert_eq!(reply.json()["error"]["_tag"], "SudoRequired");
}

#[tokio::test]
async fn request_is_sudo_gated_uniform_and_enqueues_once_per_valid_address() {
    let a = app(MAIL).await.without_job_runner().await;
    assert_eq!(
        a.anonymous().get(ADDRESSES).await.status,
        StatusCode::UNAUTHORIZED
    );
    let mut b = a.sign_in(DAVID).await;
    capture_mail_jobs(&a).await;
    assert_sudo(b.write(add(LINKED)).await);
    b.grant_sudo().await;
    for address in ["name <a@example.test>", "a..b@example.com"] {
        let invalid = b.write(add(address)).await;
        assert_eq!(
            invalid.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{address}: {}",
            invalid.text()
        );
        assert_eq!(
            invalid.json()["error"]["fields"]["emailAddress"],
            json!(["isn't valid"])
        );
    }

    let addresses = [
        LINKED.to_owned(),
        primary_of(&a, KEVIN).await,
        primary_of(&a, DAVID).await,
    ];
    let mut expected = Vec::new();
    for address in addresses {
        let job = confirmation_job(&a, DAVID, &address).await;
        expected.push(
            serde_json::to_value(DeliveryJob {
                notification: Notification::EmailAddressConfirmation(job),
            })
            .unwrap(),
        );
        let reply = b.write(add(&format!(" {address} "))).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        assert_eq!(
            reply.json(),
            json!({"kind":"confirmationSent","emailAddress":address})
        );
    }
    assert_eq!(captured_mail_jobs(&a).await, expected);
    for _ in 0..2 {
        assert_eq!(b.write(add(LINKED)).await.status, StatusCode::OK);
    }
    let limited = b.write(add(LINKED)).await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.json()["error"]["_tag"], "RateLimited");
    assert_eq!(captured_mail_jobs(&a).await.len(), 5);
}

#[tokio::test]
async fn mail_configuration_controls_requests_and_delivery_checks_current_ownership() {
    let unconfigured = app(&[]).await;
    let mut b = unconfigured.sign_in(DAVID).await;
    assert_eq!(b.get(ADDRESSES).await.json()["canAdd"], false);
    assert_eq!(b.write(add(LINKED)).await.status, StatusCode::NOT_FOUND);

    let a = app(MAIL).await;
    let requested_at = a.db().env().now();
    assert!(
        confirmation_message(
            &a,
            DAVID,
            LINKED,
            requested_at.ago(email_address_confirmation::EXPIRES_IN),
        )
        .await
        .is_none()
    );
    let message = confirmation_message(&a, DAVID, LINKED, requested_at)
        .await
        .unwrap();
    assert_eq!(message.to, LINKED);
    let claim = email_address_confirmation::verify(
        &a.booted.app.secrets,
        link_token(&message),
        a.booted.app.clock.now(),
    )
    .expect("mail carries a confirmation claim");
    assert_eq!(claim.user_id, DAVID);
    assert_eq!(claim.email_address, LINKED);
    assert_eq!(claim.issued_at, requested_at.jiff());
    assert!(
        confirmation_message(&a, DAVID, &primary_of(&a, DAVID).await, requested_at)
            .await
            .is_none()
    );
    link(&a, KEVIN, LINKED).await;
    assert!(
        confirmation_message(&a, DAVID, &LINKED.to_ascii_lowercase(), requested_at)
            .await
            .is_none()
    );
    a.db()
        .write(|tx| User::find(tx.conn(), DAVID)?.deactivate(tx))
        .await
        .unwrap();
    assert!(
        confirmation_message(&a, DAVID, "free@example.test", requested_at)
            .await
            .is_none()
    );
}

#[tokio::test]
async fn confirmation_is_public_idempotent_and_audited() {
    let a = app(MAIL).await;
    let token = token(&a, DAVID, LINKED, SignedDuration::ZERO).await;
    let mut phone = a.anonymous();
    let page = phone
        .get(&format!("/app/email_addresses/confirm/{token}"))
        .await;
    assert_eq!(page.status, StatusCode::OK);
    assert_eq!(page.header("referrer-policy"), Some("no-referrer"));
    assert_eq!(inline_boot(&page)["kind"], "signedOut");
    assert_eq!(inline_boot(&page)["signedIn"], false);
    let status = phone.send(confirmation(Method::GET, &token)).await;
    assert_eq!(status.status, StatusCode::OK, "{}", status.text());
    assert_eq!(status.json()["emailAddress"], LINKED);
    assert_eq!(status.json()["requiresSignIn"], true);
    assert_eq!(status.json()["accountMismatch"], false);
    let applied = phone.write(confirmation(Method::PUT, &token)).await;
    assert_eq!(applied.status, StatusCode::OK, "{}", applied.text());
    assert_eq!(
        applied.json(),
        json!({"kind": "navigate", "location": "http://campfire.test/app/session/new"})
    );
    assert_eq!(
        inline_boot(&phone.get("/app/session/new").await)["flash"],
        json!({"kind":"notice","message":"Your email address is linked. You can use it to sign in."})
    );

    let mut owner = a.sign_in(DAVID).await;
    assert_eq!(
        owner.send(confirmation(Method::GET, &token)).await.json()["requiresSignIn"],
        false
    );
    assert_eq!(
        owner.write(confirmation(Method::PUT, &token)).await.json(),
        json!({"kind":"navigate","location":"http://campfire.test/app/settings"}),
    );
    let list = owner.get(ADDRESSES).await.json();
    assert_eq!(list["primary"], primary_of(&a, DAVID).await);
    assert_eq!(list["canAdd"], true);
    let entries = list["secondary"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["emailAddress"], LINKED);
    assert!(
        entries[0]["createdAt"]
            .as_str()
            .unwrap()
            .parse::<jiff::Timestamp>()
            .is_ok()
    );
    assert_eq!(
        audits(&a).await,
        [json!(["user.email_address.add", DAVID, DAVID])]
    );
}

#[tokio::test]
async fn confirmation_refuses_bad_links_and_an_address_claimed_in_the_meantime() {
    let a = app(MAIL).await;
    let mut phone = a.anonymous();
    let zero = SignedDuration::ZERO;
    let valid = token(&a, DAVID, LINKED, zero).await;
    let foreign = rails_compat::signed_id::avatar_token(&a.booted.app.secrets, DAVID);
    let expired = token(&a, DAVID, LINKED, email_address_confirmation::EXPIRES_IN).await;
    for token in [foreign, expired, format!("{valid}0")] {
        for method in [Method::GET, Method::PUT] {
            let reply = phone.write(confirmation(method, &token)).await;
            assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.text());
            assert_eq!(reply.json()["kind"], "error");
            assert!(reply.json()["fieldErrors"]["base"].is_array());
        }
    }
    link(&a, KEVIN, LINKED).await;
    let reply = phone.write(confirmation(Method::PUT, &valid)).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reply.json(),
        json!({
            "kind":"error",
            "fieldErrors":{"emailAddress":["This email address is already linked to another account."]}
        })
    );
    assert!(audits(&a).await.is_empty());
}

#[tokio::test]
async fn confirmation_names_the_account_and_refuses_a_different_viewer() {
    let a = app(MAIL).await;
    let name = "D. Example";
    a.db()
        .write(move |tx| {
            User::find(tx.conn(), DAVID)?.update(
                tx,
                UserChanges {
                    nickname: Some(Some(name.into())),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let mail = confirmation_message(&a, DAVID, LINKED, a.db().env().now())
        .await
        .unwrap();
    let sentence = format!("Someone asked to link {LINKED} to {name}'s Smartfire account.");
    assert!(mail.text.contains(&sentence), "{}", mail.text);
    assert!(mail.html.contains(&sentence), "{}", mail.html);
    let token = link_token(&mail);

    let mut other = a.sign_in(KEVIN).await;
    let page = other
        .get(&format!("/app/email_addresses/confirm/{token}"))
        .await;
    assert_eq!(inline_boot(&page)["signedIn"], true);
    let shown = other.send(confirmation(Method::GET, token)).await;
    assert_eq!(shown.status, StatusCode::OK, "{}", shown.text());
    assert_eq!(
        shown.json(),
        json!({"emailAddress": LINKED, "accountName": name, "requiresSignIn": true, "accountMismatch": true})
    );
    let reply = other.write(confirmation(Method::PUT, token)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        reply.json(),
        json!({"kind":"error","fieldErrors":{"base":["You're signed in to a different account. Sign out, then open this link again."]}})
    );
    let mut owner = a.sign_in(DAVID).await;
    assert_eq!(owner.get(ADDRESSES).await.json()["secondary"], json!([]));
    assert!(audits(&a).await.is_empty());
    assert_eq!(
        owner.write(confirmation(Method::PUT, token)).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn email_address_removal_retires_links_and_jobs_even_when_the_clock_goes_back() {
    let (a, clock) = app_with_clock(MAIL).await;
    let a = a.without_job_runner().await;
    let requested_at = a.db().env().now();
    let token = token(&a, DAVID, LINKED, SignedDuration::ZERO).await;
    let mut owner = a.sign_in(DAVID).await;
    owner.grant_sudo().await;
    assert_eq!(
        owner.write(confirmation(Method::PUT, &token)).await.status,
        StatusCode::OK
    );
    let listed = owner.get(ADDRESSES).await.json();
    let id = listed["secondary"][0]["id"].as_i64().unwrap();
    capture_mail_jobs(&a).await;
    assert_eq!(
        owner.write(add(&LINKED.to_ascii_lowercase())).await.status,
        StatusCode::OK
    );
    let job = queued_confirmation(&a).await;

    clock.advance(SignedDuration::from_secs(-1));
    let delete = Req::new(Method::DELETE, &format!("{ADDRESSES}/{id}"));
    assert_eq!(owner.write(delete).await.status, StatusCode::NO_CONTENT);
    let mut phone = a.anonymous();
    assert_invalid_confirmation(&mut phone, &token).await;
    assert!(
        campfire_runtime::mail::email_address_confirmation_message(
            &a.booted.app,
            job,
            requested_at
        )
        .await
        .unwrap()
        .is_none()
    );
    assert_eq!(owner.get(ADDRESSES).await.json()["secondary"], json!([]));

    clock.advance(SignedDuration::from_secs(1));
    let fresh = confirmation_message(&a, DAVID, LINKED, a.db().env().now())
        .await
        .unwrap();
    assert_eq!(
        phone
            .write(confirmation(Method::PUT, link_token(&fresh)))
            .await
            .status,
        StatusCode::OK
    );
    assert_invalid_confirmation(&mut phone, &token).await;
}

#[tokio::test]
async fn email_address_release_by_another_owner_retires_links_and_jobs() {
    let (a, clock) = app_with_clock(MAIL).await;
    let a = a.without_job_runner().await;
    let mut owner = a.sign_in(DAVID).await;
    owner.grant_sudo().await;
    capture_mail_jobs(&a).await;
    let requested_at = a.db().env().now();
    assert_eq!(
        owner.write(add("strasse@example.test")).await.status,
        StatusCode::OK
    );
    let job = queued_confirmation(&a).await;
    let message = campfire_runtime::mail::email_address_confirmation_message(
        &a.booted.app,
        job.clone(),
        requested_at,
    )
    .await
    .unwrap()
    .unwrap();

    clock.advance(SignedDuration::from_secs(1));
    let address = link(&a, KEVIN, "straße@example.test").await;
    let mut other = a.sign_in(KEVIN).await;
    other.grant_sudo().await;
    let delete = Req::new(Method::DELETE, &format!("{ADDRESSES}/{}", address.id));
    assert_eq!(other.write(delete).await.status, StatusCode::NO_CONTENT);

    assert_invalid_confirmation(&mut a.anonymous(), link_token(&message)).await;
    assert!(
        campfire_runtime::mail::email_address_confirmation_message(
            &a.booted.app,
            job,
            requested_at
        )
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn email_address_deactivation_retires_links_and_jobs_after_unban() {
    let a = app(MAIL).await.without_job_runner().await;
    let mut owner = a.sign_in(KEVIN).await;
    owner.grant_sudo().await;
    capture_mail_jobs(&a).await;
    let requested_at = a.db().env().now();
    assert_eq!(owner.write(add(LINKED)).await.status, StatusCode::OK);
    let job = queued_confirmation(&a).await;
    let message = campfire_runtime::mail::email_address_confirmation_message(
        &a.booted.app,
        job.clone(),
        requested_at,
    )
    .await
    .unwrap()
    .unwrap();

    a.db()
        .write(|tx| User::find(tx.conn(), KEVIN)?.deactivate(tx))
        .await
        .unwrap();
    let mut admin = a.sign_in(DAVID).await;
    admin.grant_sudo().await;
    let unban = admin
        .write(Req::new(
            Method::DELETE,
            &format!("/api/v1/people/{KEVIN}/ban"),
        ))
        .await;
    assert_eq!(unban.status, StatusCode::OK, "{}", unban.text());
    assert!(
        a.db()
            .read(|conn| Ok(User::find(conn, KEVIN)?.is_active()))
            .await
            .unwrap()
    );

    assert_invalid_confirmation(&mut a.anonymous(), link_token(&message)).await;
    assert!(
        campfire_runtime::mail::email_address_confirmation_message(
            &a.booted.app,
            job,
            requested_at
        )
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn email_address_confirmation_job_without_an_audit_id_is_skipped() {
    let a = app(MAIL).await;
    let mut data = serde_json::to_value(confirmation_job(&a, DAVID, LINKED).await).unwrap();
    assert!(
        data.as_object_mut()
            .unwrap()
            .remove("after_audit_id")
            .is_some()
    );
    let old_job = serde_json::from_value(data).unwrap();

    let delivery = campfire_runtime::mail::email_address_confirmation_message(
        &a.booted.app,
        old_job,
        a.db().env().now(),
    )
    .await
    .unwrap();
    assert!(delivery.is_none());
}

async fn session_count(a: &TestApp) -> i64 {
    a.db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT count(*) FROM sessions WHERE user_id=?",
                [DAVID],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap()
}

async fn remove_secondary(db: campfire_db::Database, id: i64) {
    db.write(move |tx| {
        assert!(EmailAddress::remove(tx, DAVID, id)?.is_some());
        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn email_address_removed_after_password_check_starts_no_session() {
    use campfire_runtime::authentication::test_hooks::BEFORE_SESSION_WRITE;
    use std::cell::RefCell;

    let a = app(&[]).await;
    a.db()
        .write(|tx| User::find(tx.conn(), DAVID)?.reset_two_factor(tx))
        .await
        .unwrap();
    let linked = link(&a, DAVID, LINKED).await;
    let before = session_count(&a).await;
    let reply = BEFORE_SESSION_WRITE
        .scope(
            RefCell::new(Some(Box::pin(remove_secondary(a.db().clone(), linked.id)))),
            a.anonymous().password_sign_in(LINKED, "secret123456"),
        )
        .await;
    assert_eq!(
        reply.json(),
        json!({"kind":"navigate","location":"http://campfire.test/app/session/new"})
    );
    assert_eq!(session_count(&a).await, before);
}

#[tokio::test]
async fn email_address_removed_before_second_factor_write_starts_no_session() {
    use campfire_runtime::authentication::test_hooks::BEFORE_SESSION_WRITE;
    use std::cell::RefCell;

    let a = app(&[]).await;
    let linked = link(&a, DAVID, LINKED).await;
    let mut browser = a.anonymous();
    let started = browser.password_sign_in(LINKED, "secret123456").await;
    assert_eq!(started.json()["kind"], "secondFactorRequired");
    let encryption = a.booted.app.ar_encryption.clone();
    let secret = a
        .db()
        .read(move |conn| {
            campfire_db::TwoFactorCredential::for_user(conn, DAVID)?
                .expect("David has two-step sign-in")
                .secret(&encryption)
        })
        .await
        .unwrap();
    let code = rails_compat::totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    let before = session_count(&a).await;

    let reply = BEFORE_SESSION_WRITE
        .scope(
            RefCell::new(Some(Box::pin(remove_secondary(a.db().clone(), linked.id)))),
            browser.answer_challenge(&code, true),
        )
        .await;
    assert_eq!(
        reply.json(),
        json!({"kind":"navigate","location":"http://campfire.test/app/session/new"})
    );
    assert_eq!(session_count(&a).await, before);
    assert!(
        browser.answer_challenge(&code, true).await.json()["location"]
            .as_str()
            .unwrap()
            .ends_with("/app/session/new")
    );
}

#[tokio::test]
async fn email_address_confirmation_checks_the_ten_secondary_limit() {
    let a = app(MAIL).await.without_job_runner().await;
    let mut owner = a.sign_in(DAVID).await;
    owner.grant_sudo().await;
    capture_mail_jobs(&a).await;
    assert_eq!(owner.write(add(LINKED)).await.status, StatusCode::OK);
    let candidate = token(&a, DAVID, LINKED, SignedDuration::ZERO).await;
    let tenth = token(&a, DAVID, "tenth@example.test", SignedDuration::ZERO).await;
    a.db()
        .write(|tx| {
            let user = User::find(tx.conn(), DAVID)?;
            for n in 1..=9 {
                assert_eq!(
                    EmailAddress::confirm(tx, &user, &format!("linked-{n}@example.test"))?,
                    Confirmation::Added,
                );
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut phone = a.anonymous();
    for _ in 0..2 {
        assert_eq!(
            phone.write(confirmation(Method::PUT, &tenth)).await.status,
            StatusCode::OK
        );
    }
    let reply = phone.write(confirmation(Method::PUT, &candidate)).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert_eq!(
        reply.json()["fieldErrors"]["base"],
        json!(["You can link up to 10 additional email addresses. Remove one and try again."]),
    );
    assert_eq!(
        owner.write(add("extra@example.test")).await.status,
        StatusCode::OK
    );
    let list = owner.get(ADDRESSES).await.json();
    let secondaries = list["secondary"].as_array().unwrap();
    assert_eq!(secondaries.len(), 10);
    let id = secondaries[0]["id"].as_i64().unwrap();
    let delete = Req::new(Method::DELETE, &format!("{ADDRESSES}/{id}"));
    assert_eq!(owner.write(delete).await.status, StatusCode::NO_CONTENT);
    assert_eq!(
        phone
            .write(confirmation(Method::PUT, &candidate))
            .await
            .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn setting_a_password_retires_a_passwordless_confirmation_and_queued_delivery() {
    let (a, clock) = app_with_clock(MAIL).await;
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET password_digest=NULL WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    let requested_at = a.db().env().now();
    let message = confirmation_message(&a, DAVID, LINKED, requested_at)
        .await
        .unwrap();
    let token = link_token(&message);
    let mut phone = a.anonymous();
    assert_eq!(
        phone.send(confirmation(Method::GET, token)).await.status,
        StatusCode::OK
    );

    clock.advance(SignedDuration::from_secs(1));
    let mut owner = a.sign_in(DAVID).await;
    let reply = owner
        .write(
            Req::new(Method::PATCH, "/api/v1/settings/profile")
                .json_body(&json!({"password": "   "})),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_invalid_confirmation(&mut phone, token).await;
    assert!(
        confirmation_message(&a, DAVID, LINKED, requested_at)
            .await
            .is_none()
    );

    clock.advance(SignedDuration::from_secs(1));
    let fresh = confirmation_message(&a, DAVID, LINKED, a.db().env().now())
        .await
        .unwrap();
    assert_eq!(
        phone
            .write(confirmation(Method::PUT, link_token(&fresh)))
            .await
            .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn put_confirmation_refuses_a_deactivated_account() {
    let a = app(MAIL).await;
    let token = token(&a, DAVID, LINKED, SignedDuration::ZERO).await;
    a.db()
        .write(|tx| User::find(tx.conn(), DAVID)?.deactivate(tx))
        .await
        .unwrap();
    assert_invalid_confirmation(&mut a.anonymous(), &token).await;
    assert!(audits(&a).await.is_empty());
}

async fn assert_invalid_confirmation(phone: &mut Browser<'_>, token: &str) {
    for method in [Method::GET, Method::PUT] {
        let reply = phone.write(confirmation(method, token)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.text());
        assert!(reply.json()["fieldErrors"]["base"].is_array());
    }
}

#[tokio::test]
async fn changing_a_promoted_primary_retires_its_earlier_confirmation_and_queued_mail() {
    let (a, clock) = app_with_clock(MAIL).await;
    let a = a.without_job_runner().await;
    let mut owner = a.sign_in(DAVID).await;
    owner.grant_sudo().await;
    capture_mail_jobs(&a).await;
    let requested_at = a.db().env().now();
    let address = LINKED.to_ascii_lowercase();
    assert_eq!(owner.write(add(&address)).await.status, StatusCode::OK);
    let job = queued_confirmation(&a).await;
    let token = token(&a, DAVID, LINKED, SignedDuration::ZERO).await;
    let mut phone = a.anonymous();
    assert_eq!(
        phone.write(confirmation(Method::PUT, &token)).await.status,
        StatusCode::OK
    );
    let id = owner.get(ADDRESSES).await.json()["secondary"][0]["id"]
        .as_i64()
        .unwrap();

    clock.advance(SignedDuration::from_secs(1));
    assert_eq!(
        owner.write(promote(id)).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        phone.write(confirmation(Method::PUT, &token)).await.status,
        StatusCode::OK
    );
    clock.advance(SignedDuration::from_secs(1));
    let reply = owner
        .write(
            Req::new(Method::PATCH, "/api/v1/settings/profile").json_body(&json!({
                "emailAddress": "new-primary@example.test",
                "currentPassword": "secret123456",
            })),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());

    let delivery = campfire_runtime::mail::email_address_confirmation_message(
        &a.booted.app,
        job,
        requested_at,
    )
    .await
    .unwrap();
    let replay = phone.write(confirmation(Method::PUT, &token)).await;
    assert_eq!(
        (delivery.is_none(), replay.status),
        (true, StatusCode::BAD_REQUEST)
    );
    assert_invalid_confirmation(&mut phone, &token).await;
    assert_eq!(owner.get(ADDRESSES).await.json()["secondary"], json!([]));
}

#[tokio::test]
async fn promotion_releases_a_legacy_mailbox_without_queuing_or_sending_confirmation() {
    let a = app(MAIL).await.without_job_runner().await;
    let victim = primary_of(&a, KEVIN).await;
    let legacy = format!("X <{victim}>");
    let stored = legacy.clone();
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
    let owned = link(&a, DAVID, LINKED).await;
    let mut owner = a.sign_in(DAVID).await;
    owner.grant_sudo().await;
    capture_mail_jobs(&a).await;
    let reply = owner.write(promote(owned.id)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    assert_eq!(primary_of(&a, DAVID).await, LINKED);
    assert_eq!(primary_of(&a, KEVIN).await, victim);

    let queued = captured_mail_jobs(&a).await;
    let delivery = confirmation_message(&a, DAVID, &legacy, a.db().env().now()).await;
    assert_eq!((queued.len(), delivery.is_none()), (0, true));
}

#[tokio::test]
async fn promotion_releases_and_emails_the_old_primary_without_stamping_it() {
    let a = app(MAIL).await.without_job_runner().await;
    let previous_primary = primary_of(&a, DAVID).await;
    let owned = link(&a, DAVID, LINKED).await;
    let foreign = link(&a, KEVIN, "foreign@example.test").await;
    let removable = link(&a, DAVID, "removable@example.test").await;
    let remove = |id| Req::new(Method::DELETE, &format!("{ADDRESSES}/{id}"));
    let mut owner = a.sign_in(DAVID).await;
    assert_sudo(owner.write(remove(owned.id)).await);
    assert_sudo(owner.write(promote(owned.id)).await);
    owner.grant_sudo().await;
    assert_eq!(
        owner.write(remove(foreign.id)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        owner.write(promote(foreign.id)).await.status,
        StatusCode::NOT_FOUND
    );

    capture_mail_jobs(&a).await;
    let promoted = owner.write(promote(owned.id)).await;
    assert_eq!(
        promoted.status,
        StatusCode::NO_CONTENT,
        "{}",
        promoted.text()
    );
    let list = owner.get(ADDRESSES).await.json();
    assert_eq!(list["primary"], LINKED);
    assert_eq!(list["secondary"].as_array().unwrap().len(), 1);
    assert_eq!(list["secondary"][0]["id"], removable.id);
    let job = queued_confirmation(&a).await;
    assert_eq!(job.user_id, DAVID);
    assert_eq!(job.email_address, previous_primary);
    assert_eq!(
        owner.write(remove(removable.id)).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(owner.get(ADDRESSES).await.json()["secondary"], json!([]));
    assert_eq!(
        audits(&a).await,
        [
            json!(["user.email.change", DAVID, DAVID]),
            json!(["user.email_address.remove", DAVID, DAVID]),
        ],
    );
    let self_changed = a
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT email_self_changed_at IS NOT NULL FROM users WHERE id=?",
                [DAVID],
                |row| row.get::<_, bool>(0),
            )?)
        })
        .await
        .unwrap();
    assert!(!self_changed);

    let mail = campfire_runtime::mail::email_address_confirmation_message(
        &a.booted.app,
        job,
        a.db().env().now(),
    )
    .await
    .unwrap()
    .expect("the old primary receives a confirmation link");
    let relink = link_token(&mail);
    assert_eq!(
        a.anonymous()
            .write(confirmation(Method::PUT, relink))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        owner.get(ADDRESSES).await.json()["secondary"][0]["emailAddress"],
        previous_primary
    );
}

#[tokio::test]
async fn profile_refuses_owned_and_other_members_addresses() {
    let a = app(MAIL).await;
    let previous_primary = primary_of(&a, DAVID).await;
    let own = link(&a, DAVID, LINKED).await;
    let other = link(&a, KEVIN, "other@example.test").await;
    let other_primary = primary_of(&a, KEVIN).await;
    let mut owner = a.sign_in(DAVID).await;
    for (address, problem) in [
        (
            LINKED,
            "This address is already linked to your account. Use Make primary under Email addresses.",
        ),
        (&other.email_address, "is already taken"),
        (&other_primary, "is already taken"),
    ] {
        let reply = owner
            .write(
                Req::new(Method::PATCH, "/api/v1/settings/profile").json_body(
                    &json!({"emailAddress": address, "currentPassword": "secret123456"}),
                ),
            )
            .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert_eq!(
            reply.json()["error"]["fields"]["emailAddress"],
            json!([problem])
        );
    }
    let list = owner.get(ADDRESSES).await.json();
    assert_eq!(list["primary"], previous_primary);
    assert_eq!(list["secondary"][0]["id"], own.id);
    assert!(audits(&a).await.is_empty());
}

#[tokio::test]
async fn profile_refuses_a_case_variant_of_its_own_secondary_with_make_primary_guidance() {
    let a = app(MAIL).await;
    let previous = primary_of(&a, DAVID).await;
    let linked = link(&a, DAVID, LINKED).await;
    let mut owner = a.sign_in(DAVID).await;
    let reply = owner
        .write(
            Req::new(Method::PATCH, "/api/v1/settings/profile").json_body(&json!({
                "emailAddress": LINKED.to_ascii_uppercase(),
                "currentPassword": "secret123456",
            })),
        )
        .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert_eq!(
        reply.json()["error"]["fields"]["emailAddress"],
        json!([
            "This address is already linked to your account. Use Make primary under Email addresses."
        ]),
    );
    let list = owner.get(ADDRESSES).await.json();
    assert_eq!(list["primary"], previous);
    assert_eq!(list["secondary"][0]["id"], linked.id);
    assert!(audits(&a).await.is_empty());
}

#[tokio::test]
async fn email_address_promotion_refuses_unicode_equivalent_legacy_primaries() {
    let a = app(MAIL).await.without_job_runner().await;
    let previous = "strasse@example.test";
    a.db()
        .write(move |tx| {
            // Legacy rows predate checks for equivalent spellings.
            for (id, email) in [(DAVID, previous), (KEVIN, "straße@example.test")] {
                tx.conn().execute(
                    "UPDATE users SET email_address=? WHERE id=?",
                    rusqlite::params![email, id],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let linked = link(&a, DAVID, LINKED).await;
    capture_mail_jobs(&a).await;
    let mut owner = a.sign_in(DAVID).await;
    owner.grant_sudo().await;
    let reply = owner
        .write(Req::new(
            Method::POST,
            &format!("{ADDRESSES}/{}/primary", linked.id),
        ))
        .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert!(reply.json()["error"]["fields"]["emailAddress"].is_array());
    let list = owner.get(ADDRESSES).await.json();
    assert_eq!(list["primary"], previous);
    assert_eq!(list["secondary"][0]["id"], linked.id);
    assert!(captured_mail_jobs(&a).await.is_empty());
    assert!(audits(&a).await.is_empty());
}

#[tokio::test]
async fn a_member_without_a_primary_can_promote() {
    let a = app(MAIL).await;
    let linked = link(&a, DAVID, LINKED).await;
    a.db()
        .write(|tx| {
            User::find(tx.conn(), DAVID)?.update(
                tx,
                campfire_db::UserChanges {
                    email_address: Some(None),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let mut owner = a.sign_in(DAVID).await;
    owner.grant_sudo().await;
    let reply = owner
        .write(Req::new(
            Method::POST,
            &format!("{ADDRESSES}/{}/primary", linked.id),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    assert_eq!(
        owner.get(ADDRESSES).await.json(),
        json!({"primary": LINKED, "secondary": [], "canAdd": true})
    );
}

async fn audits(a: &TestApp) -> Vec<Value> {
    a.db()
        .read(|conn| {
            let mut query = conn.prepare(
                "SELECT action, actor_id, target_id FROM audit_logs WHERE action IN
            ('user.email.change','user.email_address.add','user.email_address.remove') ORDER BY id",
            )?;
            Ok(query
                .query_map([], |row| {
                    Ok(json!([
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?
                    ]))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
        .unwrap()
}
