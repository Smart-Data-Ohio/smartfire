//! Drive controller security and viewer/recipient policy through the full router.
use super::google_api_tests::{self as support, Recorded};
use crate::controllers::presenters::test_support::{
    ALL_TALK, BENDER, BENDER_KEY, DAVID, DIRECT_KEVIN_BENDER, JASON, KEVIN, Req, TestApp,
};
use campfire_db::Timestamp;
use hyper::{Method, StatusCode};
use serde_json::{Value, json};
use std::sync::Arc;
async fn app() -> (TestApp, Arc<Recorded>) {
    let clock = Arc::new(campfire_kit::FrozenClock::new(
        crate::controllers::presenters::test_support::seed_clock().now(),
    ));
    let a = TestApp::boot_with_clock(clock)
        .await
        .expect("default seed required");
    let r = Recorded::new(vec![]);
    support::install(&a, r.clone()).await;
    a.booted.app.google.drive().install_picker(true);
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM google_accounts", [])?;
            Ok(())
        })
        .await
        .unwrap();
    (a, r)
}
async fn grant(a: &TestApp, id: i64) {
    support::grant(
        a,
        id,
        Timestamp::from_jiff(a.booted.app.clock.now()).since(jiff::SignedDuration::from_hours(1)),
        true,
    )
    .await;
}
const FILE: &str = "1AbcDefGhIjKlMnOpQrSt";
fn json_req(method: Method, path: &str, body: Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&body).unwrap())
}
#[tokio::test]
async fn google_drive_viewer_inaccessible_file_is_empty_404_and_never_reuses_other_viewer_cache() {
    let (a, r) = app().await;
    grant(&a, DAVID).await;
    grant(&a, JASON).await;
    let path = format!("/google/drive/files/{FILE}");
    let mut owner = a.sign_in(DAVID).await;
    let mut viewer = a.sign_in(JASON).await;
    r.answer(200, support::vectors()["drive_file"].clone());
    let reply = owner.get(&path).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json()["name"], "Q3 Planning");
    assert_eq!(reply.json()["kind"], "document");
    assert_eq!(owner.get(&path).await.status, StatusCode::OK);
    assert_eq!(r.calls.lock().unwrap().len(), 1);
    r.answer(
        403,
        json!({"error":{"message":"private document metadata"}}),
    );
    let denied = viewer.get(&path).await;
    assert_eq!(denied.status, StatusCode::NOT_FOUND);
    assert!(denied.body.is_empty());
    assert_eq!(r.calls.lock().unwrap().len(), 2);
}
#[tokio::test]
async fn google_drive_requires_viewer_grant_and_valid_id_and_has_distinct_anonymous_behavior() {
    let (a, r) = app().await;
    let mut anon = a.anonymous();
    assert_eq!(
        anon.get("/google/drive/files").await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        anon.get(&format!("/google/drive/files/{FILE}"))
            .await
            .location(),
        Some("http://campfire.test/session/new")
    );
    let mut b = a.sign_in(DAVID).await;
    assert_eq!(
        b.get("/google/drive/files").await.status,
        StatusCode::NOT_FOUND
    );
    grant(&a, DAVID).await;
    for id in ["short", "not%20a%20file%20id!!"] {
        let reply = b.get(&format!("/google/drive/files/{id}")).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        assert!(reply.body.is_empty());
    }
    a.db().write(|tx|{tx.conn().execute("UPDATE google_accounts SET scopes='https://www.googleapis.com/auth/drive.metadata.readonly' WHERE user_id=?",[DAVID])?;Ok(())}).await.unwrap();
    assert_eq!(
        b.get("/google/drive/files").await.status,
        StatusCode::NOT_FOUND
    );
    assert!(r.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn google_drive_lists_never_cache_trim_and_cap_terms_and_return_502_on_failure() {
    let (a, r) = app().await;
    grant(&a, DAVID).await;
    let mut b = a.sign_in(DAVID).await;
    let v = support::vectors();
    for _ in 0..2 {
        r.answer(200, v["drive_list"].clone());
        let reply = b
            .get("/google/drive/files?q=%20%20bob%27s%5Cdraft%20%20")
            .await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.json()["files"].as_array().unwrap().len(), 2);
    }
    let calls = r.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 2);
    assert!(
        calls[0]["path"]
            .as_str()
            .unwrap()
            .contains("bob%5C%27s%5C%5Cdraft")
    );
    let term = "x".repeat(105);
    r.answer(200, json!({"files":[]}));
    b.get(&format!("/google/drive/files?q={term}")).await;
    let path = r.calls.lock().unwrap()[2]["path"]
        .as_str()
        .unwrap()
        .to_string();
    let u = url::Url::parse(&format!("https://www.googleapis.com{path}")).unwrap();
    let q = u
        .query_pairs()
        .find(|(k, _)| k == "q")
        .unwrap()
        .1
        .into_owned();
    assert_eq!(
        q,
        format!("name contains '{}' and trashed=false", "x".repeat(100))
    );
    r.answer(500, json!({}));
    let reply = b.get("/google/drive/files").await;
    assert_eq!(reply.status, StatusCode::BAD_GATEWAY);
    assert_eq!(reply.json(), json!({"error":"drive_unavailable"}));
}
#[tokio::test]
async fn google_drive_show_and_list_throttle_before_new_http_with_independent_budgets() {
    let (a, r) = app().await;
    grant(&a, DAVID).await;
    let mut b = a.sign_in(DAVID).await;
    r.answer(200, support::vectors()["drive_file"].clone());
    let path = format!("/google/drive/files/{FILE}");
    for _ in 0..60 {
        assert_eq!(b.get(&path).await.status, StatusCode::OK);
    }
    assert_eq!(b.get(&path).await.status, StatusCode::TOO_MANY_REQUESTS);
    for _ in 0..30 {
        r.answer(200, json!({"files":[]}));
        assert_eq!(b.get("/google/drive/files").await.status, StatusCode::OK);
    }
    let denied = b.get("/google/drive/files").await;
    assert_eq!(denied.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(denied.json(), json!({"error":"rate_limited"}));
    assert_eq!(r.calls.lock().unwrap().len(), 31);
}
#[tokio::test]
async fn google_drive_recipients_require_human_membership_but_no_google_grant() {
    let (a, r) = app().await;
    let path = format!("/rooms/{ALL_TALK}/drive_recipients");
    let mut b = a.sign_in(DAVID).await;
    let reply = b
        .send(Req::new(Method::GET, &path).header("accept", "application/json"))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.headers["cache-control"], "no-store");
    assert_eq!(
        reply.json()["recipients"],
        json!([{"id":JASON,"name":"Jason","email":"jason@37signals.com"}])
    );
    let reply = b
        .send(
            Req::new(
                Method::GET,
                &format!("/rooms/{DIRECT_KEVIN_BENDER}/drive_recipients"),
            )
            .header("accept", "application/json"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert!(reply.body.is_empty());
    let mut anon = a.anonymous();
    assert_eq!(
        anon.send(Req::new(Method::GET, &path).header("accept", "application/json"))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let mut bot = a.sign_in(BENDER).await;
    assert_eq!(bot.get(&path).await.status, StatusCode::FORBIDDEN);
    assert_eq!(
        anon.get(&format!("{path}?bot_key={BENDER_KEY}"))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert!(r.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn google_drive_recipient_validation_rechecks_stale_membership_and_preserves_order() {
    let (a, _) = app().await;
    let path = format!("/rooms/{ALL_TALK}/drive_recipients/validate");
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    let valid = b
        .write(json_req(
            Method::POST,
            &path,
            json!({"user_ids":[format!(" {JASON} "),JASON]}),
        ))
        .await;
    assert_eq!(valid.status, StatusCode::OK);
    assert_eq!(valid.json()["recipients"].as_array().unwrap().len(), 1);
    for value in [
        json!("mail@external.test"),
        json!(["mail@external.test"]),
        Value::Null,
    ] {
        let reply = b
            .write(json_req(Method::POST, &path, json!({"user_ids":value})))
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(reply.json(), json!({"error":"invalid_recipients"}));
    }
    let reply = b
        .write(json_req(
            Method::POST,
            &path,
            json!({"user_ids":[KEVIN,DAVID,BENDER]}),
        ))
        .await;
    assert_eq!(
        reply.json(),
        json!({"error":"invalid_recipients","invalid_ids":[KEVIN,DAVID,BENDER]})
    );
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                [ALL_TALK, JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = b
        .write(json_req(Method::POST, &path, json!({"user_ids":[JASON]})))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(reply.json()["invalid_ids"], json!([JASON]));
    let empty = b
        .write(json_req(Method::POST, &path, json!({"user_ids":[]})))
        .await;
    assert_eq!(empty.status, StatusCode::OK);
    assert_eq!(empty.json(), json!({"recipients":[]}));
}
#[tokio::test]
async fn google_drive_recipients_exclude_agents_inactive_and_bad_email_without_domain_filter() {
    let (a, _) = app().await;
    let path = format!("/rooms/{ALL_TALK}/drive_recipients");
    let mut b = a.sign_in(DAVID).await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET email_address='external@contractor.test' WHERE id=?",
                [JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        b.get(&path).await.json()["recipients"][0]["email"],
        "external@contractor.test"
    );
    for status in [1, 2] {
        a.db()
            .write(move |tx| {
                tx.conn()
                    .execute("UPDATE users SET status=? WHERE id=?", [status, JASON])?;
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(b.get(&path).await.json()["recipients"], json!([]));
    }
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET status=0,email_address='not-an-email' WHERE id=?",
                [JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(b.get(&path).await.json()["recipients"], json!([]));
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET email_address='external@contractor.test' WHERE id=?",
                [JASON],
            )?;
            tx.conn().execute(
                "INSERT INTO agents(user_id,owner_id,created_at,updated_at) VALUES(?,?,?,?)",
                rusqlite::params![JASON, DAVID, tx.now(), tx.now()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(b.get(&path).await.json()["recipients"], json!([]));
    let mut agent = a.sign_in(JASON).await;
    assert_eq!(agent.get(&path).await.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn google_drive_file_json_and_all_mime_kinds_match_rails() {
    let (a, r) = app().await;
    grant(&a, DAVID).await;
    let mut b = a.sign_in(DAVID).await;
    let file = support::vectors()["drive_file"].clone();
    for (index, (mime, kind)) in [
        ("application/vnd.google-apps.document", "document"),
        ("application/vnd.google-apps.spreadsheet", "spreadsheet"),
        ("application/vnd.google-apps.presentation", "presentation"),
        ("application/vnd.google-apps.form", "form"),
        ("application/vnd.google-apps.folder", "folder"),
        ("application/pdf", "pdf"),
        ("image/png", "file"),
        ("application/vnd.google-apps.unknown", "file"),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("kind-mapping-{index}1");
        let mut payload = file.clone();
        payload["id"] = json!(id);
        payload["mimeType"] = json!(mime);
        r.answer(200, payload.clone());
        let reply = b.get(&format!("/google/drive/files/{id}")).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(
            reply.json(),
            json!({"id":id,"name":payload["name"],"kind":kind,"modified_at":payload["modifiedTime"],"owner":payload["owners"][0]["displayName"],"url":payload["webViewLink"]})
        );
    }
    let mut payload = file;
    payload["mimeType"] = json!("image/png");
    r.answer(200, json!({"files":[payload]}));
    assert_eq!(
        b.get("/google/drive/files").await.json()["files"][0]["kind"],
        "file"
    );
}
#[tokio::test]
async fn google_drive_transport_quota_and_forbidden_fail_with_rails_statuses() {
    let (a, r) = app().await;
    grant(&a, DAVID).await;
    let mut b = a.sign_in(DAVID).await;
    let show = format!("/google/drive/files/{FILE}");
    r.fail_next();
    assert_eq!(b.get(&show).await.status, StatusCode::SERVICE_UNAVAILABLE);
    r.answer(429, json!({}));
    assert_eq!(b.get(&show).await.status, StatusCode::SERVICE_UNAVAILABLE);
    for status in [403, 404] {
        r.answer(status, json!({}));
        let denied = b.get(&show).await;
        assert_eq!(denied.status, StatusCode::NOT_FOUND);
        assert!(denied.body.is_empty());
    }
    r.fail_next();
    let failed = b.get("/google/drive/files").await;
    assert_eq!(failed.status, StatusCode::BAD_GATEWAY);
    assert_eq!(failed.json(), json!({"error":"drive_unavailable"}));
    r.answer(403, json!({}));
    let denied = b.get("/google/drive/files").await;
    assert_eq!(denied.status, StatusCode::NOT_FOUND);
    assert!(denied.body.is_empty());
}
#[tokio::test]
async fn google_drive_dead_unreadable_or_calendar_only_accounts_do_not_call_google() {
    let (a, r) = app().await;
    let mut b = a.sign_in(DAVID).await;
    for mode in ["disconnected", "unreadable", "calendar-only"] {
        grant(&a, DAVID).await;
        a.db().write(move|tx| {match mode {"disconnected"=>{tx.conn().execute("UPDATE google_accounts SET disconnected_reason='rejected' WHERE user_id=?",[DAVID])?;},"unreadable"=>{tx.conn().execute("UPDATE google_accounts SET refresh_token='tampered-ciphertext' WHERE user_id=?",[DAVID])?;},_=>{tx.conn().execute("UPDATE google_accounts SET scopes=NULL WHERE user_id=?",[DAVID])?;}}Ok(())}).await.unwrap();
        for path in [
            "/google/drive/files".to_owned(),
            format!("/google/drive/files/{FILE}"),
        ] {
            let reply = b.get(&path).await;
            assert_eq!(reply.status, StatusCode::NOT_FOUND, "{mode}");
            assert!(reply.body.is_empty());
        }
        if mode == "unreadable" {
            let reason = a
                .db()
                .read(|c| {
                    Ok(
                        campfire_db::models::google_account::GoogleAccount::for_user(c, DAVID)?
                            .unwrap()
                            .disconnected_reason,
                    )
                })
                .await
                .unwrap();
            assert_eq!(
                reason.as_deref(),
                Some(campfire_db::models::google_account::UNREADABLE_TOKEN_REASON)
            );
        }
    }
    assert!(r.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn google_drive_index_refreshes_expired_access_and_invalid_grant_disconnects() {
    let (a, r) = app().await;
    let mut b = a.sign_in(DAVID).await;
    let v = support::vectors();
    let old =
        Timestamp::from_jiff(a.booted.app.clock.now()).ago(jiff::SignedDuration::from_hours(1));
    support::grant(&a, DAVID, old, true).await;
    r.answer(200, v["refresh"].clone());
    r.answer(200, v["drive_list"].clone());
    assert_eq!(
        b.get("/google/drive/files?q=%20%20").await.status,
        StatusCode::OK
    );
    let calls = r.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["path"], "/token");
    assert_eq!(calls[1]["access_token"], "refreshed-access-token");
    let u = url::Url::parse(&format!(
        "https://www.googleapis.com{}",
        calls[1]["path"].as_str().unwrap()
    ))
    .unwrap();
    let q = u
        .query_pairs()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(q["q"], "trashed=false");
    assert_eq!(q["pageSize"], "10");
    assert_eq!(q["orderBy"], "modifiedTime desc");
    assert_eq!(q["spaces"], "drive");
    support::grant(&a, DAVID, old, true).await;
    r.answer(400, v["invalid_grant"].clone());
    let denied = b.get("/google/drive/files").await;
    assert_eq!(denied.status, StatusCode::NOT_FOUND);
    assert!(denied.body.is_empty());
    let account = a
        .db()
        .read(|c| campfire_db::models::google_account::GoogleAccount::for_user(c, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        account.disconnected_reason.as_deref(),
        Some("Google rejected the connection")
    );
}
#[tokio::test]
async fn google_drive_index_rejects_unenrolled_and_stale_enrolled_sessions_before_http() {
    use campfire_db::NewSession;
    use campfire_kit::Crypto;
    for stale in [false, true] {
        let (a, r) = app().await;
        grant(&a, DAVID).await;
        a.db()
            .write(|tx| {
                if let Some(c) = campfire_db::TwoFactorCredential::for_user(tx.conn(), DAVID)? {
                    c.destroy(tx)?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let token = a
            .db()
            .write(|tx| {
                campfire_db::Session::start_with(
                    tx,
                    DAVID,
                    NewSession {
                        two_factor_verified: false,
                        ..Default::default()
                    },
                )
                .map(|s| s.token)
            })
            .await
            .unwrap();
        if stale {
            let enc = rails_compat::ar_encryption::ArEncryption::new(&a.booted.app.secrets);
            a.db()
                .write(move |tx| {
                    let c = campfire_db::TwoFactorCredential::create(
                        tx,
                        &enc,
                        DAVID,
                        "JBSWY3DPEHPK3PXP",
                    )?;
                    tx.conn().execute(
                        "UPDATE two_factor_credentials SET confirmed_at=? WHERE id=?",
                        rusqlite::params![tx.now(), c.id],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        let signed = campfire_kit::RailsCrypto::new(a.booted.app.secrets.clone()).sign_cookie(
            "session_token",
            &token,
            None,
        );
        let mut b = a.anonymous();
        b.absorb_cookie_header(&format!(
            "session_token={}",
            campfire_kit::cookies::escape(&signed)
        ));
        let denied = b
            .send(Req::new(Method::GET, "/google/drive/files").header("accept", "application/json"))
            .await;
        assert_eq!(
            denied.status,
            if stale {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::FORBIDDEN
            }
        );
        assert!(r.calls.lock().unwrap().is_empty());
        if stale {
            assert!(
                a.db()
                    .read(move |c| campfire_db::Session::find_by_token(c, &token))
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    }
}

#[tokio::test]
async fn google_drive_real_agent_credentials_match_rails_request_authentication_order() {
    const JZ: i64 = 773523953; // Rails fixture identity, shared with google_admin_links.json.
    use campfire_db::models::{
        agent::{Agent, NewAgent},
        agent_credential::AgentCredential,
    };
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/google_drive_agent_auth.json"
    ))
    .unwrap();
    let (mut a, r) = app().await;
    a.booted.jobs.stop(std::time::Duration::from_secs(5)).await;
    let (agent_id, credential_id, secret) = a
        .db()
        .write(|tx| {
            let agent = Agent::create(
                tx,
                NewAgent {
                    user_id: JZ,
                    owner_id: Some(DAVID),
                    ..Default::default()
                },
            )?;
            let (credential, secret) = AgentCredential::create_with_secret(
                tx,
                agent.id,
                "Drive authorization fixture",
                DAVID,
                None,
            )?;
            Ok((agent.id, credential.id, secret))
        })
        .await
        .unwrap();
    grant(&a, JZ).await;
    // Recorded metadata is only usable if a guard regresses; the correct path makes no HTTP.
    r.answer(200, support::vectors()["drive_file"].clone());
    for case in vectors["cases"].as_array().unwrap() {
        a.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE agent_credentials SET last_used_at=NULL,last_used_ip=NULL WHERE id=?",
                    [credential_id],
                )?;
                tx.conn()
                    .execute("UPDATE agents SET last_seen_at=NULL WHERE id=?", [agent_id])?;
                Ok(())
            })
            .await
            .unwrap();
        let method: Method = case["method"]
            .as_str()
            .unwrap()
            .to_uppercase()
            .parse()
            .unwrap();
        let path = case["path"].as_str().unwrap();
        let mut caller = a.anonymous();
        let reply = caller
            .send(
                json_req(method, path, json!({"user_ids":[DAVID]}))
                    .header("authorization", &format!("Bearer {secret}")),
            )
            .await;
        assert_eq!(
            json!(reply.status.as_u16()),
            case["status"],
            "{path}: authorized agent status"
        );
        assert_eq!(json!(reply.text()), case["body"], "{path}: exact body");
        let usage = a
            .db()
            .read(move |c| {
                let used = c.query_row(
                    "SELECT last_used_at IS NOT NULL FROM agent_credentials WHERE id=?",
                    [credential_id],
                    |r| r.get::<_, bool>(0),
                )?;
                let seen = c.query_row(
                    "SELECT last_seen_at IS NOT NULL FROM agents WHERE id=?",
                    [agent_id],
                    |r| r.get::<_, bool>(0),
                )?;
                Ok((used, seen))
            })
            .await
            .unwrap();
        assert_eq!(
            json!(usage.0),
            case["credential_used"],
            "{path}: credential use"
        );
        assert_eq!(json!(usage.1), case["agent_seen"], "{path}: agent activity");
        assert!(
            r.calls.lock().unwrap().is_empty(),
            "{path}: Google must not be contacted"
        );
    }
    println!("Pinned Rails real-agent Drive requests: 4 exercised; 0 skipped");
}

mod recipients;

mod endpoints;
