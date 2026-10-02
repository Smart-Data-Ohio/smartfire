//! Mirrors the pinned HTTP probe views/agents_ui/access_boundaries.rb.
use crate::controllers::presenters::test_support::{BENDER, HQ, Req, TestApp};
use axum::http::{Method, StatusCode};

async fn admin(t: &TestApp) -> crate::controllers::presenters::test_support::Browser<'_> {
    let mut browser = t.david();
    let response = browser
        .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
        .await;
    assert_eq!(response.status, StatusCode::FOUND);
    browser
}
async fn reject_audit(t: &TestApp) {
    t.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_access_audit BEFORE INSERT ON audit_logs WHEN NEW.action IN ('agent.credential.create','agent.credential.revoke','agent.grant.create','agent.grant.revoke') BEGIN SELECT RAISE(ABORT,'fixture access audit rejection'); END;")?;Ok(())}).await.unwrap();
}
#[tokio::test]
async fn ws11ui_access_credential_commit_precedes_audit_and_rerevoke_is_noop() {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = admin(&t).await;
    reject_audit(&t).await;
    let response = browser
        .write(
            Req::new(Method::POST, &format!("/account/bots/{BENDER}/credentials"))
                .form(&[("agent_credential[name]", "Fault credential")]),
        )
        .await;
    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    let id = t
        .db()
        .read(|conn| {
            let id: i64 = conn.query_row(
                "SELECT id FROM agent_credentials WHERE name='Fault credential'",
                [],
                |r| r.get(0),
            )?;
            let credential = campfire_db::AgentCredential::find(conn, id)?.unwrap();
            assert!(!credential.token_digest.is_empty());
            assert_eq!(credential.token_last_four.len(), 4);
            assert!(credential.revoked_at.is_none());
            Ok(id)
        })
        .await
        .unwrap();
    let path = format!("/account/bots/{BENDER}/credentials/{id}");
    assert_eq!(
        browser.write(Req::new(Method::DELETE, &path)).await.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let before = t
        .db()
        .read(move |conn| {
            let c = campfire_db::AgentCredential::find(conn, id)?.unwrap();
            assert!(c.revoked_at.is_some());
            Ok(c.updated_at)
        })
        .await
        .unwrap();
    let response = browser.write(Req::new(Method::DELETE, &path)).await;
    assert_eq!(response.status, StatusCode::FOUND);
    t.db().read(move|conn|{
        assert_eq!(campfire_db::AgentCredential::find(conn,id)?.unwrap().updated_at,before);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action IN ('agent.credential.create','agent.credential.revoke')",[],|r|r.get::<_,i64>(0))?,0);
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn ws11ui_access_grant_commit_precedes_audit_and_rerevoke_is_noop() {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = admin(&t).await;
    reject_audit(&t).await;
    t.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM agent_grants WHERE capability='external_action' AND room_id=?",
                [HQ],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let response = browser
        .write(
            Req::new(Method::POST, &format!("/account/bots/{BENDER}/grants")).form(&[
                ("agent_grant[capability]", "external_action"),
                ("agent_grant[room_id]", &HQ.to_string()),
            ]),
        )
        .await;
    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    let id=t.db().read(|conn|Ok(conn.query_row("SELECT id FROM agent_grants WHERE capability='external_action' AND room_id=? AND revoked_at IS NULL",[HQ],|r|r.get::<_,i64>(0))?)).await.unwrap();
    let path = format!("/account/bots/{BENDER}/grants/{id}");
    assert_eq!(
        browser.write(Req::new(Method::DELETE, &path)).await.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let before = t
        .db()
        .read(move |conn| {
            let g = campfire_db::AgentGrant::find(conn, id)?.unwrap();
            assert!(g.revoked_at.is_some());
            Ok(g.updated_at)
        })
        .await
        .unwrap();
    assert_eq!(
        browser.write(Req::new(Method::DELETE, &path)).await.status,
        StatusCode::FOUND
    );
    t.db().read(move|conn|{
        assert_eq!(campfire_db::AgentGrant::find(conn,id)?.unwrap().updated_at,before);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action IN ('agent.grant.create','agent.grant.revoke')",[],|r|r.get::<_,i64>(0))?,0);
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn ws11ui_access_grant_unique_rescue_rolls_back_its_transaction() {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = admin(&t).await;
    t.db().write(|tx|{
        tx.conn().execute("DELETE FROM agent_grants WHERE capability='external_action' AND room_id=?",[HQ])?;
        tx.conn().execute_batch("CREATE TRIGGER collide_access_grant BEFORE INSERT ON agent_grants WHEN NEW.capability='external_action' AND NEW.room_id=201306877 BEGIN INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,created_at,updated_at) VALUES(NEW.agent_id,NEW.capability,NEW.room_id,NEW.granted_by_id,NEW.created_at,NEW.updated_at); END;")?;
        Ok(())
    }).await.unwrap();
    let response = browser
        .write(
            Req::new(Method::POST, &format!("/account/bots/{BENDER}/grants")).form(&[
                ("agent_grant[capability]", "external_action"),
                ("agent_grant[room_id]", &HQ.to_string()),
            ]),
        )
        .await;
    assert_eq!(response.status, StatusCode::FOUND);
    t.db().read(|conn|{assert_eq!(conn.query_row("SELECT COUNT(*) FROM agent_grants WHERE capability='external_action' AND room_id=? AND revoked_at IS NULL",[HQ],|r|r.get::<_,i64>(0))?,0);Ok(())}).await.unwrap();
}
