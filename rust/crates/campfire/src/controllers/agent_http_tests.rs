//! Committed Rails HTTP vectors, through the complete router and live SQLite policy.
use super::presenters::test_support::{Reply, Req, TestApp};
use campfire_db::{AgentCredential, NewCredential};
use campfire_kit::Method;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) const SECRET: &str = "ws11api-fixture-credential";
pub(super) const AGENT: i64 = 773018776;

pub(super) async fn setup() -> TestApp {
    let app = TestApp::boot_with_clock(std::sync::Arc::new(campfire_kit::FrozenClock::new(
        "2026-03-02T16:00:00Z".parse().unwrap(),
    )))
    .await
    .expect("WS11-api tests require the pinned default seed");
    initialize(&app).await;
    app
}

pub(super) async fn initialize(app: &TestApp) {
    app.db().write(|tx| {
        tx.conn().execute("UPDATE agents SET status='idle',status_note=NULL,status_changed_at=NULL,working_presence=NULL,working_presence_expires_at=NULL,last_seen_at=NULL WHERE id=?",[AGENT])?;
        tx.conn().execute("DELETE FROM agent_events WHERE agent_id=?", [AGENT])?;
        tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=?", [AGENT])?;
        tx.conn().execute("DELETE FROM agent_credentials WHERE agent_id=?", [AGENT])?;
        let digest = format!("{:x}", Sha256::digest(SECRET));
        AgentCredential::create(tx, NewCredential { agent_id: AGENT, created_by_id: 127326141, name: "HTTP contract".into(), token_last_four: digest[..4].into(), token_digest: digest, ..Default::default() })?;
        for (id, kind, metadata) in [(900_000_001, "github_action_completed", json!({"action":"github.comment","status":"done"})), (900_000_002, "approval_decided", json!({"approval_id":0}))] {
            tx.conn().execute("INSERT INTO agent_events(id,agent_id,event_type,outcome,metadata,created_at) VALUES(?,?,?,'delivered',?,'2026-03-02 16:00:00')", rusqlite::params![id, AGENT, kind, metadata.to_string()])?;
        }
        Ok(())
    }).await.unwrap();
}

async fn request(app: &TestApp, case: &Value) -> Reply {
    let mut req = Req::new(
        Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
        case["path"].as_str().unwrap(),
    )
    .header("accept", "application/json")
    .header("content-type", "application/json");
    if let Some(secret) = case["token"].as_str() {
        req = req.header("authorization", &["Bearer", secret].join(" "));
    }
    if let Some(body) = case["body"].as_str() {
        req = req.body(body);
    }
    for (name, value) in case["headers"].as_object().unwrap() {
        req = req.header(name, value.as_str().unwrap());
    }
    app.anonymous().send(req).await
}

async fn check(name: &str) {
    let app = setup().await;
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_http.json")).unwrap();
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap();
    app.db().write({let name=name.to_string(); move |tx| {
        if ["profile_invalid","profile_clear","profile_boolean_note"].contains(&name.as_str()) {
            tx.conn().execute("UPDATE agents SET status='working',status_note='Tests',status_changed_at=?,working_presence='Thinking',working_presence_expires_at=? WHERE id=?",rusqlite::params![tx.now(),tx.now().since(jiff::SignedDuration::from_mins(5)),AGENT])?;
        }
        if name == "revoked_credential" { tx.conn().execute("UPDATE agent_credentials SET revoked_at=? WHERE agent_id=?", rusqlite::params![tx.now(),AGENT])?; }
        if name == "expired_credential" { tx.conn().execute("UPDATE agent_credentials SET expires_at=? WHERE agent_id=?", rusqlite::params![tx.now(),AGENT])?; }
        if ["missing_grant", "slash_missing_grant"].contains(&name.as_str()) {
            tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,revoked_at,created_at,updated_at) VALUES(?,'read_messages',127326141,?,?,?)", rusqlite::params![AGENT, if name == "missing_grant" {Some(tx.now())} else {None},tx.now(),tx.now()])?;
        }
        Ok(())
    }}).await.unwrap();
    if name == "event_rate_overflow" {
        let mut polling = case.clone();
        polling["path"] = json!("/agents/events");
        for _ in 0..120 {
            assert_eq!(request(&app, &polling).await.status.as_u16(), 200);
        }
    }
    if ["event_ack_replay", "slash_register_replay"].contains(&name) {
        assert_eq!(
            request(&app, case).await.status.as_u16(),
            case["status"].as_u64().unwrap() as u16
        );
    }
    let reply = request(&app, case).await;
    if name == "profile_update" {
        let agent = app
            .db()
            .read(|conn| campfire_db::Agent::find(conn, AGENT))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(agent.provider, None);
        assert_eq!(agent.daily_message_cap, None);
    }
    assert_eq!(
        reply.status.as_u16(),
        case["status"].as_u64().unwrap() as u16,
        "{name}: {}",
        reply.text()
    );
    let body = if reply.body.is_empty() {
        Value::Null
    } else {
        reply.json()
    };
    assert_eq!(body, case["response"], "{name}");
    for (header, expected) in case["response_headers"].as_object().unwrap() {
        assert_eq!(reply.header(header), expected.as_str(), "{name}: {header}");
    }
}

#[tokio::test]
async fn revoked_and_expired_credentials() {
    check("revoked_credential").await;
    check("expired_credential").await;
}
#[tokio::test]
async fn grant_denial_is_403_and_nonmembership_is_404() {
    check("missing_grant").await;
    check("slash_nonmember").await;
    check("slash_missing_grant").await;
}
#[tokio::test]
async fn credential_rate_overflow() {
    check("event_rate_overflow").await;
}
#[tokio::test]
async fn agent_token_cannot_access_human_endpoint() {
    check("agent_human_endpoint").await;
}
#[tokio::test]
async fn rest_vectors() {
    for name in [
        "event_envelope",
        "event_array",
        "event_dropped_cursor",
        "event_ack",
        "event_ack_replay",
        "event_ack_missing",
        "unknown_credential",
        "step_no_parent",
        "step_missing",
        "step_create",
        "slash_invalid",
        "slash_register",
        "slash_register_replay",
        "slash_missing",
    ] {
        check(name).await;
    }
}

#[tokio::test]
async fn profile_vectors() {
    for name in [
        "profile_get",
        "profile_update",
        "profile_invalid",
        "profile_clear",
        "profile_boolean_note",
    ] {
        check(name).await;
    }
}

#[tokio::test]
async fn step_duration_validates_before_cast() {
    for index in 0..7 {
        check(&format!("step_duration_{index}")).await;
    }
}

#[tokio::test]
async fn invalid_duration_never_persists_and_parent_policy_runs_first() {
    let app = setup().await;
    for (message_id, status) in [(935961918, 422), (0, 404)] {
        let reply = app
            .anonymous()
            .send(
                Req::new(Method::POST, "/agents/steps")
                    .header("authorization", &["Bearer", SECRET].join(" "))
                    .header("content-type", "application/json")
                    .body(
                        json!({"message_id":message_id,"name":"Inspect","duration_ms":4.5})
                            .to_string(),
                    ),
            )
            .await;
        assert_eq!(reply.status.as_u16(), status);
    }
    let count: i64 = app
        .db()
        .read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM agent_steps", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(count, 0);
}
