//! The production seam calls the existing linked-account domain over actual TLS.
use super::*;
use crate::controllers::presenters::test_support::{BENDER, DAVID, SEED_NOW, TestApp};
use crate::integrations::{
    net::Network,
    test_support::{FakeResolver, FakeServer, MappingDialer, Route},
};
use serde_json::{Value, json};
use std::sync::Arc;
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/agents_repository_adapter_contract.json"
    ))
    .unwrap()
}

#[tokio::test]
async fn ws11_kill_case_approved_github_job_rechecks_before_http() {
    use campfire_db::{AgentApproval, Event, NewApproval};
    let test = TestApp::boot_with_clock(Arc::new(campfire_kit::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    )))
    .await
    .expect("default seed");
    let app = test.booted.app.clone();
    let (agent, approval)=app.db.write(|tx| {
        let agent=Agent::for_user(tx.conn(),BENDER)?.unwrap();
        let approval=AgentApproval::create(tx,NewApproval {agent_id:agent.id,room_id:Some(crate::controllers::presenters::test_support::ALL_TALK),action:"github.comment".into(),summary:"Comment".into(),..Default::default()})?;
        tx.conn().execute("UPDATE agent_approvals SET status='approved',decided_by_id=?,decided_at=? WHERE id=?",rusqlite::params![DAVID,tx.now(),approval.id])?;
        campfire_db::models::agent_lifecycle::kill_switch(tx,agent.id,&Default::default())?;
        tx.emit_after_commit(Event::job(&crate::integrations::github::jobs::PerformAgentActionJob {approval_id:approval.id}));
        Ok((agent.id,approval.id))
    }).await.unwrap();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let rows=app.db.read(move|c| {
            let mut q=c.prepare("SELECT metadata FROM agent_events WHERE agent_id=? AND agent_approval_id=? AND event_type='github_action_completed'")?;
            Ok(q.query_map(rusqlite::params![agent,approval],|r|r.get::<_,Value>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
        }).await.unwrap();
        if !rows.is_empty() {
            assert_eq!(rows.len(), 1);
            let all: Value = serde_json::from_str(include_str!(
                "../../../../../vectors/agents_security_lifecycle_cases.json"
            ))
            .unwrap();
            assert_eq!(
                json!({"status":rows[0]["status"],"message":rows[0]["message"],"events":rows.len()}),
                all["cases"]["kill_external"]
            );
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "registered durable GitHub job did not finish"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    test.booted
        .jobs
        .shutdown(std::time::Duration::from_secs(1))
        .await;
}

#[tokio::test]
async fn ws11_installed_github_reference_hook_follows_stream_claims_and_quiet_finalization() {
    use campfire_db::{Message, MessageChanges, NewMessage};
    let (app, _dir) = TestApp::boot_with_clock(Arc::new(campfire_kit::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    )))
    .await
    .expect("default seed")
    .stop_jobs()
    .await;
    let gold: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/agents_github_stream_peer_contract.json"
    ))
    .unwrap();
    app.db.write(move|tx| {
        let snapshot=|tx:&campfire_db::Tx<'_>,message:&Message,claimed:Option<bool>|->campfire_db::Result<Value> {
            let mut q=tx.conn().prepare("SELECT p.number FROM github_pull_request_references r JOIN github_pull_requests p ON p.id=r.github_pull_request_id WHERE r.message_id=? ORDER BY p.number")?;
            let refs=q.query_map([message.id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let jobs=tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'",[],|r|r.get::<_,i64>(0))?;
            Ok(json!({"claimed":claimed,"streaming":message.streaming,"references":refs,"fetches":jobs}))
        };
        let attributes=|key:&str|NewMessage {room_id:crate::controllers::presenters::test_support::ALL_TALK,creator_id:BENDER,streaming:true,client_message_id:Some(key.into()),..Default::default()};
        let mut message=Message::create_markdown(tx,attributes("ws11-github-stream"),"https://github.com/ws11-fixture/public/pull/1")?;
        assert_eq!(snapshot(tx,&message,None)?,gold["results"]["start"]);
        message.update(tx,MessageChanges {markdown_source:Some("https://github.com/ws11-fixture/public/pull/2".into()),..Default::default()})?;
        assert_eq!(snapshot(tx,&message,None)?,gold["results"]["append"]);
        let claimed=message.finalize_stream(tx)?;
        assert_eq!(snapshot(tx,&message,Some(claimed))?,gold["results"]["final"]);
        let claimed=message.finalize_stream(tx)?;
        assert_eq!(snapshot(tx,&message,Some(claimed))?,gold["results"]["repeat"]);
        let mut quiet=Message::create_markdown(tx,attributes("ws11-github-quiet"),"https://github.com/ws11-fixture/public/pull/3")?;
        let claimed=quiet.finalize_stream_quietly(tx)?;
        assert_eq!(snapshot(tx,&quiet,Some(claimed))?,gold["results"]["quiet"]);
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn ws11_live_repository_adapter_matches_rails_allow_denial_disconnect_and_retry() {
    for status in [403, 404, 401, 500, 200] {
        let (server, roots) = FakeServer::start_named_tls_ws15e(
            vec![Route::new("GET", "api.github.com", "/repos/mixed/repo", status).body("{}")],
            vec!["api.github.com".into()],
        )
        .await;
        let net = Network {
            resolver: Arc::new(FakeResolver::new([(
                "api.github.com",
                vec!["93.184.216.34"],
            )])),
            dialer: Arc::new(MappingDialer {
                public: ["93.184.216.34".parse().unwrap()].into(),
                to: server.addr,
                dialed: Default::default(),
            }),
            tls: crate::integrations::net::tls_config(roots),
        };
        let (app, _dir) = TestApp::boot_with_github_network(net)
            .await
            .expect("default seed")
            .stop_jobs()
            .await;
        let (agent, thread) = super::tests::setup(&app.db, app.ar_encryption.clone()).await;
        let account = app
            .db
            .read(|c| {
                Ok(c.query_row(
                    "SELECT id FROM github_connected_accounts WHERE user_id=?",
                    [DAVID],
                    |r| r.get::<_, i64>(0),
                )?)
            })
            .await
            .unwrap();
        // Prime through the ordinary GitHub owner service, then use the boot-installed
        // agent adapter. Rails uses one cache, including case-normalized denials.
        let direct = app
            .github_accounts
            .can_read_repository(account, "MIXED", "REPO")
            .await
            .unwrap();
        let mut decisions = vec![];
        for index in 0..2 {
            let access = if index == 0 {
                let mut access = RepositoryAccess::default();
                if direct {
                    access.insert((DAVID, "mixed".into(), "repo".into()));
                }
                access
            } else {
                app.agent_repositories
                    .resolve_threads(&app.db, agent, vec![thread, thread])
                    .await
                    .unwrap()
            };
            decisions.push(access.contains(&(DAVID, "mixed".into(), "repo".into())));
            let accessible = app
                .db
                .read(move |c| {
                    Ok(campfire_db::models::agent_payloads::work_payload(
                        c,
                        &campfire_db::ChannelThread::find(c, thread)?,
                        Some(DAVID),
                        &access,
                    )?["links"]
                        .clone())
                })
                .await
                .unwrap();
            assert_eq!(
                accessible,
                oracle()["cases"][status.to_string()]["links"],
                "Rails private/unknown redaction and public details"
            );
        }
        let reason = app
            .db
            .read(|c| {
                Ok(c.query_row(
                    "SELECT disconnected_reason FROM github_connected_accounts WHERE user_id=?",
                    [DAVID],
                    |r| r.get::<_, Option<String>>(0),
                )?)
            })
            .await
            .unwrap();
        let paths = server
            .received()
            .iter()
            .map(|r| r.target.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            json!({"decisions":decisions,"paths":paths,"disconnected_reason":reason}),
            {
                let mut expected = oracle()["cases"][status.to_string()].clone();
                expected.as_object_mut().unwrap().remove("links");
                expected
            }
        );
        assert_eq!(
            app.db
                .read(move |c| Ok(Agent::for_user(c, BENDER)?.unwrap().owner_id))
                .await
                .unwrap(),
            Some(DAVID)
        );
    }
}
