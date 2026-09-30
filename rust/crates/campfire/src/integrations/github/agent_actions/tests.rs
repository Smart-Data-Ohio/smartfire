use super::*;
use crate::integrations::{
    github::{
        client::AppClient,
        tests::{crypto, fake},
    },
    test_support::{Received, Route, TestDb},
};
use campfire_db::{BasicRichText, Env, Event, EventSink, TestClock, Timestamp, Tx};
use campfire_jobs::{JobQueue, QueueConfig, Registry, RunnerConfig};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use std::sync::Arc;

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/github_actions.json"
    ))
    .unwrap()
}
struct QueueSink(JobQueue);
impl EventSink for QueueSink {
    fn emit(&self, _: Event) {}
    fn persist(&self, tx: &Tx<'_>, event: &Event) -> campfire_db::Result<()> {
        if let Event::Job(request) = event {
            self.0.enqueue(tx, request)?;
        }
        Ok(())
    }
}
async fn database(case: &Value) -> TestDb {
    let queue = JobQueue::new(
        &Registry::<()>::new(),
        &RunnerConfig::new(vec![QueueConfig::new("default", 1)]),
    )
    .unwrap();
    let env = Env {
        clock: Arc::new(TestClock::frozen_at(Timestamp::from_jiff(
            "2026-01-01T12:00:00Z".parse().unwrap(),
        ))),
        sink: Arc::new(QueueSink(queue)),
        rich_text: Arc::new(BasicRichText),
        bcrypt_cost: 4,
        ..Default::default()
    };
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws15g");
    let fixture = tokio::task::spawn_blocking(move || TestDb::with_env(env, &directory))
        .await
        .unwrap();
    seed(&fixture.db, case).await;
    fixture
}
async fn seed(db: &Database, case: &Value) {
    let case = case.clone();
    db.write(move |tx| {
        let now = tx.now();
        tx.conn().execute("INSERT INTO users (id,name,role,created_at,updated_at) VALUES (810,'Machine',2,?,?)", params![now,now])?;
        tx.conn().execute("INSERT INTO users (id,name,email_address,role,created_at,updated_at) VALUES (811,'Oracle','oracle@example.test',1,?,?)", params![now,now])?;
        tx.conn().execute("INSERT INTO agents (id,user_id,owner_id,created_at,updated_at) VALUES (812,810,811,?,?)", params![now,now])?;
        tx.conn().execute("INSERT INTO rooms (id,type,creator_id,name,created_at,updated_at) VALUES (815,'Rooms::Closed',811,'Action room',?,?)", params![now,now])?;
        tx.conn().execute("INSERT INTO memberships (user_id,room_id,created_at,updated_at) VALUES (810,815,?,?)", params![now,now])?;
        tx.conn().execute("INSERT INTO messages (id,creator_id,room_id,client_message_id,created_at,updated_at) VALUES (818,811,815,'fixture-parent',?,?)", params![now,now])?;
        tx.conn().execute("INSERT INTO channel_threads (id,creator_id,room_id,parent_message_id,name,last_activity_at,created_at,updated_at) VALUES (817,811,815,818,'Discussion',?,?,?)", params![now,now,now])?;
        tx.conn().execute("INSERT INTO github_pull_requests (id,owner,repo,number,created_at,updated_at) VALUES (816,'rails','rails',12,?,?)", params![now,now])?;
        tx.conn().execute("INSERT INTO github_pull_request_threads (github_pull_request_id,room_id,channel_thread_id,created_at,updated_at) VALUES (816,815,817,?,?)", params![now,now])?;
        tx.conn().execute("INSERT INTO agent_grants (id,agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES (820,812,815,811,'external_action',?,?)", params![now,now])?;
        let access=crypto().encrypt("fixture-agent-token");
        tx.conn().execute("INSERT INTO github_connected_accounts (id,user_id,github_login,access_token,token_source,created_at,updated_at) VALUES (819,810,'machine',?,'pat',?,?)", params![access,now,now])?;
        let a=&case["approval"];
        tx.conn().execute("INSERT INTO agent_approvals (id,agent_id,room_id,action,summary,payload,status,github_account_id,github_login,decided_by_id,expires_at,created_at,updated_at) VALUES (813,812,?,?,?,?,?,?,?,811,?,?,?)", params![a["room_id"].as_i64(),a["action"].as_str(),a["summary"].as_str(),a["payload"].as_str(),a["status"].as_str(),a["github_account_id"].as_i64(),a["github_login"].as_str(),now.since(jiff::SignedDuration::from_hours(24)),now,now])?;
        for (key, sql) in [
            ("suspended", "UPDATE agents SET suspended_at=CURRENT_TIMESTAMP WHERE id=812"),
            ("deactivated", "UPDATE users SET status=1 WHERE id=810"),
            ("soft_deleted", "UPDATE rooms SET deleted_at=CURRENT_TIMESTAMP WHERE id=815"),
            ("membership_removed", "DELETE FROM memberships WHERE user_id=810 AND room_id=815"),
            ("grant_revoked", "UPDATE agent_grants SET revoked_at=CURRENT_TIMESTAMP WHERE id=820"),
            ("legacy", "DELETE FROM agent_grants WHERE agent_id=812"),
            ("workspace_grant", "UPDATE agent_grants SET room_id=NULL WHERE id=820"),
            ("mapping_removed", "DELETE FROM github_pull_request_threads WHERE github_pull_request_id=816"),
            ("relinked", "UPDATE github_connected_accounts SET github_login='someone-else' WHERE id=819"),
            ("disconnected", "UPDATE github_connected_accounts SET disconnected_reason='Disconnected fixture' WHERE id=819"),
            ("destroyed", "DELETE FROM github_connected_accounts WHERE id=819"),
            ("unreadable", "UPDATE github_connected_accounts SET access_token='bogus-ciphertext' WHERE id=819"),
            ("missing_approval", "DELETE FROM agent_approvals WHERE id=813"),
        ] { if case[key]==true {tx.conn().execute(sql,[])?;} }
        if case["expired"]==true { tx.conn().execute("UPDATE agent_approvals SET expires_at=? WHERE id=813",[now.ago(jiff::SignedDuration::from_mins(1))])?; }
        if case["replaced"]==true { tx.conn().execute("UPDATE github_connected_accounts SET id=823 WHERE id=819",[])?; }
        if case["owner_app"]==true || case["owner_pat"]==true {
            tx.conn().execute("INSERT INTO github_connected_accounts (id,user_id,github_login,access_token,refresh_token,token_expires_at,token_source,created_at,updated_at) VALUES (821,811,'owner',?,?,?,?,?,?)",params![crypto().encrypt("fixture-owner-token"),crypto().encrypt("fixture-refresh"),now.since(jiff::SignedDuration::from_hours(1)),if case["owner_app"]==true {"app"} else {"pat"},now,now])?;
        }
        if case["pr_swapped"]==true {
            tx.conn().execute("INSERT INTO github_pull_requests (id,owner,repo,number,created_at,updated_at) VALUES (822,'other','repo',13,?,?)",params![now,now])?;
            tx.conn().execute("UPDATE github_pull_request_threads SET github_pull_request_id=822 WHERE github_pull_request_id=816",[])?;
        }
        if case["historical"]==true || case["running"]==true {
            tx.conn().execute("INSERT INTO agent_events (id,agent_id,agent_approval_id,room_id,event_type,outcome,metadata,created_at) VALUES (814,812,?,815,'github_action_completed','delivered',?,?)",params![if case["historical"]==true {None} else {Some(813)},json!({"approval_id":813,"action":a["action"],"status":if case["running"]==true {"running"} else {"completed"}}).to_string(),now])?;
        }
        if case["webhook"]==true {tx.conn().execute("INSERT INTO webhooks (user_id,url,created_at,updated_at) VALUES (810,'https://example.com/hooks',?,?)",params![now,now])?;}
        if case["audit_failure"]==true {tx.conn().execute_batch("CREATE TRIGGER fail_action_audit BEFORE INSERT ON audit_logs BEGIN SELECT RAISE(ABORT,'fixture audit down'); END;")?;}
        Ok(())
    }).await.unwrap();
}
async fn snapshot(db: &Database) -> Value {
    db.read(|conn| {
        let mut stmt=conn.prepare("SELECT agent_id,agent_approval_id,room_id,actor_id,outcome,detail,event_type,metadata,webhook_status,webhook_attempts,hop FROM agent_events WHERE agent_id=812 ORDER BY id")?;
        let events:Vec<Value>=stmt.query_map([],|r|Ok(json!({"agent_id":r.get::<_,i64>(0)?,"agent_approval_id":r.get::<_,Option<i64>>(1)?,"room_id":r.get::<_,Option<i64>>(2)?,"actor_id":r.get::<_,Option<i64>>(3)?,"outcome":r.get::<_,Option<String>>(4)?,"detail":r.get::<_,Option<String>>(5)?,"event_type":r.get::<_,String>(6)?,"metadata":serde_json::from_str::<Value>(&r.get::<_,String>(7)?).unwrap(),"webhook_status":r.get::<_,String>(8)?,"webhook_attempts":r.get::<_,i64>(9)?,"hop":r.get::<_,i64>(10)?})))?.collect::<rusqlite::Result<_>>()?;
        let mut stmt=conn.prepare("SELECT action,actor_id,actor_label,target_type,target_id,target_label,details FROM audit_logs WHERE action='agent.github_action.execute' ORDER BY id")?;
        let audits:Vec<Value>=stmt.query_map([],|r|Ok(json!({"action":r.get::<_,String>(0)?,"actor_id":r.get::<_,Option<i64>>(1)?,"actor_label":r.get::<_,Option<String>>(2)?,"target_type":r.get::<_,String>(3)?,"target_id":r.get::<_,i64>(4)?,"target_label":r.get::<_,String>(5)?,"details":serde_json::from_str::<Value>(&r.get::<_,String>(6)?).unwrap()})))?.collect::<rusqlite::Result<_>>()?;
        let mut stmt=conn.prepare("SELECT job_class,arguments FROM background_jobs ORDER BY id")?;
        let jobs:Vec<Value>=stmt.query_map([],|r|{let args:Value=serde_json::from_str(&r.get::<_,String>(1)?).unwrap(); Ok(json!({"class":r.get::<_,String>(0)?,"attempt":args["attempt"]}))})?.collect::<rusqlite::Result<_>>()?;
        let disconnected:Option<String>=conn.query_row("SELECT disconnected_reason FROM github_connected_accounts WHERE id=819",[],|r|r.get(0)).optional()?.flatten();
        Ok(json!({"events":events,"audits":audits,"jobs":jobs,"disconnected_reason":disconnected}))
    }).await.unwrap()
}
fn request_snapshot(received: &[Received], owner: bool) -> Value {
    json!(received.iter().map(|r|json!({"method":r.method,"path":r.target,"body":serde_json::from_slice::<Value>(&r.body).unwrap(),"agent_authorization":r.header("Authorization")==Some(format!("Bearer {}",if owner {"fixture-owner-token"}else{"fixture-agent-token"}).as_str())})).collect::<Vec<_>>())
}
async fn run_case(case: &Value) {
    let fixture = database(case).await;
    let path = match case["kind"].as_str().unwrap_or("comment") {
        "approve" | "request_changes" => "/repos/rails/rails/pulls/12/reviews",
        "request_review" => "/repos/rails/rails/pulls/12/requested_reviewers",
        _ => "/repos/rails/rails/issues/12/comments",
    };
    let mut route = Route::new(
        "POST",
        "api.github.com",
        path,
        case["code"].as_u64().unwrap_or(201) as u16,
    )
    .body(
        case.get("response")
            .cloned()
            .unwrap_or(json!({"html_url":"https://github.com/rails/rails/pull/12#fixture"}))
            .to_string(),
    );
    if case["destroy_during_http"] == true {
        route.delay = std::time::Duration::from_millis(150);
    }
    let (server, network) = fake(vec![route]).await;
    let accounts = Accounts::with_network(
        fixture.db.clone(),
        crypto(),
        AppClient::new(None, None),
        network,
    );
    if case["destroy_during_http"] == true {
        let db = fixture.db.clone();
        let accounts = accounts.clone();
        let worker = tokio::spawn(async move { perform(&db, &accounts, 813).await });
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while server.received().is_empty() {
            assert!(tokio::time::Instant::now() < deadline);
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }
        fixture
            .db
            .write(|tx| {
                tx.conn()
                    .execute("DELETE FROM github_connected_accounts WHERE id=819", [])?;
                Ok(())
            })
            .await
            .unwrap();
        worker.await.unwrap().unwrap();
    } else {
        perform(&fixture.db, &accounts, 813).await.unwrap();
    }
    if case["twice"] == true {
        perform(&fixture.db, &accounts, 813).await.unwrap();
    }
    let mut observed = snapshot(&fixture.db).await;
    observed["received"] = request_snapshot(&server.received(), case["owner_app"] == true);
    assert_eq!(observed, case["expected"], "{}", case["name"]);
}
#[tokio::test]
async fn github_agent_rechecks_authority_before_any_write() {
    for case in vectors()["cases"].as_array().unwrap().iter().filter(|c| {
        [
            "denied",
            "pending_expired",
            "cancelled",
            "suspended",
            "deactivated",
            "missing_room",
            "soft_deleted_room",
            "membership_removed",
            "grant_revoked",
            "legacy_without_grants",
            "mapping_removed",
        ]
        .contains(&c["name"].as_str().unwrap())
    }) {
        run_case(case).await;
    }
}
#[tokio::test]
async fn github_agent_rechecks_payload_and_linked_identity_before_any_write() {
    for case in vectors()["cases"].as_array().unwrap().iter().filter(|c| {
        [
            "no_payload",
            "invalid_json",
            "array_payload",
            "missing_pr",
            "invalid_body",
            "invalid_reviewers",
            "summary_swapped",
            "action_swapped",
            "pr_swapped",
            "account_relinked",
            "account_replaced",
            "missing_identity",
            "disconnected",
            "destroyed",
            "unreadable_access",
        ]
        .contains(&c["name"].as_str().unwrap())
    }) {
        run_case(case).await;
    }
}
#[tokio::test]
async fn github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails() {
    for case in vectors()["cases"].as_array().unwrap() {
        run_case(case).await;
    }
}
#[test]
fn github_agent_action_validation_summary_payload_and_normalization_match_rails() {
    use crate::integrations::github::{actions::Action, client::PullRequestKey};
    for case in vectors()["validation"].as_array().unwrap() {
        let action = Action::from_payload(
            816,
            PullRequestKey {
                owner: "rails".into(),
                repo: "rails".into(),
                number: 12,
            },
            case,
        );
        let errors = action.errors();
        let mut grouped = serde_json::Map::new();
        for (key, msg) in &errors.0 {
            grouped
                .entry(key.to_string())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .unwrap()
                .push(json!(msg));
        }
        let observed = json!({"valid":errors.is_empty(),"errors":grouped,"body":action.normalized_body(),"reviewers":action.normalized_reviewers(),"action":action.action_name(),"summary":action.summary(),"payload":action.payload(),"payload_json":action.payload_json()});
        assert_eq!(observed, case["expected"], "kind={}", case["kind"]);
    }
}
#[tokio::test]
async fn github_agent_concurrent_duplicates_share_one_claim_and_one_outbound_write() {
    let case = &vectors()["cases"][0];
    let fixture = database(case).await;
    let mut route = Route::new(
        "POST",
        "api.github.com",
        "/repos/rails/rails/issues/12/comments",
        201,
    )
    .body(json!({"html_url":"https://github.com/rails/rails/pull/12#fixture"}).to_string());
    route.delay = std::time::Duration::from_millis(250);
    let (server, network) = fake(vec![route]).await;
    let accounts = Accounts::with_network(
        fixture.db.clone(),
        crypto(),
        AppClient::new(None, None),
        network,
    );
    let mut jobs = tokio::task::JoinSet::new();
    for _ in 0..24 {
        let db = fixture.db.clone();
        let accounts = accounts.clone();
        jobs.spawn(async move {
            perform(&db, &accounts, 813).await.unwrap();
        });
    }
    while let Some(result) = jobs.join_next().await {
        result.unwrap();
    }
    assert_eq!(server.received().len(), 1);
    let observed = snapshot(&fixture.db).await;
    assert_eq!(observed["events"], case["expected"]["events"]);
    assert_eq!(observed["audits"], case["expected"]["audits"]);
}
#[test]
fn github_agent_and_notifier_declare_rails_retry_policies() {
    use crate::integrations::github::jobs::{DeliverSubscriptionEventJob, PerformAgentActionJob};
    use campfire_jobs::JobKind;
    assert_eq!(PerformAgentActionJob::retry_policy().attempts, 1);
    assert_eq!(
        PerformAgentActionJob::retry_policy().retry_delay(1, None, 0.5),
        None
    );
    assert_eq!(DeliverSubscriptionEventJob::retry_policy().attempts, 5);
}
#[tokio::test]
async fn github_agent_runtime_performs_discards_and_leaves_terminal_claim_on_writer_failure() {
    use crate::integrations::github::{client::ReadClient, jobs::PerformAgentActionJob};
    for mode in [
        "success",
        "missing",
        "malformed",
        "persist_failure",
        "unauthorized",
    ] {
        let case = &vectors()["cases"][0];
        let code = if mode == "unauthorized" { 401 } else { 201 };
        let (server, network) = fake(vec![
            Route::new(
                "POST",
                "api.github.com",
                "/repos/rails/rails/issues/12/comments",
                code,
            )
            .body(json!({"html_url":"https://github.com/rails/rails/pull/12#fixture"}).to_string()),
        ])
        .await;
        let scratch =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws15g");
        let dir = tempfile::tempdir_in(scratch).unwrap();
        let fixture_secret: Value =
            serde_json::from_str(include_str!("../../../../../../vectors/github.json")).unwrap();
        let config = crate::config::Config::from_lookup(|key| match key {
            "SECRET_KEY_BASE" => Some(fixture_secret["secret_key_base"].as_str().unwrap().into()),
            "CAMPFIRE_STORAGE_PATH" => Some(dir.path().to_string_lossy().into_owned()),
            "DISABLE_SSL" => Some("1".into()),
            _ => None,
        })
        .unwrap();
        let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
            "2026-01-01T12:00:00Z".parse().unwrap(),
        ));
        let booted = crate::app::boot_with_github_network(
            config,
            clock,
            ReadClient::with_network(None, network.clone()),
            network,
        )
        .await
        .unwrap();
        if mode != "missing" && mode != "malformed" {
            seed(&booted.app.db, case).await;
        }
        booted.app.db.write(move |tx| {
            if mode=="persist_failure" {tx.conn().execute_batch("CREATE TRIGGER fail_action_finish BEFORE UPDATE ON agent_events BEGIN SELECT RAISE(ABORT,'fixture finish failed'); END;")?;}
            let mut request=campfire_db::JobRequest::new(&PerformAgentActionJob{approval_id:813});
            if mode=="malformed" {request.arguments=json!({"approval_id":"bad"});}
            tx.emit_after_commit(Event::Job(request));Ok(())
        }).await.unwrap();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let jobs = booted
                .app
                .db
                .read(campfire_jobs::inspect::all)
                .await
                .unwrap();
            if mode == "persist_failure" {
                if jobs.len() == 1 && jobs[0].status == campfire_jobs::FAILED {
                    assert_eq!(jobs[0].attempts, 1);
                    assert_eq!(jobs[0].run_at, jobs[0].created_at);
                    assert!(
                        jobs[0]
                            .last_error
                            .as_deref()
                            .unwrap()
                            .contains("fixture finish failed")
                    );
                    break;
                }
            } else if jobs.is_empty() {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "runtime {mode}: {jobs:?}"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        if mode == "success" {
            let mut observed = snapshot(&booted.app.db).await;
            observed["received"] = request_snapshot(&server.received(), false);
            assert_eq!(observed, case["expected"]);
        }
        if mode == "missing" || mode == "malformed" {
            assert!(server.received().is_empty());
        }
        if mode == "persist_failure" {
            assert_eq!(
                snapshot(&booted.app.db).await["events"][0]["metadata"]["status"],
                "running"
            );
            assert_eq!(server.received().len(), 1);
            perform(&booted.app.db, &booted.app.github_accounts, 813)
                .await
                .unwrap();
            assert_eq!(server.received().len(), 1);
        }
        if mode == "unauthorized" {
            assert_eq!(
                snapshot(&booted.app.db).await["events"][0]["metadata"]["status"],
                "failed"
            );
            assert_eq!(server.received().len(), 1);
        }
        booted
            .jobs
            .shutdown(std::time::Duration::from_secs(1))
            .await;
    }
}
#[tokio::test]
async fn github_agent_late_finish_audit_snapshot_and_queue_rollback_use_the_real_http_boundary() {
    use crate::integrations::action_claims;
    for mode in ["sweep", "audit_snapshot", "queue_failure"] {
        let case = vectors()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "webhook")
            .unwrap()
            .clone();
        let fixture = database(&case).await;
        let mut route = Route::new(
            "POST",
            "api.github.com",
            "/repos/rails/rails/issues/12/comments",
            201,
        )
        .body(json!({"html_url":"https://github.com/rails/rails/pull/12#fixture"}).to_string());
        route.delay = std::time::Duration::from_millis(250);
        let (server, network) = fake(vec![route]).await;
        let accounts = Accounts::with_network(
            fixture.db.clone(),
            crypto(),
            AppClient::new(None, None),
            network,
        );
        let db = fixture.db.clone();
        let worker = tokio::spawn(async move { perform(&db, &accounts, 813).await });
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while server.received().is_empty() {
            assert!(tokio::time::Instant::now() < deadline);
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }
        // The writer is available while HTTP is pending and the running claim is already durable.
        assert_eq!(
            snapshot(&fixture.db).await["events"][0]["metadata"]["status"],
            "running"
        );
        fixture.db.write(move |tx| {
            match mode {
                "sweep"=>{tx.conn().execute("UPDATE agent_events SET created_at=? WHERE agent_id=812",[tx.now().ago(jiff::SignedDuration::from_mins(16))])?;},
                "audit_snapshot"=>{tx.conn().execute("UPDATE users SET name='Renamed' WHERE id=811",[])?;tx.conn().execute("UPDATE agent_approvals SET action='github.approve',summary='Rewritten' WHERE id=813",[])?;},
                "queue_failure"=>tx.conn().execute_batch("CREATE TRIGGER reject_action_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'fixture queue down'); END;")?,
                _=>unreachable!(),
            }
            Ok(())
        }).await.unwrap();
        if mode == "sweep" {
            assert_eq!(
                action_claims::recover_stuck_claims(
                    &fixture.db,
                    action_claims::GITHUB,
                    fixture.db.env().now()
                )
                .await,
                1
            );
        }
        let result = worker.await.unwrap();
        let observed = snapshot(&fixture.db).await;
        assert_eq!(server.received().len(), 1);
        if mode == "queue_failure" {
            assert!(result.is_err());
            assert_eq!(observed["events"][0]["metadata"]["status"], "running");
            assert_eq!(observed["events"][0]["webhook_status"], "none");
            assert_eq!(observed["audits"], json!([]));
            assert_eq!(observed["jobs"], json!([]));
        } else {
            result.unwrap();
            assert_eq!(observed["audits"].as_array().unwrap().len(), 1);
            assert_eq!(observed["jobs"].as_array().unwrap().len(), 1);
            if mode == "sweep" {
                assert_eq!(observed["events"][0]["metadata"]["status"], "failed");
                assert_eq!(
                    observed["events"][0]["metadata"]["message"],
                    action_claims::GITHUB.timeout_message
                );
                assert!(observed["events"][0]["metadata"].get("url").is_none());
            } else {
                assert_eq!(observed["audits"], case["expected"]["audits"]);
                assert_eq!(observed["events"], case["expected"]["events"]);
            }
        }
    }
}
#[tokio::test]
async fn github_agent_completion_index_is_scoped_to_agent_approval_and_event_type() {
    let fixture = database(&vectors()["cases"][0]).await;
    fixture.db.write(|tx| {
        let insert=|kind,agent_id|tx.conn().execute("INSERT INTO agent_events (agent_id,agent_approval_id,event_type,outcome,created_at) VALUES (?,813,?,'delivered',?)",params![agent_id,kind,tx.now()]);
        insert("approval_decided",812)?;
        insert("github_action_completed",812)?;
        let error=campfire_db::Error::from(insert("github_action_completed",812).unwrap_err());
        assert!(error.is_record_not_unique());
        insert("fizzy_action_completed",812)?;
        tx.conn().execute("INSERT INTO users (id,name,role,created_at,updated_at) VALUES (825,'Other machine',2,?,?)",params![tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO agents (id,user_id,owner_id,created_at,updated_at) VALUES (824,825,811,?,?)",params![tx.now(),tx.now()])?;
        insert("github_action_completed",824)?;
        Ok(())
    }).await.unwrap();
}
