use super::google_api_tests::{self as support, Recorded};
use crate::controllers::presenters::test_support::{Browser, DAVID, Req, TestApp, encode};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use campfire_db::models::google_account::GoogleAccount;
use campfire_kit::{Crypto, RailsCrypto};
use hyper::{Method, StatusCode};
use serde_json::{Value, json};
use std::sync::Arc;
async fn app() -> (TestApp, Arc<Recorded>) {
    let a = TestApp::boot().await.expect("default seed required");
    let r = Recorded::new(vec![]);
    support::install(&a, r.clone()).await;
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            tx.conn().execute("DELETE FROM google_accounts", [])?;
            Ok(())
        })
        .await
        .unwrap();
    super::google_test_support::observe_jobs(&a).await;
    (a, r)
}
pub async fn sudo(a: &TestApp, b: &mut Browser<'_>) {
    let crypto = RailsCrypto::new(a.booted.app.secrets.clone());
    let raw = b
        .cookie_header()
        .split(';')
        .find(|p| p.trim().starts_with("_campfire_session="))
        .map(|p| p.trim().split_once('=').unwrap().1.to_owned());
    let mut value = raw
        .and_then(|s| {
            crypto.decrypt_cookie(
                "_campfire_session",
                &rails_compat::cookies::unescape(&s),
                a.booted.app.clock.now(),
            )
        })
        .unwrap_or(json!({"session_id":"ws14g-connection"}));
    value["sudo_verified_at"] = json!(a.booted.app.clock.now().as_second());
    let cookie = crypto.encrypt_cookie("_campfire_session", &value, None);
    b.absorb_cookie_header(&format!(
        "_campfire_session={}",
        campfire_kit::cookies::escape(&cookie)
    ));
}
async fn start(a: &TestApp, b: &mut Browser<'_>) -> String {
    sudo(a, b).await;
    let reply = b
        .write(Req::new(Method::POST, "/google/connect").form(&[("features[]", "drive")]))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND, "{}", reply.text());
    let url = url::Url::parse(reply.location().unwrap()).unwrap();
    let q = url
        .query_pairs()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(q["prompt"], "consent");
    assert_eq!(q["access_type"], "offline");
    assert!(q["scope"].contains("/drive.file"));
    assert!(!q.contains_key("code_challenge"));
    q["state"].to_string()
}
fn tokens(a: &TestApp, scopes: Option<&str>) -> Value {
    let claim = json!({"iss":"https://accounts.google.com","aud":"test-client-id","exp":a.booted.app.clock.now().as_second()+3600,"email":"david@gmail.test"});
    let mut t = json!({"access_token":"new-access-token","refresh_token":"new-refresh-token","expires_in":3600,"id_token":format!("e30.{}.fixture",URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claim).unwrap()))});
    if let Some(s) = scopes {
        t["scope"] = json!(s);
    }
    t
}
async fn callback(
    b: &mut Browser<'_>,
    state: &str,
) -> crate::controllers::presenters::test_support::Reply {
    b.get(&format!(
        "/google/callback?state={}&code=fixture",
        encode(state)
    ))
    .await
}
#[tokio::test]
async fn google_connection_state_is_one_use_and_sudo_guards_writes() {
    let (a, r) = app().await;
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    assert_eq!(
        b.write(Req::new(Method::POST, "/google/connect"))
            .await
            .location(),
        Some("http://campfire.test/sudo/new")
    );
    let state = start(&a, &mut b).await;
    assert_eq!(
        callback(&mut b, "forged").await.location(),
        Some("http://campfire.test/users/me/profile")
    );
    callback(&mut b, &state).await;
    assert!(r.calls.lock().unwrap().is_empty());
    assert!(
        a.db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .is_none()
    );
    let state = start(&a, &mut b).await;
    b.get(&format!(
        "/google/callback?state={}&error=access_denied",
        encode(&state)
    ))
    .await;
    assert!(r.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn google_connection_saves_encrypted_grant_audit_and_durable_watch_atomically() {
    let (a, r) = app().await;
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    let state = start(&a, &mut b).await;
    r.answer(200, tokens(&a, None));
    assert_eq!(callback(&mut b, &state).await.status, StatusCode::FOUND);
    let account = a
        .db()
        .read(|c| GoogleAccount::for_user(c, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.email, "david@gmail.test");
    assert_eq!(
        account
            .refresh_token(&rails_compat::ar_encryption::ArEncryption::new(
                &a.booted.app.secrets
            ))
            .unwrap()
            .as_deref(),
        Some("new-refresh-token")
    );
    let(rows,audits)=a.db().read(|c|Ok((c.query_row("SELECT count(*) FROM ws14g_emitted_jobs WHERE job_class='Calendar::WatchChannelJob'",[],|r|r.get::<_,i64>(0))?,c.query_row("SELECT count(*) FROM audit_logs WHERE action='google.account.connect'",[],|r|r.get::<_,i64>(0))?))).await.unwrap();
    assert_eq!((rows, audits), (1, 1));
    a.db().write(|tx|{tx.conn().execute("DELETE FROM google_accounts",[])?;tx.conn().execute("DELETE FROM audit_logs",[])?;tx.conn().execute_batch("CREATE TRIGGER reject_connection_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'fixture queue failure'); END;")?;Ok(())}).await.unwrap();
    let state = start(&a, &mut b).await;
    r.answer(200, tokens(&a, None));
    assert_eq!(
        callback(&mut b, &state).await.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(
        a.db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        a.db()
            .read(
                |c| Ok(c.query_row("SELECT count(*) FROM audit_logs", [], |r| r
                    .get::<_, i64>(0))?)
            )
            .await
            .unwrap(),
        0
    );
}
#[tokio::test]
async fn google_connection_without_calendar_stores_grant_without_audit_or_jobs() {
    let (a, r) = app().await;
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    let state = start(&a, &mut b).await;
    r.answer(
        200,
        tokens(&a, Some(campfire_db::models::google_account::DRIVE_SCOPE)),
    );
    callback(&mut b, &state).await;
    assert!(
        !a.db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .unwrap()
            .calendar()
    );
    assert_eq!(
        a.db()
            .read(|c| Ok(c.query_row(
                "SELECT count(*) FROM audit_logs WHERE action='google.account.connect'",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
}
#[tokio::test]
async fn google_connection_disconnect_cleanup_snapshot_and_audit_roll_back_on_queue_failure() {
    let (a, r) = app().await;
    let now = campfire_db::Timestamp::from_jiff(a.booted.app.clock.now());
    support::grant(
        &a,
        DAVID,
        now.since(jiff::SignedDuration::from_hours(1)),
        true,
    )
    .await;
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    sudo(&a, &mut b).await;
    a.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER reject_disconnect_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'fixture queue failure'); END;")?;Ok(())}).await.unwrap();
    r.answer(200, json!({}));
    assert_eq!(
        b.write(Req::new(Method::DELETE, "/google/connection"))
            .await
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(
        a.db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .is_some()
    );
    a.db()
        .write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER reject_disconnect_job")?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        b.write(Req::new(Method::DELETE, "/google/connection"))
            .await
            .status,
        StatusCode::FOUND
    );
    assert!(
        a.db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .is_none()
    );
    let args:String=a.db().read(|c|Ok(c.query_row("SELECT arguments FROM ws14g_emitted_jobs WHERE job_class='Calendar::DisconnectCleanupJob'",[],|r|r.get(0))?)).await.unwrap();
    let args: Value = serde_json::from_str(&args).unwrap();
    assert!(args[1].as_str().is_some());
    assert!(!args.to_string().contains("refresh-token"));
    assert!(
        r.calls
            .lock()
            .unwrap()
            .iter()
            .all(|call| call["path"] == "/revoke")
    );
}

#[tokio::test]
async fn google_profile_reads_calendar_and_login_identity_separately() {
    use crate::integrations::google::sign_in::{Config, SignIn};
    use campfire_db::models::google_identity::GoogleIdentity;
    let (a, r) = app().await;
    // Both protocols use the injected transport; rendering makes no outbound calls.
    a.booted.app.google.install(SignIn::with_client(
        Config {
            client_id: "test-client-id".into(),
            client_secret: "FAKE-google-client-secret".into(),
            domains: vec!["smartdata.net".into()],
        },
        r.clone(),
    ));
    support::grant(
        &a,
        DAVID,
        campfire_db::Timestamp::from_jiff(a.booted.app.clock.now())
            .since(jiff::SignedDuration::from_hours(1)),
        true,
    )
    .await;
    a.db()
        .write(|tx| {
            GoogleIdentity::link_to_user(
                tx,
                json!({"sub":"profile","email":"login@smartdata.net","hd":"smartdata.net"})
                    .as_object()
                    .unwrap(),
                DAVID,
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut b = a.sign_in(DAVID).await;
    let reply = b.get("/users/me/profile").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        reply
            .text()
            .contains("Linked to login@smartdata.net. You can sign in with Google.")
    );
    assert!(reply.text().contains("Connected as david@gmail.test"));
    assert!(reply.text().contains("Drive previews enabled"));
    assert!(reply.text().contains("action=\"/google/connection\""));
    assert!(r.calls.lock().unwrap().is_empty());
}
