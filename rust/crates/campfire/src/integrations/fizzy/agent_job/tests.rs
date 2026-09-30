use super::*;
use crate::{
    controllers::presenters::test_support::*,
    integrations::{
        fizzy::accounts::Input,
        test_support::{FakeResolver, FakeServer, MappingDialer, Route, network},
    },
};
use std::sync::Arc;
async fn app() -> TestApp {
    let mut app = TestApp::boot().await.expect("pinned seeds required");
    app.booted
        .jobs
        .stop(std::time::Duration::from_secs(1))
        .await;
    app
}
async fn setup(app: &TestApp, kind: &str) -> (i64, i64) {
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    let kind = kind.to_owned();
    app.db().write(move|tx| {
        let account=Account::relink(tx,&crypto,&Input{user_id:DAVID,account_id:"897362094",account_name:Some("Smart Data"),fizzy_user_id:Some("03user1"),fizzy_user_name:Some("David"),token:"owner-token-abc"})?;
        let agent:i64=tx.conn().query_row("SELECT id FROM agents WHERE user_id=?",[BENDER],|r|r.get(0))?;
        tx.conn().execute("UPDATE agents SET owner_id=?,suspended_at=NULL WHERE id=?",params![DAVID,agent])?;
        tx.conn().execute("INSERT INTO agent_grants (agent_id,capability,granted_by_id,created_at,updated_at) VALUES (?,'external_action',?,?,?)",params![agent,DAVID,tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO webhooks (user_id,url,created_at,updated_at) SELECT ?1,'https://example.test/events',?2,?3 WHERE NOT EXISTS(SELECT 1 FROM webhooks WHERE user_id=?1)",params![BENDER,tx.now(),tx.now()])?;
        let action=Action::from_payload(json!({"account_id":"897362094","kind":kind,"number":579,"body":"Nice work","board_id":"03board1","title":"Ship it"}));
        tx.conn().execute("INSERT INTO agent_approvals (agent_id,action,summary,payload,status,expires_at,decided_by_id,fizzy_connected_account_id,fizzy_user_id,created_at,updated_at) VALUES (?,?,?,?,'approved',?,?,?,?,?,?)",params![agent,action.action_name(),action.summary(),action.payload_json(),tx.now().since(jiff::SignedDuration::from_hours(24)),DAVID,account.id,"03user1",tx.now(),tx.now()])?;
        Ok((tx.conn().last_insert_rowid(),agent))
    }).await.unwrap()
}
async fn state(app: &TestApp, id: i64) -> Value {
    app.db().read(move|c| {
        let (event,metadata,outcome,actor,room,message,webhook):(i64,String,String,Option<i64>,Option<i64>,Option<i64>,String)=c.query_row("SELECT id,metadata,outcome,actor_id,room_id,message_id,webhook_status FROM agent_events WHERE agent_approval_id=? AND event_type='fizzy_action_completed'",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?)))?;
        let audits:i64=c.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.fizzy_action.execute' AND target_id=?",[id],|r|r.get(0))?;
        let jobs:i64=c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id')=?",[event],|r|r.get(0))?;
        Ok(json!({"event":event,"metadata":serde_json::from_str::<Value>(&metadata).unwrap(),"outcome":outcome,"actor":actor,"room":room,"message":message,"webhook":webhook,"audits":audits,"jobs":jobs}))
    }).await.unwrap()
}
#[tokio::test]
async fn ws15e_fizzy_agent_execution_rechecks_and_records_once() {
    for case in [
        "comment",
        "create",
        "revoked_grant",
        "room_grant",
        "suspended",
        "relinked",
        "disconnected",
        "mismatch",
        "invalid",
        "denied",
        "readonly",
        "revoked_token",
        "probe_failure",
        "refused",
        "audit_failure",
        "legacy_completed",
    ] {
        let app = app().await;
        let (id, agent) = setup(
            &app,
            if case == "create" {
                "create"
            } else {
                "comment"
            },
        )
        .await;
        let case_owned = case.to_owned();
        app.db().write(move|tx| {
            match case_owned.as_str() {
                "revoked_grant"=>{tx.conn().execute("UPDATE agent_grants SET revoked_at=? WHERE agent_id=?",params![tx.now(),agent])?;},
                "room_grant"=>{tx.conn().execute("UPDATE agent_grants SET room_id=? WHERE agent_id=?",params![ALL_TALK,agent])?;},
                "suspended"=>{tx.conn().execute("UPDATE agents SET suspended_at=? WHERE id=?",params![tx.now(),agent])?;},
                "relinked"=>{tx.conn().execute("UPDATE fizzy_connected_accounts SET fizzy_user_id='03someoneelse' WHERE user_id=?",[DAVID])?;},
                "disconnected"=>{Account::for_user(tx.conn(),DAVID)?.unwrap().mark_disconnected(tx,"Rejected")?;},
                "mismatch"=>{tx.conn().execute("UPDATE agent_approvals SET payload=? WHERE id=?",params![json!({"account_id":"897362094","kind":"close","number":579}).to_string(),id])?;},
                "invalid"=>{tx.conn().execute("UPDATE agent_approvals SET payload='not JSON' WHERE id=?",[id])?;},
                "denied"=>{tx.conn().execute("UPDATE agent_approvals SET status='denied' WHERE id=?",[id])?;},
                "audit_failure"=>{tx.conn().execute_batch("CREATE TRIGGER ws15e_audit_failure BEFORE INSERT ON audit_logs WHEN NEW.action='agent.fizzy_action.execute' BEGIN SELECT RAISE(ABORT,'audit down'); END;")?;},
                "legacy_completed"=>{tx.conn().execute("INSERT INTO agent_events (agent_id,event_type,outcome,metadata,created_at) VALUES (?,'fizzy_action_completed','delivered',?,?)",params![agent,json!({"approval_id":id,"status":"completed"}).to_string(),tx.now()])?;},_=>{}
            } Ok(())
        }).await.unwrap();
        let status = match case {
            "readonly" | "revoked_token" | "probe_failure" => 401,
            "refused" => 422,
            _ => 201,
        };
        let probe = match case {
            "revoked_token" => 401,
            "probe_failure" => 500,
            _ => 200,
        };
        let path = if case == "create" {
            "/897362094/boards/03board1/cards.json"
        } else {
            "/897362094/cards/579/comments.json"
        };
        let server = FakeServer::start_ws15e(vec![
            Route::new("POST", "app.fizzy.do", path, status)
                .body("{\"url\":\"https://app.fizzy.do/created\",\"error\":\"Refused\"}"),
            Route::new("GET", "app.fizzy.do", "/my/identity.json", probe).body("{}"),
        ])
        .await;
        let resolver = Arc::new(FakeResolver::new([("app.fizzy.do", vec!["93.184.216.34"])]));
        let dialer = Arc::new(MappingDialer {
            public: ["93.184.216.34".parse().unwrap()].into(),
            to: server.addr,
            dialed: Default::default(),
        });
        let net = network(resolver.clone(), dialer);
        execute(&app.booted.app, &net, "http://app.fizzy.do", id)
            .await
            .unwrap();
        execute(&app.booted.app, &net, "http://app.fizzy.do", id)
            .await
            .unwrap();
        if case == "legacy_completed" {
            assert!(resolver.lookups().is_empty());
            continue;
        }
        let result = state(&app, id).await;
        if case != "audit_failure" {
            let entry=app.db().read(move|c| Ok(c.query_row("SELECT actor_id,target_type,target_id,details FROM audit_logs WHERE action='agent.fizzy_action.execute' AND target_id=?",[id],|r| Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,String>(3)?)))?)).await.unwrap();
            assert_eq!(entry.0, DAVID);
            assert_eq!(entry.1, "AgentApproval");
            assert_eq!(entry.2, id);
            let details: Value = serde_json::from_str(&entry.3).unwrap();
            assert_eq!(details["action"], result["metadata"]["action"]);
            assert_eq!(details["status"], result["metadata"]["status"]);
            assert_eq!(details["url"], result["metadata"]["url"]);
            assert_eq!(details["message"], result["metadata"]["message"]);
        }
        assert_eq!(result["actor"], DAVID);
        assert!(result["room"].is_null());
        assert!(result["message"].is_null());
        assert_eq!(result["outcome"], "delivered");
        assert_eq!(result["webhook"], "pending");
        assert_eq!(result["jobs"], 1);
        assert_eq!(
            result["audits"],
            if case == "audit_failure" { 0 } else { 1 }
        );
        let reason = match case {
            "revoked_grant" | "room_grant" => {
                Some("Agent no longer has the external_action capability")
            }
            "suspended" => Some("Agent is suspended or deactivated"),
            "relinked" => Some("The agent owner's Fizzy account changed since this was approved"),
            "disconnected" => Some("Agent owner has no usable Fizzy account"),
            "mismatch" => Some("Approval summary does not match its payload"),
            "invalid" => Some("Approval payload is invalid"),
            "denied" => Some("Approval is no longer approved"),
            "readonly" => Some(
                "The agent owner's Fizzy token is read-only; card writes need a Read + Write token",
            ),
            "revoked_token" => Some("Fizzy rejected the agent owner's linked token (401)"),
            "probe_failure" => Some("Could not verify the agent owner's Fizzy token; try again"),
            "refused" => Some("Fizzy refused: Refused"),
            _ => None,
        };
        assert_eq!(
            result["metadata"]["status"],
            if reason.is_some() {
                "failed"
            } else {
                "completed"
            }
        );
        if let Some(reason) = reason {
            assert_eq!(result["metadata"]["message"], reason, "{case}");
        } else {
            assert_eq!(result["metadata"]["url"], "https://app.fizzy.do/created");
        }
        let no_http = matches!(
            case,
            "revoked_grant"
                | "room_grant"
                | "suspended"
                | "relinked"
                | "disconnected"
                | "mismatch"
                | "invalid"
                | "denied"
        );
        assert_eq!(
            server.received.lock().unwrap().len(),
            if no_http {
                0
            } else if matches!(case, "readonly" | "revoked_token" | "probe_failure") {
                2
            } else {
                1
            }
        );
        assert_eq!(
            app.db()
                .read(|c| Account::for_user(c, DAVID))
                .await
                .unwrap()
                .unwrap()
                .connected(),
            case != "disconnected" && case != "revoked_token"
        );
        println!("Fizzy agent execution Rails case {case}: 1 passed; 0 failed");
    }
}
#[tokio::test]
async fn ws15e_fizzy_agent_approval_enqueue_hook_is_transactional_and_one_attempt() {
    let app = app().await;
    let (id, _) = setup(&app, "comment").await;
    assert_eq!(PerformJob::retry_policy().attempts, 1);
    app.db()
        .write(move |tx| enqueue_approved(tx, id))
        .await
        .unwrap();
    let jobs = app.db().read(campfire_jobs::inspect::all).await.unwrap();
    assert_eq!(
        jobs.iter().filter(|j| j.class == PerformJob::CLASS).count(),
        1
    );
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agent_approvals SET status='denied' WHERE id=?",
                [id],
            )?;
            enqueue_approved(tx, id)
        })
        .await
        .unwrap();
    let jobs = app.db().read(campfire_jobs::inspect::all).await.unwrap();
    assert_eq!(
        jobs.iter().filter(|j| j.class == PerformJob::CLASS).count(),
        1
    );
    app.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER ws15e_reject_action BEFORE INSERT ON background_jobs WHEN NEW.job_class='Fizzy::PerformAgentActionJob' BEGIN SELECT RAISE(ABORT,'queue down'); END;")?;Ok(())}).await.unwrap();
    assert!(
        app.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE agent_approvals SET status='approved' WHERE id=?",
                    [id],
                )?;
                enqueue_approved(tx, id)
            })
            .await
            .is_err()
    );
    let status = app
        .db()
        .read(move |c| {
            Ok(
                c.query_row("SELECT status FROM agent_approvals WHERE id=?", [id], |r| {
                    r.get::<_, String>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(status, "denied");
    let resolver = Arc::new(FakeResolver::new([]));
    let dialer = Arc::new(MappingDialer {
        public: Default::default(),
        to: "127.0.0.1:51594".parse().unwrap(),
        dialed: Default::default(),
    });
    execute(
        &app.booted.app,
        &network(resolver.clone(), dialer),
        "http://app.fizzy.do",
        123456789,
    )
    .await
    .unwrap();
    assert!(resolver.lookups().is_empty());
}
#[tokio::test]
async fn ws15e_fizzy_agent_worker_loses_to_sweep_and_duplicate_jobs_do_no_http() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for audit_failure in [false, true] {
        let app = app().await;
        let (id, _) = setup(&app, "comment").await;
        let listener = crate::integrations::test_support::ws15e_listener().await;
        let address = listener.local_addr().unwrap();
        let ready = Arc::new(tokio::sync::Notify::new());
        let signal = ready.clone();
        let (release, hold) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert!(socket.read(&mut [0; 8192]).await.unwrap() > 0);
            signal.notify_one();
            hold.await.unwrap();
            let body = b"{\"url\":\"https://app.fizzy.do/late\"}";
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 201 Created\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            socket.write_all(body).await.unwrap();
        });
        let resolver = Arc::new(FakeResolver::new([("app.fizzy.do", vec!["93.184.216.34"])]));
        let dialer = Arc::new(MappingDialer {
            public: ["93.184.216.34".parse().unwrap()].into(),
            to: address,
            dialed: Default::default(),
        });
        let net = network(resolver.clone(), dialer);
        let task_app = app.booted.app.clone();
        let task_net = net.clone();
        let worker =
            tokio::spawn(
                async move { execute(&task_app, &task_net, "http://app.fizzy.do", id).await },
            );
        tokio::time::timeout(std::time::Duration::from_secs(5), ready.notified())
            .await
            .unwrap();
        execute(&app.booted.app, &net, "http://app.fizzy.do", id)
            .await
            .unwrap();
        app.db().write(move|tx| {
            tx.conn().execute("UPDATE agent_events SET created_at=? WHERE agent_approval_id=?",params![tx.now().ago(jiff::SignedDuration::from_mins(20)),id])?;
            if audit_failure {tx.conn().execute_batch("CREATE TRIGGER ws15e_sweep_audit_failure BEFORE INSERT ON audit_logs WHEN NEW.action='agent.fizzy_action.execute' BEGIN SELECT RAISE(ABORT,'audit down'); END;")?;}
            Ok(())
        }).await.unwrap();
        assert_eq!(
            action_claims::recover_stuck_claims(app.db(), FIZZY, app.db().env().now()).await,
            1
        );
        let before = state(&app, id).await;
        release.send(()).unwrap();
        worker.await.unwrap().unwrap();
        server.await.unwrap();
        assert_eq!(state(&app, id).await, before);
        assert_eq!(
            before["metadata"]["message"],
            "Fizzy action execution timed out"
        );
        assert_eq!(before["audits"], if audit_failure { 0 } else { 1 });
        assert_eq!(before["jobs"], 1);
        assert_eq!(resolver.lookups().len(), 1);
    }
}
#[tokio::test]
async fn ws15e_fizzy_agent_outcome_enqueue_failure_rolls_back_to_running_claim() {
    let app = app().await;
    let (id, _) = setup(&app, "comment").await;
    app.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER ws15e_reject_outcome BEFORE INSERT ON background_jobs WHEN NEW.job_class='Agent::EventWebhookJob' BEGIN SELECT RAISE(ABORT,'queue down'); END;")?;Ok(())}).await.unwrap();
    let server = FakeServer::start_ws15e(vec![
        Route::new(
            "POST",
            "app.fizzy.do",
            "/897362094/cards/579/comments.json",
            201,
        )
        .body("{}"),
    ])
    .await;
    let resolver = Arc::new(FakeResolver::new([("app.fizzy.do", vec!["93.184.216.34"])]));
    let dialer = Arc::new(MappingDialer {
        public: ["93.184.216.34".parse().unwrap()].into(),
        to: server.addr,
        dialed: Default::default(),
    });
    let net = network(resolver, dialer);
    assert!(
        execute(&app.booted.app, &net, "http://app.fizzy.do", id)
            .await
            .is_err()
    );
    let state = state(&app, id).await;
    assert_eq!(state["metadata"]["status"], "running");
    assert_eq!(state["webhook"], "none");
    assert_eq!(state["jobs"], 0);
    assert_eq!(state["audits"], 0);
    execute(&app.booted.app, &net, "http://app.fizzy.do", id)
        .await
        .unwrap();
    assert_eq!(server.received.lock().unwrap().len(), 1);
}
