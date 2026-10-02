//! UI decision -> persisted queue argument -> WS15g job -> real fake-server HTTP.
//! Expectations are independently asserted by the pinned execution_bridge.rb probe.
use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, DAVID, Req, TestApp};
use crate::integrations::github::{
    accounts::{Account, AccountInput},
    agent_actions,
    tests::fake,
};
use crate::integrations::test_support::Route;
use axum::http::{Method, StatusCode};
use campfire_db::{
    Agent, AgentApproval, AgentChanges, AgentCredential, AgentGrant, ChannelThread, NewApproval,
    NewChannelThread, NewGrant, User,
};
use rails_compat::ar_encryption::ArEncryption;
use rusqlite::params;
use serde_json::{Value, json};

#[tokio::test]
async fn ws11ui_human_github_decision_executes_real_transport_and_rechecks_later_changes() {
    for ((case, expected_status, expected_message, calls), html) in [
        ("success", "completed", None, 1),
        (
            "identity_changed",
            "failed",
            Some("The agent's GitHub account changed since this was approved"),
            0,
        ),
        (
            "grant_revoked",
            "failed",
            Some("Agent no longer has the external_action capability"),
            0,
        ),
        ("credential_revoked", "completed", None, 1),
        (
            "suspended",
            "failed",
            Some("Agent is suspended or deactivated"),
            0,
        ),
        (
            "deactivated",
            "failed",
            Some("Agent is suspended or deactivated"),
            0,
        ),
    ]
    .into_iter()
    .flat_map(|case| [(case, false), (case, true)])
    {
        let (server, network) = fake(vec![
            Route::new(
                "POST",
                "api.github.com",
                "/repos/rails/rails/issues/999/comments",
                201,
            )
            .body(r#"{"html_url":"https://github.com/rails/rails/pull/999#fixture"}"#),
        ])
        .await;
        let t = TestApp::boot_with_github_network(network)
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let crypto = ArEncryption::new(&t.booted.app.secrets);
        let (approval_id, agent_id, grant_id, credential_id) = t.db().write(move |tx| {
            let mut agent = Agent::for_user(tx.conn(), BENDER)?.unwrap();
            agent.update(tx, AgentChanges { owner_id: Some(Some(DAVID)), suspended_at: Some(None), daily_external_action_cap: Some(None), ..Default::default() })?;
            tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=? AND capability='external_action'", [agent.id])?;
            let grant = AgentGrant::create(tx, NewGrant { agent_id: agent.id, room_id: Some(ALL_TALK), capability: "external_action".into(), granted_by_id: DAVID, ..Default::default() })?;
            let (credential, _) = AgentCredential::create_with_secret(tx, agent.id, "Execution bridge", DAVID, None)?;
            let account = Account::relink(tx, &crypto, &AccountInput { user_id: BENDER, github_login: "machine", access_token: "fixture-agent-token", refresh_token: None, token_expires_at: None, token_source: "pat" })?;
            let now = tx.now();
            let pr: i64 = tx.conn().query_row("INSERT INTO github_pull_requests(owner,repo,number,created_at,updated_at) VALUES ('rails','rails',999,?,?) RETURNING id", params![now,now], |r| r.get(0))?;
            let thread = ChannelThread::create(tx, NewChannelThread { room_id: ALL_TALK, creator_id: DAVID, name: Some("Execution bridge".into()), ..Default::default() })?;
            tx.conn().execute("INSERT INTO github_pull_request_threads(github_pull_request_id,room_id,channel_thread_id,created_at,updated_at) VALUES (?,?,?,?,?)", params![pr,ALL_TALK,thread.id,now,now])?;
            let approval = AgentApproval::create(tx, NewApproval { agent_id: agent.id, agent_credential_id: Some(credential.id), room_id: Some(ALL_TALK), action: "github.comment".into(), summary: "Comment on rails/rails#999: Nice work".into(), payload: Some(json!({"pull_request_id":pr,"kind":"comment","body":"Nice work","reviewers":null}).to_string()), github_account_id: Some(account.id), github_login: Some(account.github_login), ..Default::default() })?;
            Ok((approval.id, agent.id, grant.id, credential.id))
        }).await.unwrap();
        let mut admin = t.david();
        let response = admin
            .write(
                Req::new(
                    Method::PATCH,
                    &format!(
                        "/agent_approvals/{approval_id}{}",
                        if html { "" } else { ".json" }
                    ),
                )
                .header(
                    "accept",
                    if html {
                        "text/html"
                    } else {
                        "application/json"
                    },
                )
                .form(&[("decision", "approved")]),
            )
            .await;
        assert_eq!(
            response.status,
            if html {
                StatusCode::SEE_OTHER
            } else {
                StatusCode::OK
            },
            "{case}: {}",
            response.text()
        );
        if html {
            assert_eq!(response.text(), "", "complete Rails HTML redirect body");
            assert_eq!(
                response.header("location"),
                Some("http://campfire.test/activity")
            );
        } else {
            assert_eq!(response.json()["status"], "approved");
        }
        // Queue assertions always run after without_job_runner() has joined workers.
        let queued_id = t.db().read(move |conn| {
            let mut statement = conn.prepare("SELECT arguments FROM background_jobs WHERE job_class='Github::PerformAgentActionJob' AND json_extract(arguments,'$.approval_id')=?")?;
            let rows: Vec<String> = statement.query_map([approval_id], |r|r.get(0))?.collect::<rusqlite::Result<_>>()?;
            assert_eq!(rows.len(), 1);
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND handled_at IS NULL", [approval_id], |r|r.get::<_,i64>(0))?, 0);
            Ok(serde_json::from_str::<Value>(&rows[0]).unwrap()["approval_id"].as_i64().unwrap())
        }).await.unwrap();
        let crypto = ArEncryption::new(&t.booted.app.secrets);
        t.db()
            .write(move |tx| {
                match case {
                    "identity_changed" => {
                        Account::relink(
                            tx,
                            &crypto,
                            &AccountInput {
                                user_id: BENDER,
                                github_login: "changed",
                                access_token: "fixture-agent-token",
                                refresh_token: None,
                                token_expires_at: None,
                                token_source: "pat",
                            },
                        )?;
                    }
                    "grant_revoked" => {
                        AgentGrant::find(tx.conn(), grant_id)?.unwrap().revoke(tx)?
                    }
                    "credential_revoked" => AgentCredential::find(tx.conn(), credential_id)?
                        .unwrap()
                        .revoke(tx)?,
                    "suspended" => {
                        let mut a = Agent::find(tx.conn(), agent_id)?.unwrap();
                        a.update(
                            tx,
                            AgentChanges {
                                suspended_at: Some(Some(tx.now())),
                                ..Default::default()
                            },
                        )?;
                    }
                    "deactivated" => User::find(tx.conn(), BENDER)?.deactivate(tx)?,
                    _ => (),
                }
                Ok(())
            })
            .await
            .unwrap();
        // This is the real handler registered for the persisted job, including HTTP and claims.
        for _ in 0..2 {
            agent_actions::perform(t.db(), &t.booted.app.github_accounts, queued_id)
                .await
                .unwrap();
        }
        t.db().read(move |conn| {
            let mut query = conn.prepare("SELECT outcome,metadata FROM agent_events WHERE event_type='github_action_completed' AND agent_approval_id=?")?;
            let events: Vec<(Option<String>,String)> = query.query_map([approval_id], |r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
            assert_eq!(events.len(),1,"{case}");
            let metadata: Value=serde_json::from_str(&events[0].1).unwrap();
            assert_eq!(metadata["status"],expected_status,"{case}");
            assert_eq!(metadata.get("message").and_then(Value::as_str),expected_message,"{case}");
            assert_eq!(events[0].0.as_deref(),Some("delivered"),"{case}");
            Ok(())
        }).await.unwrap();
        let requests = server.received();
        assert_eq!(requests.len(), calls, "{case}");
        if let Some(request) = requests.first() {
            assert_eq!(request.target, "/repos/rails/rails/issues/999/comments");
            assert_eq!(
                request.header("Authorization"),
                Some(format!("Bearer {}", "fixture-agent-token").as_str())
            );
            assert_eq!(
                serde_json::from_slice::<Value>(&request.body).unwrap(),
                json!({"body":"Nice work"})
            );
        }
    }
}

#[tokio::test]
async fn ws11ui_human_fizzy_decision_executes_real_transport_and_rechecks_later_changes() {
    use crate::integrations::fizzy::{
        accounts::{Account as Fizzy, Input},
        agent_action::Action,
        agent_job,
    };
    use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, network};
    use std::sync::Arc;
    for ((case, expected_status, expected_message, calls), html) in [
        ("success", "completed", None, 1),
        (
            "identity_changed",
            "failed",
            Some("The agent owner's Fizzy account changed since this was approved"),
            0,
        ),
        (
            "grant_revoked",
            "failed",
            Some("Agent no longer has the external_action capability"),
            0,
        ),
        ("credential_revoked", "completed", None, 1),
        (
            "suspended",
            "failed",
            Some("Agent is suspended or deactivated"),
            0,
        ),
        (
            "deactivated",
            "failed",
            Some("Agent is suspended or deactivated"),
            0,
        ),
    ]
    .into_iter()
    .flat_map(|case| [(case, false), (case, true)])
    {
        // Held listener from #173: no unreserved free-port probe or global env mutation.
        let server = FakeServer::start_ws15e(vec![
            Route::new(
                "POST",
                "app.fizzy.do",
                "/12345/cards/579/comments.json",
                201,
            )
            .body(r#"{"url":"https://app.fizzy.do/created"}"#),
        ])
        .await;
        let resolver = Arc::new(FakeResolver::new([("app.fizzy.do", vec!["93.184.216.34"])]));
        let dialer = Arc::new(MappingDialer {
            public: ["93.184.216.34".parse().unwrap()].into(),
            to: server.addr,
            dialed: Default::default(),
        });
        let net = network(resolver, dialer);
        let t = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let crypto = ArEncryption::new(&t.booted.app.secrets);
        let (approval_id, agent_id, grant_id, credential_id) = t
            .db()
            .write(move |tx| {
                let mut agent = Agent::for_user(tx.conn(), BENDER)?.unwrap();
                agent.update(
                    tx,
                    AgentChanges {
                        owner_id: Some(Some(DAVID)),
                        suspended_at: Some(None),
                        daily_external_action_cap: Some(None),
                        ..Default::default()
                    },
                )?;
                tx.conn().execute(
                    "DELETE FROM agent_grants WHERE agent_id=? AND capability='external_action'",
                    [agent.id],
                )?;
                let grant = AgentGrant::create(
                    tx,
                    NewGrant {
                        agent_id: agent.id,
                        room_id: None,
                        capability: "external_action".into(),
                        granted_by_id: DAVID,
                        ..Default::default()
                    },
                )?;
                let (credential, _) = AgentCredential::create_with_secret(
                    tx,
                    agent.id,
                    "Execution bridge",
                    DAVID,
                    None,
                )?;
                let account = Fizzy::relink(
                    tx,
                    &crypto,
                    &Input {
                        user_id: DAVID,
                        account_id: "12345",
                        account_name: Some("Fixture"),
                        fizzy_user_id: Some("fixture-user"),
                        fizzy_user_name: Some("Fixture User"),
                        token: "fixture-owner-token",
                    },
                )?;
                let action = Action::from_payload(
                    json!({"account_id":"12345","kind":"comment","number":579,"body":"Nice work"}),
                );
                let approval = AgentApproval::create(
                    tx,
                    NewApproval {
                        agent_id: agent.id,
                        agent_credential_id: Some(credential.id),
                        action: action.action_name(),
                        summary: action.summary().unwrap(),
                        payload: Some(action.payload_json()),
                        fizzy_connected_account_id: Some(account.id),
                        fizzy_user_id: Some("fixture-user".into()),
                        ..Default::default()
                    },
                )?;
                Ok((approval.id, agent.id, grant.id, credential.id))
            })
            .await
            .unwrap();
        let mut admin = t.david();
        let response = admin
            .write(
                Req::new(
                    Method::PATCH,
                    &format!(
                        "/agent_approvals/{approval_id}{}",
                        if html { "" } else { ".json" }
                    ),
                )
                .header(
                    "accept",
                    if html {
                        "text/html"
                    } else {
                        "application/json"
                    },
                )
                .form(&[("decision", "approved")]),
            )
            .await;
        assert_eq!(
            response.status,
            if html {
                StatusCode::SEE_OTHER
            } else {
                StatusCode::OK
            },
            "{case}: {}",
            response.text()
        );
        if html {
            assert_eq!(response.text(), "", "complete Rails HTML redirect body");
            assert_eq!(
                response.header("location"),
                Some("http://campfire.test/activity")
            );
        } else {
            assert_eq!(response.json()["status"], "approved");
        }
        let queued_id=t.db().read(move |conn| {
            let mut query=conn.prepare("SELECT arguments FROM background_jobs WHERE job_class='Fizzy::PerformAgentActionJob' AND json_extract(arguments,'$.approval_id')=?")?;
            let rows: Vec<String>=query.query_map([approval_id],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?;
            assert_eq!(rows.len(),1);
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND handled_at IS NULL",[approval_id],|r|r.get::<_,i64>(0))?,0);
            Ok(serde_json::from_str::<Value>(&rows[0]).unwrap()["approval_id"].as_i64().unwrap())
        }).await.unwrap();
        let crypto = ArEncryption::new(&t.booted.app.secrets);
        t.db()
            .write(move |tx| {
                match case {
                    "identity_changed" => {
                        Fizzy::relink(
                            tx,
                            &crypto,
                            &Input {
                                user_id: DAVID,
                                account_id: "12345",
                                account_name: Some("Fixture"),
                                fizzy_user_id: Some("changed"),
                                fizzy_user_name: Some("Changed"),
                                token: "fixture-owner-token",
                            },
                        )?;
                    }
                    "grant_revoked" => {
                        AgentGrant::find(tx.conn(), grant_id)?.unwrap().revoke(tx)?
                    }
                    "credential_revoked" => AgentCredential::find(tx.conn(), credential_id)?
                        .unwrap()
                        .revoke(tx)?,
                    "suspended" => {
                        let mut agent = Agent::find(tx.conn(), agent_id)?.unwrap();
                        agent.update(
                            tx,
                            AgentChanges {
                                suspended_at: Some(Some(tx.now())),
                                ..Default::default()
                            },
                        )?;
                    }
                    "deactivated" => User::find(tx.conn(), BENDER)?.deactivate(tx)?,
                    _ => (),
                }
                Ok(())
            })
            .await
            .unwrap();
        for _ in 0..2 {
            agent_job::execute(&t.booted.app, &net, "http://app.fizzy.do", queued_id)
                .await
                .unwrap();
        }
        t.db().read(move |conn| {
            let mut query=conn.prepare("SELECT outcome,metadata FROM agent_events WHERE event_type='fizzy_action_completed' AND agent_approval_id=?")?;
            let events: Vec<(Option<String>,String)>=query.query_map([approval_id],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
            assert_eq!(events.len(),1,"{case}");
            let metadata: Value=serde_json::from_str(&events[0].1).unwrap();
            assert_eq!(metadata["status"],expected_status,"{case}");
            assert_eq!(metadata.get("message").and_then(Value::as_str),expected_message,"{case}");
            assert_eq!(events[0].0.as_deref(),Some("delivered"),"{case}");
            Ok(())
        }).await.unwrap();
        let requests = server.received();
        assert_eq!(requests.len(), calls, "{case}");
        if let Some(request) = requests.first() {
            assert_eq!(request.target, "/12345/cards/579/comments.json");
            assert_eq!(
                request.header("Authorization"),
                Some(format!("Bearer {}", "fixture-owner-token").as_str())
            );
            assert_eq!(
                serde_json::from_slice::<Value>(&request.body).unwrap(),
                json!({"comment":{"body":"Nice work"}})
            );
        }
    }
}
