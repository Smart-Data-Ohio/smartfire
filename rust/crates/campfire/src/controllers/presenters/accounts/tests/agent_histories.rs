//! Human history pagination, lazy expiry and content policy through the real HTTP stack.
use super::*;
use campfire_db::{Agent, AgentApproval, AgentGrant, NewApproval};

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
    let response = owner.get(&format!("/agents/{id}/approvals")).await;
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
async fn histories_apply_distinct_rails_access_and_back_links() {
    let test = boot_seed("default").await.expect("default seed");
    let id = configure(&test).await;
    let mut owner = test.browser("198.51.100.181");
    owner.sign_in(&test.label("emails.kevin")).await;
    for page in ["approvals", "events"] {
        let reply = owner.get(&format!("/agents/{id}/{page}")).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        assert!(
            reply
                .text()
                .contains(&format!("href=\"/users/{}\"", test.label("users.bender")))
        );
    }
    let mut other = test.browser("198.51.100.182");
    other.sign_in(&test.label("emails.jz")).await;
    assert_eq!(
        other.get(&format!("/agents/{id}/approvals")).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        other.get(&format!("/agents/{id}/events")).await.status,
        StatusCode::FORBIDDEN
    );
    let mut token = test.browser("198.51.100.183");
    for (page, status) in [
        ("approvals", StatusCode::NOT_FOUND),
        ("events", StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            token
                .request(
                    Method::GET,
                    &format!("/agents/{id}/{page}"),
                    &[("authorization", "Bearer bender-test-secret-1234")],
                    None
                )
                .await
                .status,
            status
        );
        assert_eq!(
            token
                .get(&format!(
                    "/agents/{id}/{page}?bot_key={}",
                    encode(&test.label("bot_keys.bender"))
                ))
                .await
                .status,
            status
        );
    }
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
        .get(&format!("/agents/{id}/approvals?status=pending"))
        .await;
    assert!(reply.text().contains("Future request"));
    assert!(!reply.text().contains("Overdue request"));
    let reply = owner
        .get(&format!("/agents/{id}/approvals?status=expired"))
        .await;
    assert!(reply.text().contains("Overdue request"));
    assert!(!reply.text().contains("Future request"));
    test.booted.app.db.read(move |conn| {
        assert_eq!(AgentApproval::find(conn,expired)?.unwrap().status,"expired");
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND handled_at IS NULL",[expired],|r|r.get::<_,i64>(0))?,0);Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn histories_use_fifty_row_pages_newest_first_and_preserve_filters() {
    let test = boot_seed("default").await.expect("default seed");
    let id = configure(&test).await;
    test.booted
        .app
        .db
        .write(move |tx| {
            for i in 0..51 {
                AgentApproval::create(
                    tx,
                    NewApproval {
                        agent_id: id,
                        action: "deploy".into(),
                        summary: format!("Page request {i:02}"),
                        ..Default::default()
                    },
                )?;
                campfire_db::models::agent_delivery::AgentEvent::create(
                    tx,
                    campfire_db::models::agent_delivery::NewEvent {
                        agent_id: id,
                        event_type: "posted".into(),
                        outcome: Some("suppressed".into()),
                        detail: Some(format!("Page event {i:02}")),
                        ..Default::default()
                    },
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut admin = test.browser("198.51.100.185");
    admin.sign_in(&test.label("emails.david")).await;
    for (page, filter, marker) in [
        ("approvals", "status=pending", "Page request"),
        ("events", "outcome=suppressed", "Page event"),
    ] {
        let path = format!("/agents/{id}/{page}?{filter}");
        let reply = admin.get(&path).await;
        let html = reply.text();
        assert_eq!(html.matches(marker).count(), 50);
        assert!(html.contains(&format!("{marker} 50")));
        assert!(!html.contains(&format!("{marker} 00")));
        assert!(html.contains("page=2") && html.contains(filter));
        assert!(!html.contains(">Newer</a>"));
        let html = admin.get(&format!("{path}&page=2")).await.text();
        assert!(html.contains(&format!("{marker} 00")));
        assert!(!html.contains(">Older</a>"));
        assert!(html.contains("page=1") && html.contains(filter));
    }
}
#[tokio::test]
async fn ledger_redacts_content_when_viewer_agent_membership_or_read_grant_is_missing() {
    let test = boot_seed("default").await.expect("default seed");
    let id = configure(&test).await;
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let room: i64 = test.label("rooms.watercooler").parse().unwrap();
    let event_id=test.booted.app.db.write(move |tx| {
        let message:i64=tx.conn().query_row("SELECT id FROM messages WHERE room_id=? ORDER BY id LIMIT 1",[room],|r|r.get(0))?;
        tx.conn().execute("UPDATE action_text_rich_texts SET body='<p>Ledger private content</p>' WHERE record_type='Message' AND record_id=?",[message])?;
        Ok(campfire_db::models::agent_delivery::AgentEvent::create(tx,campfire_db::models::agent_delivery::NewEvent{agent_id:id,room_id:Some(room),message_id:Some(message),event_type:"mention".into(),outcome:Some("delivered".into()),detail:Some("Ledger metadata remains".into()),..Default::default()})?.id)
    }).await.unwrap();
    let mut admin = test.browser("198.51.100.186");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/agents/{id}/events");
    assert!(
        admin
            .get(&path)
            .await
            .text()
            .contains("Ledger private content")
    );
    let mut owner = test.browser("198.51.100.187");
    owner.sign_in(&test.label("emails.kevin")).await;
    test.booted
        .app
        .db
        .write(move |tx| campfire_db::Room::find(tx.conn(), room)?.grant_to(tx, &[712064548]))
        .await
        .unwrap();
    assert!(
        owner
            .get(&path)
            .await
            .text()
            .contains("Ledger private content")
    );
    test.booted
        .app
        .db
        .write(move |tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=712064548 AND room_id=?",
                [room],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let html = owner.get(&path).await.text();
    assert!(!html.contains("Ledger private content"));
    assert!(html.contains("Ledger metadata remains"));
    test.booted
        .app
        .db
        .write(move |tx| {
            AgentGrant::create(
                tx,
                campfire_db::NewGrant {
                    agent_id: id,
                    capability: "post_messages".into(),
                    room_id: None,
                    granted_by_id: 127326141,
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        !admin
            .get(&path)
            .await
            .text()
            .contains("Ledger private content")
    );
    let read_grant = test
        .booted
        .app
        .db
        .write(move |tx| {
            Ok(AgentGrant::create(
                tx,
                campfire_db::NewGrant {
                    agent_id: id,
                    capability: "read_messages".into(),
                    room_id: Some(room),
                    granted_by_id: 127326141,
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    assert!(
        admin
            .get(&path)
            .await
            .text()
            .contains("Ledger private content")
    );
    test.booted
        .app
        .db
        .write(move |tx| AgentGrant::find(tx.conn(), read_grant)?.unwrap().revoke(tx))
        .await
        .unwrap();
    assert!(
        !admin
            .get(&path)
            .await
            .text()
            .contains("Ledger private content")
    );
    test.booted
        .app
        .db
        .write(move |tx| {
            AgentGrant::create(
                tx,
                campfire_db::NewGrant {
                    agent_id: id,
                    capability: "read_messages".into(),
                    room_id: Some(room),
                    granted_by_id: 127326141,
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                rusqlite::params![bot, room],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let html = admin.get(&path).await.text();
    assert!(!html.contains("Ledger private content"));
    assert!(html.contains("Ledger metadata remains"));
    test.booted
        .app
        .db
        .read(move |conn| {
            assert!(
                campfire_db::models::agent_delivery::AgentEvent::find(conn, event_id)?.is_some()
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn approval_cards_restrict_external_approve_controls_but_keep_deny_notes() {
    let test = boot_seed("default").await.expect("default seed");
    let id = configure(&test).await;
    test.booted
        .app
        .db
        .write(move |tx| {
            AgentApproval::create(
                tx,
                NewApproval {
                    agent_id: id,
                    action: "github.comment".into(),
                    summary: "GitHub request".into(),
                    github_login: Some("visible-octocat".into()),
                    ..Default::default()
                },
            )?;
            AgentApproval::create(
                tx,
                NewApproval {
                    agent_id: id,
                    action: "fizzy.close".into(),
                    summary: "Fizzy request".into(),
                    fizzy_user_name: Some("Visible Pat".into()),
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut owner = test.browser("198.51.100.188");
    owner.sign_in(&test.label("emails.kevin")).await;
    let html = owner.get(&format!("/agents/{id}/approvals")).await.text();
    assert!(html.contains("Only an administrator can approve GitHub write actions."));
    assert!(html.contains("Only an administrator can approve Fizzy write actions."));
    assert!(html.contains("Acts on GitHub as @visible-octocat"));
    assert!(html.contains("Acts on Fizzy as Visible Pat"));
    assert!(html.contains("Deny note (optional)"));
    assert_eq!(html.matches("decision=approved").count(), 1); // The seed's generic approval remains approvable.
}
