//! Rails kill-switch callers, using WS11's newly supplied lifecycle operation.
use super::*;
use campfire_db::{Agent, AgentApproval, AgentGrant, NewApproval, NewGrant};
#[tokio::test]
async fn admin_kill_switch_cancels_pending_expires_overdue_revokes_and_clears_presence() {
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let (id,pending,overdue)=test.booted.app.db.write(move |tx|{
        let agent=Agent::for_user(tx.conn(),bot)?.unwrap();
        let pending=AgentApproval::create(tx,NewApproval{agent_id:agent.id,action:"deploy".into(),summary:"Pending work".into(),..Default::default()})?;
        let overdue=AgentApproval::create(tx,NewApproval{agent_id:agent.id,action:"deploy".into(),summary:"Overdue work".into(),..Default::default()})?;
        tx.conn().execute("UPDATE agent_approvals SET expires_at=? WHERE id=?",rusqlite::params![tx.now(),overdue.id])?;
        tx.conn().execute("UPDATE agents SET working_presence='Thinking',working_presence_expires_at=? WHERE id=?",rusqlite::params![tx.now().since(jiff::SignedDuration::from_mins(5)),agent.id])?;
        AgentGrant::create(tx,NewGrant{agent_id:agent.id,capability:"post_messages".into(),granted_by_id:127326141,..Default::default()})?;
        Ok((agent.id,pending.id,overdue.id))
    }).await.unwrap();
    let mut admin = test.browser("198.51.100.151");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/account/bots/{bot}/kill_switch");
    assert_redirect(
        &admin.form("post", &path, &[]).await,
        &format!("http://campfire.test/account/bots/{bot}/edit"),
    );
    assert!(
        admin
            .get(&format!("/account/bots/{bot}/edit"))
            .await
            .text()
            .contains("Agent suspended; 2 approvals cancelled.")
    );
    test.booted.app.db.read(move |conn|{
        let agent=Agent::find(conn,id)?.unwrap();assert!(agent.suspended());assert!(agent.working_presence.is_none());
        assert_eq!(AgentApproval::find(conn,pending)?.unwrap().status,"cancelled");assert_eq!(AgentApproval::find(conn,overdue)?.unwrap().status,"expired");
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM agent_grants WHERE agent_id=? AND revoked_at IS NULL",[id],|r|r.get::<_,i64>(0))?,0);
        assert_eq!(conn.query_row("SELECT actor_id FROM audit_logs WHERE action='agent.kill_switch' AND target_id=?",[id],|r|r.get::<_,i64>(0))?,127326141);Ok(())
    }).await.unwrap();
    assert_redirect(
        &admin.form("post", &path, &[]).await,
        &format!("http://campfire.test/account/bots/{bot}/edit"),
    );
    assert!(
        admin
            .get(&format!("/account/bots/{bot}/edit"))
            .await
            .text()
            .contains("Agent suspended; 0 approvals cancelled.")
    );
}
#[tokio::test]
async fn owner_kill_switch_does_not_require_sudo() {
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| {
            Agent::for_user(tx.conn(), bot)?.unwrap().update(
                tx,
                campfire_db::AgentChanges {
                    owner_id: Some(Some(712064548)),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let mut owner = test.browser("198.51.100.152");
    owner.sign_in(&test.label("emails.kevin")).await;
    assert_redirect(
        &owner
            .form("post", &format!("/account/bots/{bot}/kill_switch"), &[])
            .await,
        &format!("http://campfire.test/account/bots/{bot}/edit"),
    );
    assert!(
        test.booted
            .app
            .db
            .read(move |conn| Ok(Agent::for_user(conn, bot)?.unwrap().suspended()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn kill_switch_denies_nonowner_and_does_not_convert_legacy_bot() {
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let mut member = test.browser("198.51.100.153");
    member.sign_in(&test.label("emails.kevin")).await;
    assert_eq!(
        member
            .form("post", &format!("/account/bots/{bot}/kill_switch"), &[])
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let legacy = test
        .booted
        .app
        .db
        .write(|tx| Ok(campfire_db::User::create_bot(tx, "Legacy kill", None)?.id))
        .await
        .unwrap();
    let mut admin = test.browser("198.51.100.154");
    admin.sign_in(&test.label("emails.david")).await;
    assert_eq!(
        admin
            .form("post", &format!("/account/bots/{legacy}/kill_switch"), &[])
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert!(
        test.booted
            .app
            .db
            .read(move |conn| Agent::for_user(conn, legacy))
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn kill_switch_audit_failure_rolls_back_suspension_and_cancellation() {
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let approval=test.booted.app.db.write(move |tx|{
        let agent=Agent::for_user(tx.conn(),bot)?.unwrap();
        tx.conn().execute_batch("CREATE TRIGGER reject_kill_audit BEFORE INSERT ON audit_logs WHEN NEW.action='agent.kill_switch' BEGIN SELECT RAISE(ABORT,'test audit rejection'); END;")?;
        Ok(AgentApproval::create(tx,NewApproval{agent_id:agent.id,action:"deploy".into(),summary:"Do not cancel".into(),..Default::default()})?.id)
    }).await.unwrap();
    let mut admin = test.browser("198.51.100.155");
    admin.sign_in(&test.label("emails.david")).await;
    assert_eq!(
        admin
            .form("post", &format!("/account/bots/{bot}/kill_switch"), &[])
            .await
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    test.booted
        .app
        .db
        .read(move |conn| {
            assert!(!Agent::for_user(conn, bot)?.unwrap().suspended());
            assert_eq!(
                AgentApproval::find(conn, approval)?.unwrap().status,
                "pending"
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn removal_passes_request_actor_to_owned_agent_suspension() {
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let child = test
        .booted
        .app
        .db
        .write(move |tx| {
            let user = campfire_db::User::create_bot(tx, "Owned by bot", None)?;
            Ok(Agent::create(
                tx,
                campfire_db::NewAgent {
                    user_id: user.id,
                    owner_id: Some(bot),
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    let mut admin = test.browser("198.51.100.156");
    admin.sign_in(&test.label("emails.david")).await;
    assert_redirect(
        &admin
            .form("delete", &format!("/account/bots/{bot}"), &[])
            .await,
        "http://campfire.test/account/bots",
    );
    test.booted
        .app
        .db
        .read(move |conn| {
            assert!(Agent::find(conn, child)?.unwrap().suspended());
            assert_eq!(
                conn.query_row(
                    "SELECT actor_id FROM audit_logs WHERE action='agent.suspend' AND target_id=?",
                    [child],
                    |r| r.get::<_, Option<i64>>(0)
                )?,
                Some(127326141)
            );
            Ok(())
        })
        .await
        .unwrap();
}
