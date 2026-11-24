use super::password_reset_tests::{capture_mail_jobs, captured_mail_jobs};
use super::signed_out_contracts_tests::inline_boot;
use crate::controllers::presenters::test_support::{DAVID, KEVIN, Reply, Req, SEED_NOW, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::User;
use campfire_db::user::email_addresses::{Confirmation, EmailAddress};
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
    TestApp::boot_with_clock_and_env(
        Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap())),
        env,
    )
    .await
    .expect("restored default seed")
}

fn add(address: &str) -> Req {
    Req::new(Method::POST, ADDRESSES).json_body(&json!({ "emailAddress": address }))
}

fn confirmation(method: Method, token: &str) -> Req {
    Req::new(method, &format!("{ADDRESSES}/confirmations/{token}"))
}

fn token(a: &TestApp, user_id: i64, email: &str, age: jiff::SignedDuration) -> String {
    email_address_confirmation::token(
        &a.booted.app.secrets,
        user_id,
        email,
        a.booted.app.clock.now() - age,
    )
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
    let invalid = b.write(add("name <a@example.test>")).await;
    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        invalid.json()["error"]["fields"]["emailAddress"],
        json!(["must be a valid email address"])
    );

    let addresses = [
        LINKED.to_owned(),
        primary_of(&a, KEVIN).await,
        primary_of(&a, DAVID).await,
    ];
    let expected: Vec<_> = addresses.iter().map(|address| {
        json!({"notification":{"EmailAddressConfirmation":{"user_id": DAVID, "email_address": address}}})
    }).collect();
    for address in addresses {
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
    let render = |user, address: String| {
        campfire_runtime::mail::email_address_confirmation_message(&a.booted.app, user, address)
    };
    let message = render(DAVID, LINKED.into()).await.unwrap().unwrap();
    assert_eq!(message.to, LINKED);
    let url = message
        .text
        .lines()
        .find(|line| line.starts_with("http"))
        .unwrap();
    let signature = url
        .strip_prefix("http://campfire.test/app/email_addresses/confirm/")
        .unwrap();
    assert_eq!(
        email_address_confirmation::verify(
            &a.booted.app.secrets,
            signature,
            a.booted.app.clock.now()
        ),
        Some(email_address_confirmation::Claim {
            user_id: DAVID,
            email_address: LINKED.into()
        }),
    );
    assert!(
        render(DAVID, primary_of(&a, DAVID).await)
            .await
            .unwrap()
            .is_none()
    );
    link(&a, KEVIN, LINKED).await;
    assert!(
        render(DAVID, LINKED.to_ascii_lowercase())
            .await
            .unwrap()
            .is_none()
    );
    a.db()
        .write(|tx| User::find(tx.conn(), DAVID)?.deactivate(tx))
        .await
        .unwrap();
    assert!(
        render(DAVID, "free@example.test".into())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn confirmation_is_public_idempotent_and_audited() {
    let a = app(MAIL).await;
    let token = token(&a, DAVID, LINKED, jiff::SignedDuration::ZERO);
    let mut phone = a.anonymous();
    let page = phone
        .get(&format!("/app/email_addresses/confirm/{token}"))
        .await;
    assert_eq!(page.status, StatusCode::OK);
    assert_eq!(page.header("referrer-policy"), Some("no-referrer"));
    assert_eq!(inline_boot(&page)["kind"], "signedOut");
    let status = phone.send(confirmation(Method::GET, &token)).await;
    assert_eq!(
        status.json(),
        json!({"emailAddress": LINKED, "requiresSignIn": true})
    );
    let applied = phone.write(confirmation(Method::PUT, &token)).await;
    assert_eq!(applied.status, StatusCode::OK, "{}", applied.text());
    assert_eq!(
        applied.json(),
        json!({"kind": "navigate", "location": "http://campfire.test/app/session/new"})
    );
    assert_eq!(
        inline_boot(&phone.get("/app/session/new").await)["flash"],
        json!({"kind":"notice","message":"Email address confirmed."})
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
    let zero = jiff::SignedDuration::ZERO;
    let valid = token(&a, DAVID, LINKED, zero);
    let foreign = rails_compat::signed_id::avatar_token(&a.booted.app.secrets, DAVID);
    let expired = token(&a, DAVID, LINKED, email_address_confirmation::EXPIRES_IN);
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
async fn removal_and_promotion_require_sudo_and_ownership_and_write_audits() {
    let a = app(MAIL).await;
    let previous_primary = primary_of(&a, DAVID).await;
    let owned = link(&a, DAVID, LINKED).await;
    let foreign = link(&a, KEVIN, "foreign@example.test").await;
    let remove = |id| Req::new(Method::DELETE, &format!("{ADDRESSES}/{id}"));
    let promote = |id| Req::new(Method::POST, &format!("{ADDRESSES}/{id}/primary"));
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

    let promoted = owner.write(promote(owned.id)).await;
    assert_eq!(
        promoted.status,
        StatusCode::NO_CONTENT,
        "{}",
        promoted.text()
    );
    let list = owner.get(ADDRESSES).await.json();
    assert_eq!(list["primary"], LINKED);
    let previous = &list["secondary"][0];
    assert_eq!(previous["emailAddress"], previous_primary);
    assert_ne!(previous["id"], owned.id);
    assert_eq!(
        owner
            .write(remove(previous["id"].as_i64().unwrap()))
            .await
            .status,
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
    assert!(self_changed);
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
