//! Human history pagination, lazy expiry and content policy through the real HTTP stack.
use super::*;
use campfire_db::{Agent, AgentApproval, NewApproval};

#[tokio::test]
async fn disabling_approval_inbox_preferences_preserves_history_and_decision_access() {
    let test = boot_seed("default").await.expect("default seed");
    let id = configure(&test).await;
    let owner_id: i64 = test.label("users.kevin").parse().unwrap();
    let approval_id = test
        .booted
        .app
        .db
        .write(move |tx| {
            // WS8 owns the preference editor; configure its persisted boolean shape.
            tx.conn().execute(
                "UPDATE users SET inbox_preferences=? WHERE id=?",
                rusqlite::params![serde_json::json!({"agent_approvals":false}), owner_id],
            )?;
            Ok(AgentApproval::create(
                tx,
                NewApproval {
                    agent_id: id,
                    action: "deploy".into(),
                    summary: "Available without inbox delivery".into(),
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    test.booted
        .app
        .db
        .read(move |conn| {
            assert!(
                campfire_db::ActivityItem::find_by_user_and_source(
                    conn,
                    owner_id,
                    "AgentApproval",
                    approval_id
                )?
                .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
    let mut owner = test.browser("198.51.100.160");
    owner.sign_in(&test.label("emails.kevin")).await;
    let response = owner.get(&format!("/api/v1/agents/{id}/approvals")).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    assert!(response.text().contains("Available without inbox delivery"));
    let response = owner
        .form(
            "patch",
            &format!("/agent_approvals/{approval_id}.json"),
            &[("decision", "approved")],
        )
        .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&response.body).unwrap()["status"],
        "approved"
    );
}
async fn configure(test: &Test) -> i64 {
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let owner: i64 = test.label("users.kevin").parse().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| {
            let mut agent = Agent::for_user(tx.conn(), bot)?.unwrap();
            agent.update(
                tx,
                campfire_db::AgentChanges {
                    owner_id: Some(Some(owner)),
                    ..Default::default()
                },
            )?;
            Ok(agent.id)
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn history_expires_due_requests_and_filters_effective_status() {
    let test = boot_seed("default").await.expect("default seed");
    let id = configure(&test).await;
    let expired = test
        .booted
        .app
        .db
        .write(move |tx| {
            let a = AgentApproval::create(
                tx,
                NewApproval {
                    agent_id: id,
                    action: "deploy".into(),
                    summary: "Overdue request".into(),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE agent_approvals SET expires_at=? WHERE id=?",
                rusqlite::params![tx.now(), a.id],
            )?;
            AgentApproval::create(
                tx,
                NewApproval {
                    agent_id: id,
                    action: "deploy".into(),
                    summary: "Future request".into(),
                    ..Default::default()
                },
            )?;
            Ok(a.id)
        })
        .await
        .unwrap();
    let mut owner = test.browser("198.51.100.184");
    owner.sign_in(&test.label("emails.kevin")).await;
    let reply = owner
        .get(&format!("/api/v1/agents/{id}/approvals?status=pending"))
        .await;
    assert!(reply.text().contains("Future request"));
    assert!(!reply.text().contains("Overdue request"));
    let reply = owner
        .get(&format!("/api/v1/agents/{id}/approvals?status=expired"))
        .await;
    assert!(reply.text().contains("Overdue request"));
    assert!(!reply.text().contains("Future request"));
    test.booted.app.db.read(move |conn| {
        assert_eq!(AgentApproval::find(conn,expired)?.unwrap().status,"expired");
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND handled_at IS NULL",[expired],|r|r.get::<_,i64>(0))?,0);Ok(())
    }).await.unwrap();
}
