use super::card_tests::Fresh;
use crate::integrations::{
    github::{
        accounts::{Account, AccountInput},
        tests::crypto,
    },
    test_support::Route,
};
use axum::{
    body::Body,
    http::{HeaderMap, Request},
};
use campfire_db::{Agent, AgentApproval, AgentCredential, NewAgent, NewCredential, User};
use rusqlite::params;
use serde_json::{Value, json};
use sha2::Digest;
use tower::ServiceExt;
fn vectors() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../../../vectors/github_agent_http.json"
    ))
    .unwrap()
}
async fn fixture(case: &Value) -> Fresh {
    let fresh = Fresh::with_routes(
        &json!({"private":false}),
        vec![
            Route::new(
                "POST",
                "api.github.com",
                "/repos/rails/rails/issues/12/comments",
                201,
            )
            .body("{\"html_url\":\"https://github.com/rails/rails/pull/12#fixture\"}"),
        ],
    )
    .await;
    let c = case.clone();
    fresh.app.db.write(move|tx|{
        let now=tx.now();
        tx.conn().execute("INSERT INTO users(id,name,role,created_at,updated_at) VALUES (813,'Machine',2,?,?)",params![now,now])?;
        let agent=Agent::create(tx,NewAgent{user_id:813,owner_id:Some(811),suspended_at:(c["suspended"]==true).then_some(now),..Default::default()})?;
        tx.conn().execute("UPDATE agents SET id=881,daily_external_action_cap=? WHERE id=?",params![c["cap"].as_i64(),agent.id])?;
        AgentCredential::create(tx,NewCredential{agent_id:881,created_by_id:811,name:"HTTP".into(),token_digest:format!("{:x}",sha2::Sha256::digest("fixture-agent-secret")),token_last_four:"cret".into(),expires_at:(c["expired"]==true).then_some(now),revoked_at:(c["revoked"]==true).then_some(now)})?;
        // The oracle starts from a new schema and explicit credential ID 882.
        tx.conn().execute("UPDATE agent_credentials SET id=882 WHERE agent_id=881",[])?;
        if c["member"]!=false {tx.conn().execute("INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES (815,813,?,?)",params![now,now])?;}
        if c["other_member"]==true {tx.conn().execute("INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES (825,813,?,?)",params![now,now])?;}
        if c["mapping"]==false {tx.conn().execute("DELETE FROM github_pull_request_threads WHERE room_id=815",[])?;}
        tx.conn().execute("INSERT INTO agent_grants(agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES (881,?,811,?,?,?)",params![c["grant_room"].as_i64().unwrap_or(815),c["grant"].as_str().unwrap_or("external_action"),now,now])?;
        if c["linked"]!=false {
            let a=Account::create(tx,&crypto(),&AccountInput{user_id:813,github_login:"machine",access_token:"fixture-agent-token",refresh_token:None,token_expires_at:None,token_source:"pat"})?;
            if c["disconnected"]==true {Account::mark_disconnected(tx,a.id,"Disconnected")?;}
        }
        tx.conn().execute("DELETE FROM background_jobs",[])?;
        Ok(())
    }).await.unwrap();
    fresh
}
async fn post(f: &Fresh, path: &str, body: Value, secret: &str) -> (u16, HeaderMap, Value) {
    let res = f
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("Host", "example.org")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {secret}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status().as_u16();
    let headers = res.headers().clone();
    let raw = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        headers,
        if raw.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&raw).unwrap_or_else(|_| {
                panic!(
                    "non-JSON status {status}: {}",
                    String::from_utf8_lossy(&raw)
                )
            })
        },
    )
}
#[tokio::test]
async fn github_agent_http_authorization_actions_replay_budgets_and_throttle_match_rails() {
    for c in vectors() {
        let fresh = fixture(&c).await;
        let name = c["name"].as_str().unwrap();
        let mut statuses = vec![];
        let mut last = None;
        for _ in 0..c["repeat"].as_u64().unwrap_or(1) {
            let res = post(
                &fresh,
                c["path"].as_str().unwrap(),
                c["request_body"].clone(),
                c["secret"].as_str().unwrap_or("fixture-agent-secret"),
            )
            .await;
            statuses.push(res.0);
            last = Some(res);
        }
        let (status, headers, body) = last.unwrap();
        assert_eq!(status, c["status"].as_u64().unwrap() as u16, "{name}");
        assert_eq!(body, c["body"], "{name}");
        assert_eq!(json!(statuses), c["statuses"], "{name}");
        assert_eq!(
            headers.get("retry-after").and_then(|h| h.to_str().ok()),
            c["retry_after"].as_str(),
            "{name}"
        );
        if c["cache_control"]
            .as_str()
            .is_some_and(|s| s.contains("no-store"))
        {
            assert!(
                headers["cache-control"]
                    .to_str()
                    .unwrap()
                    .contains("no-store"),
                "{name}"
            );
        }
        if c["replay"].is_object() {
            let mut submitted = c["request_body"].clone();
            submitted["kind"] = json!("merge");
            submitted["body"] = json!("Second");
            let (s, _, b) = post(
                &fresh,
                c["path"].as_str().unwrap(),
                submitted,
                "fixture-agent-secret",
            )
            .await;
            assert_eq!(json!({"status":s,"body":b}), c["replay"], "{name}");
        }
        let rows=fresh.app.db.read(|conn|{
            let ids=conn.prepare("SELECT id FROM agent_approvals WHERE agent_id=881 ORDER BY id")?.query_map([],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            ids.into_iter().map(|id|{let a=AgentApproval::find(conn,id)?.unwrap();let inbox=conn.query_row("SELECT EXISTS(SELECT 1 FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND user_id=811)",[id],|r|r.get::<_,bool>(0))?;Ok(json!({"action":a.action,"summary":a.summary,"payload":a.payload.map(|s|serde_json::from_str::<Value>(&s).unwrap()),"external_id":a.external_id,"status":a.status,"expires_at":a.expires_at.jiff().strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),"agent_id":a.agent_id,"agent_credential_id":a.agent_credential_id,"room_id":a.room_id,"github_login":a.github_login,"inbox":inbox}))}).collect::<campfire_db::Result<Vec<_>>>()
        }).await.unwrap();
        assert_eq!(json!(rows), c["approvals"], "{name}");
        let used = fresh
            .app
            .db
            .read(|conn| {
                Ok((
                    conn.query_row(
                        "SELECT last_used_at IS NOT NULL FROM agent_credentials WHERE id=882",
                        [],
                        |r| r.get::<_, bool>(0),
                    )?,
                    conn.query_row(
                        "SELECT last_seen_at IS NOT NULL FROM agents WHERE id=881",
                        [],
                        |r| r.get::<_, bool>(0),
                    )?,
                ))
            })
            .await
            .unwrap();
        assert_eq!(json!(used.0), c["credential_used"], "{name}");
        assert_eq!(json!(used.1), c["agent_seen"], "{name}");
        assert!(fresh.server.received().is_empty(), "{name}");
    }
}
#[tokio::test]
async fn github_agent_http_races_committed_fanout_expiry_and_real_approved_job() {
    let c = json!({});
    let fresh = fixture(&c).await;
    let path = "/rooms/815/agents/github/pull_request_actions";
    let body = json!({"pull_request_id":816,"kind":"comment","body":"First","external_id":"race"});
    let replies = futures_util::future::join_all(
        (0..12).map(|_| post(&fresh, path, body.clone(), "fixture-agent-secret")),
    )
    .await;
    assert_eq!(replies.iter().filter(|r| r.0 == 202).count(), 1);
    assert_eq!(replies.iter().filter(|r| r.0 == 200).count(), 11);
    let id = replies[0].2["id"].as_i64().unwrap();
    assert!(replies.iter().all(|r| r.2["id"] == id));
    fresh
        .app
        .db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agent_approvals SET expires_at=? WHERE id=?",
                params![tx.now(), id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let (_, _, expired) = post(&fresh, path, body.clone(), "fixture-agent-secret").await;
    assert_eq!(expired["status"], "expired");
    fresh.app.db.write(|tx|{tx.conn().execute_batch("CREATE TRIGGER reject_approval_inbox BEFORE INSERT ON activity_items WHEN NEW.source_type='AgentApproval' BEGIN SELECT RAISE(ABORT,'inbox unavailable'); END;")?;Ok(())}).await.unwrap();
    let mut other = body.clone();
    other["external_id"] = json!("rollback");
    // Pinned HTTP oracle: views/agents_ui/github_request_boundaries.rb.
    // The approval commits before after_create_commit fan-out, even on HTTP 500.
    let response = fresh
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("Host", "example.org")
                .header(
                    "Authorization",
                    format!("{} {}", "Bearer", "fixture-agent-secret"),
                )
                .header("Content-Type", "application/json")
                .body(Body::from(other.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 500);
    let committed_id = fresh
        .app
        .db
        .write(|tx| {
            assert_eq!(
                tx.conn().query_row(
                    "SELECT COUNT(*) FROM agent_approvals WHERE external_id='rollback'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            let id: i64 = tx.conn().query_row("SELECT id FROM agent_approvals WHERE external_id='rollback'", [], |r| r.get(0))?;
            assert_eq!(AgentApproval::find(tx.conn(), id)?.unwrap().status, "pending");
            assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=?", [id], |r| r.get::<_, i64>(0))?, 0);
            tx.conn()
                .execute_batch("DROP TRIGGER reject_approval_inbox")?;
            Ok(id)
        })
        .await
        .unwrap();
    let (status, _, created) = post(&fresh, path, other, "fixture-agent-secret").await;
    assert_eq!(
        status, 200,
        "the failed fan-out still left an idempotency winner"
    );
    let approval_id = created["id"].as_i64().unwrap();
    assert_eq!(approval_id, committed_id);
    fresh.app.db.read(move |conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=?", [approval_id], |r| r.get::<_, i64>(0))?, 0, "Rails replay does not repair fan-out");
        Ok(())
    }).await.unwrap();
    fresh.app.db.write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_approval_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Github::PerformAgentActionJob' BEGIN SELECT RAISE(ABORT,'queue unavailable'); END;")?;Ok(())}).await.unwrap();
    let rejected = fresh
        .app
        .db
        .write(move |tx| {
            let mut approval = AgentApproval::find(tx.conn(), approval_id)?.unwrap();
            let by = User::find(tx.conn(), 811)?;
            approval.decide_authorized(tx, "approved", &by, None)?;
            Ok(())
        })
        .await;
    assert!(rejected.is_err());
    fresh.app.db.write(move|tx| {
        assert_eq!(AgentApproval::find(tx.conn(),approval_id)?.unwrap().status,"pending");
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM agent_events WHERE agent_approval_id=?",[approval_id],|r|r.get::<_,i64>(0))?,0);
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::PerformAgentActionJob'",[],|r|r.get::<_,i64>(0))?,0);
        tx.conn().execute_batch("DROP TRIGGER reject_approval_job")?;Ok(())
    }).await.unwrap();
    fresh.app.db.write(move|tx|{let mut approval=AgentApproval::find(tx.conn(),approval_id)?.unwrap();let by=User::find(tx.conn(),811)?;assert!(approval.decide_authorized(tx,"approved",&by,None)?.eq(&campfire_db::models::agent_approval::ApprovalDecision::Applied));assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::PerformAgentActionJob'",[],|r|r.get::<_,i64>(0))?,1);Ok(())}).await.unwrap();
    let registry = crate::jobs::registry();
    let config = crate::jobs::runner_config(&fresh.app.config);
    let (_, adhoc) = crate::jobs::Jobs::new(&registry, &config).unwrap();
    let runner = crate::jobs::start(
        fresh.app.clone(),
        registry,
        adhoc,
        config,
        crate::jobs::periodic::Loops::new(crate::jobs::periodic::Intervals::from_env()),
    );
    tokio::time::timeout(std::time::Duration::from_secs(10),async{loop{let complete=fresh.app.db.read(move|conn|Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM agent_events WHERE event_type='github_action_completed' AND agent_approval_id=? AND json_extract(metadata,'$.status')='completed')",[approval_id],|r|r.get::<_,bool>(0))?)).await.unwrap();if complete{break;}tokio::time::sleep(std::time::Duration::from_millis(20)).await;}}).await.unwrap();
    runner.shutdown(std::time::Duration::from_secs(2)).await;
    assert_eq!(fresh.server.received().len(), 1);
    // Cookie and legacy bot auth must never reach the approval service.
    let (_, _, body) = super::test_support::request(&fresh, "POST", path, body, json!({})).await;
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["error"],
        "Forbidden: Bearer agent token required"
    );
    let key = fresh
        .app
        .db
        .write(|tx| {
            Ok(User::create_bot(tx, "Legacy", None)?
                .plain_bot_key()
                .unwrap())
        })
        .await
        .unwrap();
    let response = fresh
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "{path}?{}",
                    url::form_urlencoded::Serializer::new(String::new())
                        .append_pair("bot_key", &key)
                        .finish()
                ))
                .header("Host", "example.org")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"pull_request_id":816,"kind":"comment","body":"hi"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 403);
}

#[tokio::test]
async fn github_agent_request_revalidation_preserves_authentication_usage_stamps() {
    let fresh = fixture(&json!({})).await;
    fresh.app.db.write(|tx| {
        // Simulate the common authentication before-action earlier in a slow request.
        let original=tx.now().ago(jiff::SignedDuration::from_mins(2));
        tx.conn().execute("UPDATE agent_credentials SET last_used_at=?,last_used_ip='203.0.113.7' WHERE id=882",[original])?;
        tx.conn().execute("UPDATE agents SET last_seen_at=? WHERE id=881",[original])?;
        let account=Account::for_user(tx.conn(),813)?.unwrap();
        let reply=crate::integrations::github::approval_requests::create(tx,crate::integrations::github::approval_requests::Request {user_id:813,room_id:815,pr_id:Some(816),secret:"fixture-agent-secret".into(),account,submitted:json!({"kind":"comment","body":"hi"})})?;
        assert_eq!(reply.status,202);
        let credential=AgentCredential::find(tx.conn(),882)?.unwrap();
        assert_eq!(credential.last_used_ip.as_deref(),Some("203.0.113.7"));
        assert_eq!(credential.last_used_at,Some(original));
        let stamp:Option<campfire_db::Timestamp>=tx.conn().query_row("SELECT last_seen_at FROM agents WHERE id=881",[],|r|r.get(0))?;
        assert_eq!(stamp,Some(original));Ok(())
    }).await.unwrap();
}
