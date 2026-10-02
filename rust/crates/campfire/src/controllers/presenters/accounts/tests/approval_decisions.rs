//! Human decision requests exercise WS11's atomic approval/inbox/ledger callbacks.
use super::*;
use campfire_db::{AgentApproval, NewApproval};

async fn approval(test: &Test, action: &str) -> i64 {
    let owner_id: i64 = test.label("users.kevin").parse().unwrap();
    let bot_id: i64 = test.label("users.bender").parse().unwrap();
    let action = action.to_owned();
    test.booted
        .app
        .db
        .write(move |tx| {
            let mut agent = campfire_db::Agent::for_user(tx.conn(), bot_id)?.unwrap();
            agent.update(
                tx,
                campfire_db::AgentChanges {
                    owner_id: Some(Some(owner_id)),
                    ..Default::default()
                },
            )?;
            Ok(AgentApproval::create(
                tx,
                NewApproval {
                    agent_id: agent.id,
                    action,
                    summary: "A human must decide".into(),
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap()
}
fn payload(reply: &Reply) -> serde_json::Value {
    serde_json::from_slice(&reply.body).expect("decision JSON")
}
async fn state(test: &Test, id: i64) -> AgentApproval {
    test.booted
        .app
        .db
        .read(move |conn| Ok(AgentApproval::find(conn, id)?.unwrap()))
        .await
        .unwrap()
}
#[tokio::test]
async fn owner_approves_generic_action_with_atomic_inbox_ledger_then_audit() {
    let test = boot_seed("default").await.expect("default seed");
    let id = approval(&test, "deploy").await;
    let mut owner = test.browser("198.51.100.201");
    owner.sign_in(&test.label("emails.kevin")).await;
    let response = owner
        .form(
            "patch",
            &format!("/agent_approvals/{id}.json"),
            &[("decision", "approved")],
        )
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let result = payload(&response);
    assert_eq!(result["status"], "approved");
    assert_eq!(result["decided_by"], "Kevin");
    assert!(result["decided_at"].as_str().unwrap().ends_with('Z'));
    assert!(result.get("note").is_none());
    test.booted.app.db.read(move |conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND handled_at IS NULL", [id], |r|r.get::<_,i64>(0))?, 0);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM agent_events WHERE agent_approval_id=? AND event_type='approval_decided'", [id], |r|r.get::<_,i64>(0))?, 1);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.approval.decide' AND target_id=?", [id], |r|r.get::<_,i64>(0))?, 1);
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn owner_cannot_approve_github_or_fizzy_but_can_deny_each() {
    let test = boot_seed("default").await.expect("default seed");
    let mut owner = test.browser("198.51.100.202");
    owner.sign_in(&test.label("emails.kevin")).await;
    for (action, service) in [("github.comment", "GitHub"), ("fizzy.comment", "Fizzy")] {
        let id = approval(&test, action).await;
        let path = format!("/agent_approvals/{id}.json");
        let response = owner
            .form("patch", &path, &[("decision", "approved")])
            .await;
        assert_eq!(
            response.status,
            StatusCode::FORBIDDEN,
            "{}",
            response.text()
        );
        assert_eq!(
            payload(&response)["error"],
            format!("Only an administrator can approve {service} write actions")
        );
        assert_eq!(state(&test, id).await.status, "pending");
        let response = owner
            .form(
                "patch",
                &path,
                &[
                    ("decision", "denied"),
                    ("decision_note", "Please revise <this>"),
                ],
            )
            .await;
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(payload(&response)["decision_note"], "Please revise <this>");
        assert_eq!(
            payload(&response)["note"],
            payload(&response)["decision_note"]
        );
    }
}
#[tokio::test]
async fn human_decisions_hide_requests_from_nondeciders_credentials_and_bot_keys() {
    let test = boot_seed("default").await.expect("default seed");
    let id = approval(&test, "deploy").await;
    let mut other = test.browser("198.51.100.203");
    other.sign_in(&test.label("emails.jz")).await;
    let path = format!("/agent_approvals/{id}.json");
    assert_eq!(
        other
            .form("patch", &path, &[("decision", "approved")])
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let mut token = test.browser("198.51.100.204");
    assert_eq!(
        token
            .request(
                Method::PATCH,
                &path,
                &[("authorization", &format!("Bearer {}", "bender-test-secret-1234"))],
                Some(("application/json", "{\"decision\":\"approved\"}".into()))
            )
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let path = format!("{path}?bot_key={}", encode(&test.label("bot_keys.bender")));
    assert_eq!(
        token
            .request(
                Method::PATCH,
                &path,
                &[],
                Some(("application/json", "{\"decision\":\"approved\"}".into()))
            )
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(state(&test, id).await.status, "pending");
}
#[tokio::test]
async fn decisions_validate_choice_and_note_before_writing() {
    let test = boot_seed("default").await.expect("default seed");
    let id = approval(&test, "deploy").await;
    let mut admin = test.browser("198.51.100.205");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/agent_approvals/{id}.json");
    let response = admin
        .form("patch", &path, &[("decision", "cancelled")])
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        payload(&response)["error"],
        "Decision must be approved or denied"
    );
    let note = "💬".repeat(201);
    let response = admin
        .form(
            "patch",
            &path,
            &[("decision", "denied"), ("decision_note", &note)],
        )
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        payload(&response)["error"],
        "Decision note is too long (maximum is 200 characters)"
    );
    assert_eq!(state(&test, id).await.status, "pending");
}
#[tokio::test]
async fn overdue_decision_returns_422_and_commits_lazy_expiry_without_audit() {
    let test = boot_seed("default").await.expect("default seed");
    let id = approval(&test, "deploy").await;
    test.booted
        .app
        .db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agent_approvals SET expires_at=? WHERE id=?",
                rusqlite::params![tx.now(), id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut admin = test.browser("198.51.100.206");
    admin.sign_in(&test.label("emails.david")).await;
    let response = admin
        .form(
            "patch",
            &format!("/agent_approvals/{id}.json"),
            &[("decision", "approved")],
        )
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(payload(&response)["error"], "Request has expired");
    assert_eq!(state(&test, id).await.status, "expired");
    test.booted.app.db.read(move |conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.approval.decide' AND target_id=?", [id], |r|r.get::<_,i64>(0))?, 0); Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn deciding_twice_returns_422_and_never_appends_another_event() {
    let test = boot_seed("default").await.expect("default seed");
    let id = approval(&test, "deploy").await;
    let mut admin = test.browser("198.51.100.207");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/agent_approvals/{id}.json");
    assert_eq!(
        admin
            .form(
                "patch",
                &path,
                &[
                    ("decision", "denied"),
                    ("decision_note", ""),
                    ("note", "Fallback")
                ]
            )
            .await
            .status,
        StatusCode::OK
    );
    let response = admin
        .form("patch", &path, &[("decision", "approved")])
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(payload(&response)["error"], "Request is already denied");
    assert_eq!(
        state(&test, id).await.decision_note.as_deref(),
        Some("Fallback")
    );
    test.booted
        .app
        .db
        .read(move |conn| {
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM agent_events WHERE agent_approval_id=?",
                    [id],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn html_decisions_use_303_and_flash_with_same_host_referer_policy() {
    let test = boot_seed("default").await.expect("default seed");
    let id = approval(&test, "deploy").await;
    let mut admin = test.browser("198.51.100.208");
    admin.sign_in(&test.label("emails.david")).await;
    let response = admin
        .form(
            "patch",
            &format!("/agent_approvals/{id}"),
            &[("decision", "denied")],
        )
        .await;
    assert_eq!(response.status, StatusCode::SEE_OTHER);
    assert_eq!(response.location(), "http://campfire.test/activity");
    use campfire_kit::Crypto;
    let key = campfire_kit::session::SESSION_KEY;
    let raw = percent_encoding::percent_decode_str(admin.cookies.get(key).unwrap())
        .decode_utf8()
        .unwrap();
    let data = campfire_kit::RailsCrypto::new(test.booted.app.secrets.clone())
        .decrypt_cookie(key, &raw, test.booted.app.clock.now())
        .unwrap();
    assert_eq!(data["flash"]["flashes"]["notice"], "Request denied.");
}
#[tokio::test]
async fn audit_failure_preserves_rails_committed_decision_inbox_and_ledger() {
    let test = boot_seed("default").await.expect("default seed");
    let id = approval(&test, "deploy").await;
    test.booted.app.db.write(|tx| { tx.conn().execute_batch("CREATE TRIGGER reject_approval_audit BEFORE INSERT ON audit_logs WHEN NEW.action='agent.approval.decide' BEGIN SELECT RAISE(ABORT, 'reject test audit'); END;")?; Ok(()) }).await.unwrap();
    let mut admin = test.browser("198.51.100.209");
    admin.sign_in(&test.label("emails.david")).await;
    assert_eq!(
        admin
            .form(
                "patch",
                &format!("/agent_approvals/{id}.json"),
                &[("decision", "approved")]
            )
            .await
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(state(&test, id).await.status, "approved");
    test.booted.app.db.read(move |conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM agent_events WHERE agent_approval_id=?",[id],|r|r.get::<_,i64>(0))?, 1);
        assert!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND handled_at IS NOT NULL",[id],|r|r.get::<_,i64>(0))? > 0);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.approval.decide' AND target_id=?",[id],|r|r.get::<_,i64>(0))?, 0); Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn concurrent_deciders_produce_one_decision_event_and_audit() {
    let test = boot_seed("default").await.expect("default seed");
    let id = approval(&test, "deploy").await;
    let mut admin = test.browser("198.51.100.210");
    admin.sign_in(&test.label("emails.david")).await;
    let mut owner = test.browser("198.51.100.211");
    owner.sign_in(&test.label("emails.kevin")).await;
    let path = format!("/agent_approvals/{id}.json");
    let (approved, denied) = tokio::join!(
        admin.form("patch", &path, &[("decision", "approved")]),
        owner.form("patch", &path, &[("decision", "denied")])
    );
    let mut statuses = [approved.status.as_u16(), denied.status.as_u16()];
    statuses.sort();
    assert_eq!(statuses, [200, 422]);
    test.booted.app.db.read(move |conn|{
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM agent_events WHERE agent_approval_id=?",[id],|r|r.get::<_,i64>(0))?,1);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.approval.decide' AND target_id=?",[id],|r|r.get::<_,i64>(0))?,1);Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn deactivated_bot_requests_are_hidden_and_unchanged() {
    let test = boot_seed("default").await.expect("default seed");
    let id = approval(&test, "deploy").await;
    let bot: i64 = test.label("users.bender").parse().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| campfire_db::User::find(tx.conn(), bot)?.deactivate(tx))
        .await
        .unwrap();
    let mut admin = test.browser("198.51.100.212");
    admin.sign_in(&test.label("emails.david")).await;
    assert_eq!(
        admin
            .form(
                "patch",
                &format!("/agent_approvals/{id}.json"),
                &[("decision", "approved")]
            )
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(state(&test, id).await.status, "pending");
}
#[tokio::test]
async fn html_external_approve_is_303_for_owner_with_alert_and_safe_referer() {
    let test = boot_seed("default").await.expect("default seed");
    let id = approval(&test, "github.comment").await;
    let mut owner = test.browser("198.51.100.213");
    owner.sign_in(&test.label("emails.kevin")).await;
    let response = owner
        .request(
            Method::PATCH,
            &format!("/agent_approvals/{id}"),
            &[("referer", "http://campfire.test/agents/773018776/approvals")],
            Some((
                "application/x-www-form-urlencoded",
                "decision=approved".into(),
            )),
        )
        .await;
    assert_eq!(response.status, StatusCode::SEE_OTHER);
    assert_eq!(
        response.location(),
        "http://campfire.test/agents/773018776/approvals"
    );
    let response = owner
        .request(
            Method::PATCH,
            &format!("/agent_approvals/{id}"),
            &[("referer", "https://attacker.test/")],
            Some((
                "application/x-www-form-urlencoded",
                "decision=approved".into(),
            )),
        )
        .await;
    assert_eq!(response.status, StatusCode::SEE_OTHER);
    assert_eq!(response.location(), "http://campfire.test/activity");
    assert_eq!(state(&test, id).await.status, "pending");
}

#[tokio::test]
async fn external_decisions_revalidate_identity_and_atomically_enqueue_owner_jobs() {
    use crate::controllers::presenters::test_support::{BENDER, KEVIN, Req, TestApp};
    use crate::integrations::{
        fizzy::accounts::{Account as Fizzy, Input},
        github::accounts::{Account as Github, AccountInput},
    };
    let test = TestApp::boot_frozen()
        .await
        .expect("default seed")
        .without_job_runner()
        .await;
    let bot_id = BENDER;
    let owner_id = KEVIN;
    let crypto = test.booted.app.ar_encryption.clone();
    let (github_id, fizzy_id) = test
        .booted
        .app
        .db
        .write(move |tx| {
            let g = Github::relink(
                tx,
                &crypto,
                &AccountInput {
                    user_id: bot_id,
                    github_login: "machine-fixture",
                    access_token: "fixture-machine-credential",
                    refresh_token: None,
                    token_expires_at: None,
                    token_source: "pat",
                },
            )?;
            let f = Fizzy::relink(
                tx,
                &crypto,
                &Input {
                    user_id: owner_id,
                    account_id: "12345",
                    account_name: Some("Fixture"),
                    fizzy_user_id: Some("fixture-user"),
                    fizzy_user_name: Some("Fixture User"),
                    token: "fixture-fizzy-credential",
                },
            )?;
            Ok((g.id, f.id))
        })
        .await
        .unwrap();
    let mut admin = test.david();
    for (service, job) in [
        ("github", "Github::PerformAgentActionJob"),
        ("fizzy", "Fizzy::PerformAgentActionJob"),
    ] {
        let action = format!("{service}.comment");
        let id = test
            .db()
            .write(move |tx| {
                let mut agent = campfire_db::Agent::for_user(tx.conn(), bot_id)?.unwrap();
                agent.update(
                    tx,
                    campfire_db::AgentChanges {
                        owner_id: Some(Some(owner_id)),
                        ..Default::default()
                    },
                )?;
                Ok(AgentApproval::create(
                    tx,
                    NewApproval {
                        agent_id: agent.id,
                        action,
                        summary: "A human must decide".into(),
                        ..Default::default()
                    },
                )?
                .id)
            })
            .await
            .unwrap();
        let missing = admin
            .write(
                Req::new(Method::PATCH, &format!("/agent_approvals/{id}.json"))
                    .form(&[("decision", "approved")]),
            )
            .await;
        assert_eq!(
            missing.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            missing.text()
        );
        assert_eq!(
            test.db()
                .read(move |conn| Ok(AgentApproval::find(conn, id)?.unwrap().status))
                .await
                .unwrap(),
            "pending"
        );
        let github = service == "github";
        test.booted.app.db.write(move|tx| {
            tx.conn().execute("UPDATE agent_approvals SET github_account_id=?,github_login=?,fizzy_connected_account_id=?,fizzy_user_id=? WHERE id=?",rusqlite::params![github.then_some(github_id),github.then_some("machine-fixture"),(!github).then_some(fizzy_id),(!github).then_some("fixture-user"),id])?;Ok(())
        }).await.unwrap();
        test.booted.app.db.write(move|tx| {
            tx.conn().execute_batch("CREATE TRIGGER ws11ui_reject_external BEFORE INSERT ON background_jobs WHEN NEW.job_class IN ('Github::PerformAgentActionJob','Fizzy::PerformAgentActionJob') BEGIN SELECT RAISE(ABORT,'external enqueue rejected'); END;")?;Ok(())
        }).await.unwrap();
        let rejected = admin
            .write(
                Req::new(Method::PATCH, &format!("/agent_approvals/{id}.json"))
                    .form(&[("decision", "approved")]),
            )
            .await;
        assert_eq!(rejected.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            test.db()
                .read(move |conn| Ok(AgentApproval::find(conn, id)?.unwrap().status))
                .await
                .unwrap(),
            "pending"
        );
        test.booted.app.db.read(move|conn| {
            assert_eq!(conn.query_row("SELECT count(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND handled_at IS NOT NULL",[id],|r|r.get::<_,i64>(0))?,0);
            assert_eq!(conn.query_row("SELECT count(*) FROM agent_events WHERE event_type='approval_decided' AND json_extract(metadata,'$.approval_id')=?",[id],|r|r.get::<_,i64>(0))?,0);Ok(())
        }).await.unwrap();
        test.booted
            .app
            .db
            .write(|tx| {
                tx.conn()
                    .execute_batch("DROP TRIGGER ws11ui_reject_external")?;
                Ok(())
            })
            .await
            .unwrap();
        let accepted = admin
            .write(
                Req::new(Method::PATCH, &format!("/agent_approvals/{id}.json"))
                    .form(&[("decision", "approved")]),
            )
            .await;
        assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.text());
        assert_eq!(accepted.json()["status"], "approved");
        let job = job.to_owned();
        test.booted.app.db.read(move|conn| {
            assert_eq!(conn.query_row("SELECT count(*) FROM background_jobs WHERE job_class=? AND json_extract(arguments,'$.approval_id')=?",rusqlite::params![job,id],|r|r.get::<_,i64>(0))?,1);Ok(())
        }).await.unwrap();
    }
}
