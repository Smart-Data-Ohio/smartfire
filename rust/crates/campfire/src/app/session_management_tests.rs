//! Seeded session/device requests, durable mail jobs, and actual ordered socket revocation.
use crate::controllers::presenters::test_support::{DAVID, KEVIN, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{NewSession, Session, User, UserDevice};
use std::sync::Arc;
const CHROME: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const FIREFOX: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0";
async fn app() -> TestApp {
    TestApp::boot()
        .await
        .expect("build the pinned WS19 default seed")
}
async fn latest(a: &TestApp, user_id: i64) -> Session {
    a.db()
        .read(move |c| {
            Ok(Session::for_user(c, user_id)?
                .into_iter()
                .max_by_key(|s| s.id)
                .unwrap())
        })
        .await
        .unwrap()
}
async fn audit_count(a: &TestApp, action: &'static str) -> i64 {
    a.db()
        .read(move |c| {
            c.query_row(
                "SELECT count(*) FROM audit_logs WHERE action=?",
                [action],
                |r| r.get(0),
            )
            .map_err(Into::into)
        })
        .await
        .unwrap()
}
async fn new_other(a: &TestApp, user_id: i64, days: i64) -> Session {
    a.db()
        .write(move |tx| {
            let s = Session::start_with(
                tx,
                user_id,
                NewSession {
                    user_agent: Some(FIREFOX),
                    ip_address: Some("192.0.2.20"),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE sessions SET last_active_at=? WHERE id=?",
                rusqlite::params![
                    tx.now().ago(jiff::SignedDuration::from_secs(days * 86400)),
                    s.id
                ],
            )?;
            Ok(s)
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn list_scopes_sorts_describes_and_hides_only_expired_admin_sessions() {
    let a = app().await;
    let mut b = a.sign_in(KEVIN).await;
    let current = latest(&a, KEVIN).await;
    let id = current.id;
    a.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE sessions SET user_agent=?,ip_address='192.0.2.10' WHERE id=?",
                rusqlite::params![CHROME, id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let other = new_other(&a, KEVIN, 2).await;
    let foreign = new_other(&a, DAVID, 0).await;
    let page = b.get("/users/me/sessions").await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.text());
    assert_eq!(page.header("cache-control"), Some("no-store"));
    let text = page.text();
    for text in [
        "Chrome on macOS",
        "Firefox on Windows",
        "192.0.2.10",
        "2 days ago",
        "(this device)",
    ] {
        assert!(page.text().contains(text), "{text}");
    }
    assert!(text.contains(&format!("action=\"/users/me/sessions/{}\"", other.id)));
    assert!(!text.contains(&format!("action=\"/users/me/sessions/{}\"", foreign.id)));
    assert!(
        text.find(&format!("action=\"/users/me/sessions/{}\"", current.id))
            .unwrap()
            < text
                .find(&format!("action=\"/users/me/sessions/{}\"", other.id))
                .unwrap()
    );
    let mut admin = a.sign_in(DAVID).await;
    let expired = new_other(&a, DAVID, 8).await;
    let text = admin.get("/users/me/sessions").await.text();
    assert!(!text.contains(&format!("action=\"/users/me/sessions/{}\"", expired.id)));
    assert!(
        a.db()
            .read(move |c| Session::find(c, expired.id))
            .await
            .is_ok(),
        "hidden rows are not deleted"
    );
}

#[tokio::test]
async fn session_descriptions_match_rails_browser_os_and_unknown_cases() {
    let a=app().await;
    let mut s=new_other(&a,KEVIN,0).await;
    let safari="Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1";
    let edge="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0";
    for (ua,expected) in [(Some(CHROME),"Chrome on macOS"),(Some(FIREFOX),"Firefox on Windows"),(Some(safari),"Safari on iPhone"),(Some(edge),"Edge on Windows"),(None,"Unknown browser on Unknown device"),(Some("Rails Testing"),"Unknown browser on Unknown device")] {
        s.user_agent=ua.map(str::to_string);
        assert_eq!(crate::authentication::device_description(&s),expected);
    }
}
#[tokio::test]
async fn revoke_scopes_to_owner_and_requires_csrf_then_keeps_survivor_signed_in() {
    let a = app().await;
    let mut survivor = a.sign_in(KEVIN).await;
    let mut victim = a.sign_in(KEVIN).await;
    let id = latest(&a, KEVIN).await.id;
    let foreign = new_other(&a, DAVID, 0).await;
    let before = audit_count(&a, "session.revoke").await;
    assert_eq!(
        survivor
            .write(Req::new(
                Method::DELETE,
                &format!("/users/me/sessions/{}", foreign.id)
            ))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        survivor
            .send(Req::new(
                Method::DELETE,
                &format!("/users/me/sessions/{id}")
            ))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(audit_count(&a, "session.revoke").await, before);
    assert_eq!(
        survivor
            .write(Req::new(
                Method::DELETE,
                &format!("/users/me/sessions/{id}")
            ))
            .await
            .location(),
        Some("http://campfire.test/users/me/sessions")
    );
    assert_eq!(
        survivor.get("/users/me/profile").await.status,
        StatusCode::OK
    );
    assert_eq!(
        victim.get("/users/me/profile").await.location(),
        Some("http://campfire.test/session/new")
    );
    a.db().read(move |c|{ let (actor,target,details):(i64,i64,String)=c.query_row("SELECT actor_id,target_id,details FROM audit_logs WHERE action='session.revoke' ORDER BY id DESC LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;assert_eq!((actor,target),(KEVIN,KEVIN));assert_eq!(details,"{\"revoked_session_id\":\"[FILTERED]\"}");Ok(())}).await.unwrap();
}
#[tokio::test]
async fn revoke_current_signs_out_without_revoke_audit_and_removes_only_own_push_endpoint() {
    let a = app().await;
    let mut b = a.sign_in(KEVIN).await;
    let id = latest(&a, KEVIN).await.id;
    a.db().write(|tx|{for user in [KEVIN,DAVID]{tx.conn().execute("INSERT INTO push_subscriptions (user_id,endpoint,p256dh_key,auth_key,created_at,updated_at) VALUES (?,'https://example.com/push','k','a',?,?)",rusqlite::params![user,tx.now(),tx.now()])?;}Ok(())}).await.unwrap();
    let before = audit_count(&a, "session.revoke").await;
    assert_eq!(
        b.write(
            Req::new(Method::DELETE, &format!("/users/me/sessions/{id}"))
                .form(&[("push_subscription_endpoint", "https://example.com/push")])
        )
        .await
        .location(),
        Some("http://campfire.test/")
    );
    assert_eq!(
        b.get("/users/me/profile").await.location(),
        Some("http://campfire.test/session/new")
    );
    assert_eq!(audit_count(&a, "session.revoke").await, before);
    a.db().read(|c|{assert_eq!(c.query_row("SELECT count(*) FROM push_subscriptions WHERE endpoint='https://example.com/push' AND user_id=?",[KEVIN],|r|r.get::<_,i64>(0))?,0);assert_eq!(c.query_row("SELECT count(*) FROM push_subscriptions WHERE endpoint='https://example.com/push' AND user_id=?",[DAVID],|r|r.get::<_,i64>(0))?,1);Ok(())}).await.unwrap();
}
#[tokio::test]
async fn revoke_others_preserves_current_and_noop_has_no_audit() {
    let a = app().await;
    let mut b = a.sign_in(KEVIN).await;
    let current = latest(&a, KEVIN).await.id;
    new_other(&a, KEVIN, 0).await;
    let count = a
        .db()
        .read(|c| Session::count_for_user(c, KEVIN))
        .await
        .unwrap()
        - 1;
    assert_eq!(
        b.write(Req::new(Method::DELETE, "/users/me/sessions/revoke_others"))
            .await
            .location(),
        Some("http://campfire.test/users/me/sessions")
    );
    assert!(
        b.get("/users/me/sessions")
            .await
            .text()
            .contains(&format!("Signed out {count} other sessions."))
    );
    a.db()
        .read(move |c| {
            let rows = Session::for_user(c, KEVIN)?;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].id, current);
            Ok(())
        })
        .await
        .unwrap();
    let before = audit_count(&a, "session.revoke_others").await;
    b.write(Req::new(Method::DELETE, "/users/me/sessions/revoke_others"))
        .await;
    assert_eq!(audit_count(&a, "session.revoke_others").await, before);
    assert!(
        b.get("/users/me/sessions")
            .await
            .text()
            .contains("No other sessions to sign out.")
    );
}

pub(crate) async fn socket(
    a: &TestApp,
    cookie: &str,
    addr: std::net::SocketAddr,
) -> crate::channels::tests::support::Client {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut req = format!("ws://{addr}/cable").into_client_request().unwrap();
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    req.headers_mut()
        .insert("host", "campfire.test".parse().unwrap());
    req.headers_mut()
        .insert("origin", "http://campfire.test".parse().unwrap());
    let (socket, _) = tokio_tungstenite::connect_async(req).await.unwrap();
    let _ = a;
    crate::channels::tests::support::Client { socket }
}
#[tokio::test]
async fn revocation_drains_frames_then_disconnects_and_revoked_cookie_cannot_reconnect() {
    let a = app().await;
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = a.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut owner = a.sign_in(KEVIN).await;
    let victim = a.sign_in(KEVIN).await;
    let id = latest(&a, KEVIN).await.id;
    let cookie = victim.cookie_header();
    let mut client = socket(&a, &cookie, addr).await;
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let identifier = serde_json::json!({"channel":"ActivityChannel"}).to_string();
    client.confirm(&identifier).await;
    // Queue on the same production hub before the HTTP revoke enqueues DisconnectUser.
    let broadcasting = format!("user_{KEVIN}_activity");
    a.booted
        .app
        .cable
        .broadcast(&broadcasting, &serde_json::json!({"before":true}));
    assert_eq!(
        owner
            .write(Req::new(
                Method::DELETE,
                &format!("/users/me/sessions/{id}")
            ))
            .await
            .location(),
        Some("http://campfire.test/users/me/sessions")
    );
    let frames = client.until_closed().await;
    let delivery = frames
        .iter()
        .position(|s| s.contains("\"before\":true"))
        .expect("earlier frame delivered");
    let disconnect = frames
        .iter()
        .position(|s| s.contains("\"type\":\"disconnect\""))
        .expect("disconnect before close");
    assert!(delivery < disconnect, "{frames:?}");
    assert!(frames[disconnect].contains("\"reconnect\":true"));
    let mut reconnect = socket(&a, &cookie, addr).await;
    assert!(reconnect.next_text().await.contains("unauthorized"));
    reconnect.until_closed().await;
    assert_eq!(owner.get("/users/me/profile").await.status, StatusCode::OK);
    server.abort();
}

async fn password(
    b: &mut crate::controllers::presenters::test_support::Browser<'_>,
) -> crate::controllers::presenters::test_support::Reply {
    b.get("/session/new").await;
    b.write(
        Req::new(Method::POST, "/session")
            .header("user-agent", CHROME)
            .form(&[
                ("email_address", "david@37signals.com"),
                ("password", "secret123456"),
            ]),
    )
    .await
}
async fn stats(a: &TestApp) -> (i64, i64, i64) {
    a.db().read(|c|Ok((Session::count_for_user(c,DAVID)?,UserDevice::for_user(c,DAVID)?.len() as i64,c.query_row("SELECT count(*) FROM activity_items WHERE user_id=? AND event_type='new_sign_in'",[DAVID],|r|r.get(0))?))).await.unwrap()
}
#[tokio::test]
async fn completed_sign_ins_record_devices_first_and_known_are_quiet_new_alerts_survive_sign_out() {
    let a = app().await.without_job_runner().await;
    a.db()
        .write(|tx| {
            User::find(tx.conn(), DAVID)?.reset_two_factor(tx)?;
            tx.conn()
                .execute("DELETE FROM user_devices WHERE user_id=?", [DAVID])?;
            tx.conn().execute(
                "DELETE FROM activity_items WHERE user_id=? AND event_type='new_sign_in'",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut first = a.anonymous();
    let reply = password(&mut first).await;
    assert_eq!(reply.location(), Some("http://campfire.test/"));
    let cookie = reply
        .headers
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap())
        .find(|s| s.starts_with("device_id="))
        .unwrap()
        .to_lowercase();
    assert!(cookie.contains("httponly") && cookie.contains("samesite=lax"));
    assert_eq!((stats(&a).await.1, stats(&a).await.2), (1, 0));
    first.write(Req::new(Method::DELETE, "/session")).await;
    password(&mut first).await;
    assert_eq!((stats(&a).await.1, stats(&a).await.2), (1, 0));
    a.db()
        .write(|tx| {
            for s in Session::for_user(tx.conn(), DAVID)? {
                s.destroy(tx)?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut second = a.anonymous();
    assert_eq!(
        password(&mut second).await.location(),
        Some("http://campfire.test/")
    );
    assert_eq!((stats(&a).await.1, stats(&a).await.2), (2, 1));
    a.db().read(|c|{let session=latest_row(c)?;let (source,id):(String,i64)=c.query_row("SELECT source_type,source_id FROM activity_items WHERE user_id=? AND event_type='new_sign_in' ORDER BY id DESC LIMIT 1",[DAVID],|r|Ok((r.get(0)?,r.get(1)?)))?;assert_eq!((source,id),("Session".into(),session.id));assert_eq!(session.user_agent.as_deref(),Some(CHROME));assert!(!session.two_factor_verified());assert_eq!(c.query_row("SELECT count(*) FROM background_jobs WHERE job_class='Smartfire::MailDeliveryJob'",[],|r|r.get::<_,i64>(0))?,0);Ok(())}).await.unwrap();
}
fn latest_row(c: &campfire_db::Connection) -> campfire_db::Result<Session> {
    Ok(Session::for_user(c, DAVID)?
        .into_iter()
        .max_by_key(|s| s.id)
        .unwrap())
}

#[tokio::test]
async fn session_page_bodies_and_profile_panel_match_pinned_rails_bytes() {
    use askama::Template;
    use campfire_views::{users,helpers as h};
    struct Tokens;
    impl h::request_forgery::AuthenticityTokens for Tokens {
        fn global(&self) -> String { "GLOBAL".into() }
        fn for_form(&self, action: &str, method: &str) -> String { format!("{}:{action}",method.to_lowercase()) }
    }
    let a=app().await;
    let goldens:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../vectors/session_views.json"))).unwrap();
    let account=a.db().read(campfire_db::Account::first).await.unwrap();
    let rows=goldens["sessions"].as_array().unwrap().iter().map(|v|users::UserSession{
        id:v["id"].as_i64().unwrap(),current:v["current"].as_bool().unwrap(),description:v["description"].as_str().unwrap().into(),ip_address:v["ip_address"].as_str().map(str::to_string),last_active_at:v["last_active_at"].as_str().unwrap().parse().unwrap(),created_at:v["created_at"].as_str().unwrap().parse().unwrap(),
    }).collect::<Vec<_>>();
    for name in ["one","two","profile"] {
        let actual=h::request_forgery::rendering_with(h::request_forgery::RequestSecrets{tokens:Box::new(Tokens),csp_nonce:Some("NONCE".into())},||{
            crate::controllers::presenters::page::render_detached_at(&a.booted.app,account.as_ref(),"http://campfire.test",|ctx|{
                if name=="profile" {users::ProfileSessions{ctx}.render().unwrap()} else {users::SessionsIndex{ctx,sessions:if name=="one"{vec![rows[0].clone()]}else{rows.clone()},now:"2026-03-02T16:00:00Z".parse().unwrap()}.as_content().render().unwrap()}
            })
        });
        assert!(super::asset_goldens::compare(name, &actual, goldens[name].as_str().unwrap()));
    }
}
#[tokio::test]
async fn pending_first_factor_records_nothing_until_challenge_completes() {
    let a = app().await;
    a.db()
        .write(|tx| {
            UserDevice::record_sign_in(tx, DAVID, Some("previous-device"), Some(CHROME)).map(|_| ())
        })
        .await
        .unwrap();
    let before = stats(&a).await;
    let mut b = a.anonymous();
    assert_eq!(
        password(&mut b).await.location(),
        Some("http://campfire.test/two_factor_challenge")
    );
    assert_eq!(stats(&a).await, before);
    let enc = rails_compat::ar_encryption::ArEncryption::new(&a.booted.app.secrets);
    let secret = a
        .db()
        .write(move |tx| {
            let c = campfire_db::TwoFactorCredential::for_user(tx.conn(), DAVID)?.unwrap();
            tx.conn().execute(
                "UPDATE two_factor_credentials SET last_totp_at=NULL WHERE id=?",
                [c.id],
            )?;
            c.secret(&enc)
        })
        .await
        .unwrap();
    let code = rails_compat::totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
    b.write(Req::new(Method::POST, "/two_factor_challenge").form(&[("code", &code)]))
        .await;
    assert_eq!(stats(&a).await, (before.0 + 1, before.1 + 1, before.2 + 1));
}
#[tokio::test]
async fn configured_sign_in_uses_ws10_and_enqueue_failure_rolls_back_every_auth_row() {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    ));
    let a = TestApp::boot_with_clock_and_env(
        clock,
        &[
            ("SMTP_ADDRESS", "smtp.invalid"),
            ("MAILER_FROM", "alerts@example.com"),
            ("APP_URL", "http://campfire.test"),
        ],
    )
    .await
    .expect("default seed");
    a.db().write(|tx|{User::find(tx.conn(),DAVID)?.reset_two_factor(tx)?;tx.conn().execute_batch("CREATE TABLE ws9_sign_in_mail(args TEXT NOT NULL); CREATE TRIGGER ws9_capture_sign_in_mail BEFORE INSERT ON background_jobs WHEN NEW.job_class='Smartfire::MailDeliveryJob' BEGIN INSERT INTO ws9_sign_in_mail VALUES(NEW.arguments); END; CREATE TRIGGER ws9_reject_sign_in_mail BEFORE INSERT ON background_jobs WHEN NEW.job_class='Smartfire::MailDeliveryJob' BEGIN SELECT RAISE(ABORT,'mail unavailable'); END;")?;Ok(())}).await.unwrap();
    a.db()
        .write(|tx| {
            UserDevice::record_sign_in(tx, DAVID, Some("previous-device"), Some(CHROME)).map(|_| ())
        })
        .await
        .unwrap();
    let before = stats(&a).await;
    let mut b = a.anonymous();
    assert_eq!(
        password(&mut b).await.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(stats(&a).await, before);
    a.db()
        .write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER ws9_reject_sign_in_mail")?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        password(&mut b).await.location(),
        Some("http://campfire.test/")
    );
    a.db().read(|c|{let args:String=c.query_row("SELECT args FROM ws9_sign_in_mail",[],|r|r.get(0))?;let value:serde_json::Value=serde_json::from_str(&args).unwrap();let id=c.query_row("SELECT id FROM activity_items WHERE user_id=? AND event_type='new_sign_in' ORDER BY id DESC LIMIT 1",[DAVID],|r|r.get::<_,i64>(0))?;assert_eq!(value,serde_json::json!({"notification":{"NewSignIn":{"activity_item_id":id}}}));Ok(())}).await.unwrap();
}
