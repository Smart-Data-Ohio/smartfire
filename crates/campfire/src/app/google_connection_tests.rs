use super::google_api_tests::{self as support, Recorded};
use crate::controllers::presenters::test_support::{Browser, DAVID, Req, TestApp, encode};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use campfire_db::models::google_account::GoogleAccount;
use campfire_kit::{Crypto, RailsCrypto};
use hyper::{Method, StatusCode};
use serde_json::{Value, json};
use std::sync::Arc;
async fn app() -> (TestApp, Arc<Recorded>) {
    let a = TestApp::boot_without_periodic()
        .await
        .expect("default seed required");
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
    answer_calendar_jobs(&a, &r).await;
    (a, r)
}
/// The normal durable runner executes the callback's SyncEntry jobs. Give those
/// consumers their own Rails-recorded replies so they cannot take a /token reply.
async fn answer_calendar_jobs(a: &TestApp, r: &Recorded) {
    let inserted = support::vectors()["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "insert")
        .unwrap()["result"]
        .clone();
    let events = a
        .db()
        .read(|c| {
            Ok(
                c.prepare("SELECT event_id FROM event_attendances WHERE user_id=?")?
                    .query_map([DAVID], |r| r.get::<_, i64>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?,
            )
        })
        .await
        .unwrap();
    for _ in 0..16 {
        r.answer_for(
            Method::POST,
            crate::integrations::google::api::EVENTS,
            200,
            inserted.clone(),
        );
        for event_id in &events {
            let id = campfire_db::models::google_entry::google_id(*event_id, DAVID);
            r.answer_for(
                Method::PUT,
                &format!("{}/{id}", crate::integrations::google::api::EVENTS),
                200,
                inserted.clone(),
            );
        }
    }
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
        b.get("/users/me/profile")
            .await
            .text()
            .contains("Calendar permission was not granted. Reconnect to publish events.")
    );
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
    assert_eq!(
        a.db()
            .read(|c| Ok(
                c.query_row("SELECT count(*) FROM ws14g_emitted_jobs", [], |r| r
                    .get::<_, i64>(0))?
            ))
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

#[tokio::test]
async fn google_connection_authorize_parameters_configuration_and_sign_in_guards() {
    let (a, r) = app().await;
    let mut anonymous = a.anonymous();
    anonymous.get("/session/new").await;
    assert_eq!(
        anonymous
            .write(Req::new(Method::POST, "/google/connect"))
            .await
            .location(),
        Some("http://campfire.test/session/new")
    );
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    sudo(&a, &mut b).await;
    for (fields, drive) in [
        (vec![], false),
        (vec![("features", "drive")], false),
        (vec![("features[]", "other")], false),
        (vec![("features[]", "drive")], true),
    ] {
        let reply = b
            .write(Req::new(Method::POST, "/google/connect").form(&fields))
            .await;
        assert_eq!(reply.status, StatusCode::FOUND);
        let url = url::Url::parse(reply.location().unwrap()).unwrap();
        assert_eq!(url.host_str(), Some("accounts.google.com"));
        let q = url
            .query_pairs()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(q["client_id"], "test-client-id");
        assert_eq!(q["redirect_uri"], "http://campfire.test/google/callback");
        assert_eq!(q["response_type"], "code");
        assert_eq!(q["access_type"], "offline");
        assert_eq!(q["prompt"], "consent");
        let scope = if drive {
            format!(
                "openid email {} {}",
                campfire_db::models::google_account::CALENDAR_SCOPE,
                campfire_db::models::google_account::DRIVE_SCOPE
            )
        } else {
            format!(
                "openid email {}",
                campfire_db::models::google_account::CALENDAR_SCOPE
            )
        };
        assert_eq!(q["scope"], scope);
        for key in ["include_granted_scopes", "code_challenge", "nonce"] {
            assert!(!q.contains_key(key));
        }
        let raw = rails_compat::app_verifier(&a.booted.app.secrets, "google_oauth_state")
            .verify(&q["state"], None, a.booted.app.clock.now())
            .unwrap();
        let raw = raw.as_str().unwrap();
        assert_eq!(raw.len(), 32);
        assert!(raw.bytes().all(|c| c.is_ascii_hexdigit()));
    }
    let mut config = support::config();
    config.client_id.clear();
    a.booted
        .app
        .google
        .install_api(crate::integrations::google::api::Api::new(
            config,
            r.clone(),
        ));
    assert_eq!(
        b.write(Req::new(Method::POST, "/google/connect"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(callback(&mut b, "x").await.status, StatusCode::NOT_FOUND);
    assert_eq!(
        b.write(Req::new(Method::DELETE, "/google/connection"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert!(r.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn google_connection_failed_exchanges_and_invalid_id_tokens_never_store_grants() {
    let (a, r) = app().await;
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    let state = start(&a, &mut b).await;
    assert_eq!(
        callback(&mut b, "bogus").await.location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert!(
        b.get("/users/me/profile")
            .await
            .text()
            .contains("Google connection expired. Try again.")
    );
    callback(&mut b, &state).await;
    assert!(r.calls.lock().unwrap().is_empty());
    for case in ["transport", "exchange", "missing", "other-client"] {
        let state = start(&a, &mut b).await;
        match case {
            "transport" => r.fail_next(),
            "exchange" => r.answer(400, json!({"error":"invalid_grant"})),
            "missing" => {
                let mut t = tokens(&a, None);
                t.as_object_mut().unwrap().remove("id_token");
                r.answer(200, t);
            }
            _ => {
                let mut t = tokens(&a, None);
                let id = t["id_token"].as_str().unwrap();
                let payload = URL_SAFE_NO_PAD
                    .decode(id.split('.').nth(1).unwrap())
                    .unwrap();
                let mut payload: Value = serde_json::from_slice(&payload).unwrap();
                payload["aud"] = json!("other-client-id");
                t["id_token"] = json!(format!(
                    "e30.{}.fixture",
                    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap())
                ));
                r.answer(200, t);
            }
        }
        assert_eq!(
            callback(&mut b, &state).await.location(),
            Some("http://campfire.test/users/me/profile")
        );
        assert!(
            b.get("/users/me/profile")
                .await
                .text()
                .contains("Could not connect Google Calendar. Try again."),
            "rejection assertion: {case}"
        );
        assert!(
            a.db()
                .read(|c| GoogleAccount::for_user(c, DAVID))
                .await
                .unwrap()
                .is_none(),
            "rejection assertion: {case}"
        );
    }
    let state = start(&a, &mut b).await;
    let before = r.calls.lock().unwrap().len();
    let reply = b
        .get(&format!(
            "/google/callback?state={}&error=access_denied",
            encode(&state)
        ))
        .await;
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert!(
        b.get("/users/me/profile")
            .await
            .text()
            .contains("Google Calendar connection was not approved.")
    );
    assert_eq!(r.calls.lock().unwrap().len(), before);
    let counts = a
        .db()
        .read(|c| {
            Ok((
                c.query_row("SELECT count(*) FROM audit_logs", [], |r| {
                    r.get::<_, i64>(0)
                })?,
                c.query_row("SELECT count(*) FROM ws14g_emitted_jobs", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(counts, (0, 0));
}

#[tokio::test]
async fn google_connection_scope_retention_and_reconnect_clear_disconnected_reason() {
    let (a, r) = app().await;
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    let state = start(&a, &mut b).await;
    r.answer(200, tokens(&a, None));
    callback(&mut b, &state).await;
    let account = a
        .db()
        .read(|c| GoogleAccount::for_user(c, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.scopes, None);
    assert!(account.calendar());
    assert!(!account.drive());
    assert!(account.access_token_expires_at.is_some());
    assert!(
        b.get("/users/me/profile")
            .await
            .text()
            .contains("Google Calendar connected.")
    );
    let scopes = format!(
        "{} {}",
        campfire_db::models::google_account::CALENDAR_SCOPE,
        campfire_db::models::google_account::DRIVE_SCOPE
    );
    let state = start(&a, &mut b).await;
    r.answer(200, tokens(&a, Some(&scopes)));
    callback(&mut b, &state).await;
    let account = a
        .db()
        .read(|c| GoogleAccount::for_user(c, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.scopes.as_deref(), Some(scopes.as_str()));
    assert!(account.calendar() && account.drive());
    a.db()
        .write(|tx| {
            let mut account = GoogleAccount::for_user(tx.conn(), DAVID)?.unwrap();
            account.mark_disconnected(tx, "Google rejected the connection")
        })
        .await
        .unwrap();
    let state = start(&a, &mut b).await;
    let mut t = tokens(&a, None);
    t.as_object_mut().unwrap().remove("refresh_token");
    r.answer(200, t);
    callback(&mut b, &state).await;
    let account = a
        .db()
        .read(|c| GoogleAccount::for_user(c, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.scopes.as_deref(), Some(scopes.as_str()));
    assert_eq!(account.disconnected_reason, None);
    let enc = rails_compat::ar_encryption::ArEncryption::new(&a.booted.app.secrets);
    assert_eq!(
        account.refresh_token(&enc).unwrap().as_deref(),
        Some("new-refresh-token")
    );
}

#[tokio::test]
async fn google_connection_unreadable_disconnect_drops_cache_preserves_flags_and_other_grants() {
    use crate::controllers::presenters::test_support::JASON;
    let (a, r) = app().await;
    let now = campfire_db::Timestamp::from_jiff(a.booted.app.clock.now());
    support::grant(
        &a,
        DAVID,
        now.since(jiff::SignedDuration::from_hours(1)),
        true,
    )
    .await;
    support::grant(
        &a,
        JASON,
        now.since(jiff::SignedDuration::from_hours(1)),
        true,
    )
    .await;
    a.db()
        .write(move |tx| {
            let event_id = tx.conn().query_row(
                "SELECT event_id FROM event_attendances WHERE user_id=? LIMIT 1",
                [DAVID],
                |r| r.get::<_, i64>(0),
            )?;
            campfire_db::models::google_entry::reserve(tx, event_id, DAVID)?;
            campfire_db::models::google_entry::reserve(tx, event_id, JASON)?;
            campfire_db::models::google_meeting_cache::create(tx, DAVID)?;
            campfire_db::models::google_meeting_cache::complete(
                tx,
                DAVID,
                Some(json!([["2026-03-02T15:00:00Z", "2026-03-02T17:00:00Z"]])),
                None,
                None,
                now,
            )?;
            // Deliberately unreadable persisted ciphertext, as in Rails' corrupt_google_token!.
            tx.conn().execute(
                "UPDATE google_accounts SET access_token='unreadable fixture' WHERE user_id=?",
                [DAVID],
            )?;
            tx.conn().execute(
                "UPDATE users SET meeting_status_enabled=1,ooo_calendar_enabled=1 WHERE id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    sudo(&a, &mut b).await;
    for _ in 0..2 {
        assert_eq!(
            b.write(Req::new(Method::DELETE, "/google/connection"))
                .await
                .location(),
            Some("http://campfire.test/users/me/profile")
        );
        assert!(
            b.get("/users/me/profile")
                .await
                .text()
                .contains("Google Calendar disconnected.")
        );
    }
    let state = a.db().read(|c| Ok((
        GoogleAccount::for_user(c, DAVID)?.is_none(),
        GoogleAccount::for_user(c, JASON)?.is_some(),
        campfire_db::models::google_meeting_cache::find(c, DAVID)?.is_none(),
        c.query_row("SELECT meeting_status_enabled AND ooo_calendar_enabled FROM users WHERE id=?", [DAVID], |r| r.get::<_, bool>(0))?,
        c.query_row("SELECT count(*) FROM ws14g_emitted_jobs WHERE job_class='Calendar::DisconnectCleanupJob'", [], |r| r.get::<_, i64>(0))?,
        c.query_row("SELECT count(*) FROM audit_logs WHERE action='google.account.disconnect'", [], |r| r.get::<_, i64>(0))?,
    ))).await.unwrap();
    assert_eq!(state, (true, true, true, true, 0, 1));
    let entries = a
        .db()
        .read(|c| {
            Ok((
                c.query_row(
                    "SELECT count(*) FROM event_calendar_entries WHERE user_id=?",
                    [DAVID],
                    |r| r.get::<_, i64>(0),
                )?,
                c.query_row(
                    "SELECT count(*) FROM event_calendar_entries WHERE user_id=?",
                    [JASON],
                    |r| r.get::<_, i64>(0),
                )?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(entries, (0, 1));
    assert!(r.calls.lock().unwrap().is_empty());
}

mod parity_cases;

/// The boot data the SPA shell inlines on its next page.
async fn spa_boot(b: &mut Browser<'_>) -> Value {
    let html = b.get("/app/settings/integrations").await.text();
    let json = html
        .split("id=\"boot\"")
        .nth(1)
        .and_then(|rest| rest.split_once('>'))
        .and_then(|(_, rest)| rest.split("</script>").next())
        .expect("the SPA shell inlines its boot data");
    serde_json::from_str(json).unwrap()
}

/// The classic layout's flash: its kind (an alert carries the negative background) and message.
fn classic_flash(html: &str) -> Value {
    let flash = html
        .split("<div class=\"flash\"")
        .nth(1)
        .and_then(|rest| rest.split("<main").next())
        .expect("the classic layout shows the flash");
    let kind = if flash.contains("--flash-background: var(--color-negative)") {
        "alert"
    } else {
        "notice"
    };
    let message = flash
        .split("aria-atomic=\"true\">")
        .nth(1)
        .and_then(|rest| rest.split("</span>").next())
        .expect("the flash announces its message");
    json!({"kind": kind, "message": message})
}

#[tokio::test]
async fn google_connection_outcomes_return_new_ui_users_to_the_spa_with_their_flash() {
    use campfire_db::models::user::ui_preference::{self, UiPreference};
    for preference in [UiPreference::Classic, UiPreference::Next] {
        let a = TestApp::boot_seed_with_env(
            "default",
            crate::controllers::presenters::test_support::seed_clock(),
            &[("SPA_ENABLED", "1")],
        )
        .await
        .expect("default seed required");
        let r = Recorded::new(vec![]);
        support::install(&a, r.clone()).await;
        answer_calendar_jobs(&a, &r).await;
        a.db()
            .write(move |tx| ui_preference::store(tx, DAVID, preference))
            .await
            .unwrap();
        let destination = match preference {
            UiPreference::Next => "http://campfire.test/app/settings/integrations",
            _ => "http://campfire.test/users/me/profile",
        };
        let mut b = a.sign_in(DAVID).await;
        // The successful connection goes last: the jobs it enqueues call Google too, and must not
        // take a reply recorded for a later callback.
        let outcomes = [
            ("forged", "alert", "Google connection expired. Try again."),
            (
                "denied",
                "alert",
                "Google Calendar connection was not approved.",
            ),
            (
                "provider failure",
                "alert",
                "Could not connect Google Calendar. Try again.",
            ),
            (
                "drive-only",
                "alert",
                "Calendar permission was not granted. Reconnect to publish events.",
            ),
            ("connected", "notice", "Google Calendar connected."),
        ];
        for (outcome, kind, message) in outcomes {
            let state = start(&a, &mut b).await;
            let reply = match outcome {
                "forged" => callback(&mut b, "forged").await,
                "denied" => {
                    b.get(&format!(
                        "/google/callback?state={}&error=access_denied",
                        encode(&state)
                    ))
                    .await
                }
                "provider failure" => {
                    r.answer(400, json!({"error": "invalid_grant"}));
                    callback(&mut b, &state).await
                }
                "drive-only" => {
                    r.answer(
                        200,
                        tokens(&a, Some(campfire_db::models::google_account::DRIVE_SCOPE)),
                    );
                    callback(&mut b, &state).await
                }
                _ => {
                    let scopes = format!(
                        "{} {}",
                        campfire_db::models::google_account::CALENDAR_SCOPE,
                        campfire_db::models::google_account::DRIVE_SCOPE
                    );
                    r.answer(200, tokens(&a, Some(&scopes)));
                    callback(&mut b, &state).await
                }
            };
            assert_eq!(
                reply.status,
                StatusCode::FOUND,
                "{outcome}: {}",
                reply.text()
            );
            assert_eq!(reply.location(), Some(destination), "{outcome}");
            let flash = json!({"kind": kind, "message": message});
            if preference == UiPreference::Next {
                assert_eq!(spa_boot(&mut b).await["flash"], flash, "{outcome}");
            } else {
                let profile = b.get("/users/me/profile").await.text();
                assert_eq!(classic_flash(&profile), flash, "{outcome}");
            }
        }
        let connected = a
            .db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .expect("the last outcome connected Google");
        assert_eq!(connected.email, "david@gmail.test");
        assert!(connected.calendar() && connected.drive());
    }
}
