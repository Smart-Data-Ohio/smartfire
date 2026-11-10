use super::*;
use crate::{Agent, AgentChanges, AgentKind, WakePolicy};

#[test]
fn assistants_migration_preserves_existing_wake_behavior_and_defaults_new_personal_agents() {
    let catalog = crate::migrations::catalog();
    let old = crate::schema::generate(&catalog[..catalog.len() - 1]).unwrap();
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(&old.schema_sql).unwrap();
    for (user, kind) in [(1, "personal"), (2, "workspace")] {
        conn.execute("INSERT INTO agents(user_id,owner_id,kind,created_at,updated_at) VALUES(?,2,?,'2026-11-09','2026-11-09')",rusqlite::params![user,kind]).unwrap();
    }
    conn.execute_batch(&catalog.last().unwrap().sql).unwrap();
    assert_eq!(conn.query_row("SELECT COUNT(*) FROM agents WHERE wake_policy='anyone' AND paused_at IS NULL AND paused_by_id IS NULL",[],|r|r.get::<_,i64>(0)).unwrap(),2);
    conn.execute("INSERT INTO agents(user_id,owner_id,kind,created_at,updated_at) VALUES(3,2,'personal','2026-11-09','2026-11-09')",[]).unwrap();
    assert_eq!(
        conn.query_row("SELECT wake_policy FROM agents WHERE user_id=3", [], |r| {
            r.get::<_, WakePolicy>(0)
        })
        .unwrap(),
        WakePolicy::Owner
    );
    assert!(
        conn.execute("UPDATE agents SET wake_policy='invalid'", [])
            .is_err()
    );
}

#[test]
fn assistants_workspace_wake_and_suspension_override_pause() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        let mut agent = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
        assert_eq!(agent.kind, AgentKind::Workspace);
        agent.update(
            tx,
            AgentChanges {
                wake_policy: Some(WakePolicy::Owner),
                paused_at: Some(Some(tx.now())),
                paused_by_id: Some(Some(id("david"))),
                ..Default::default()
            },
        )?;
        assert_eq!(agent.wake_policy, WakePolicy::Anyone);
        assert!(agent.message_can_wake(id("kevin")));
        assert!(agent.paused());
        assert!(agent.active(tx.conn())?);
        agent.update(
            tx,
            AgentChanges {
                suspended_at: Some(Some(tx.now())),
                ..Default::default()
            },
        )?;
        assert!(!agent.paused());
        assert!(!agent.active(tx.conn())?);
        Ok(())
    });
}

#[test]
fn assistants_paused_messages_schedule_no_delivery_and_queued_webhooks_stop() {
    use crate::models::agent_delivery::{
        self as delivery, AgentEvent, DeliveryJob, EventWebhookJob,
    };
    let t = super::channel_thread_test::frozen();
    let event = t.write(|tx| {
        crate::Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        let message = crate::Message::create(
            tx,
            crate::NewMessage {
                room_id: id("watercooler"),
                creator_id: id("david"),
                markdown_source: Some("@[Bender Bot] before pause".into()),
                ..Default::default()
            },
        )?;
        let event = tx.conn().query_row(
            "SELECT id FROM agent_events WHERE message_id=? AND agent_id=?",
            rusqlite::params![message.id, id("bender_agent")],
            |r| r.get::<_, i64>(0),
        )?;
        delivery::perform_delivery(tx, event)?;
        assert_eq!(
            AgentEvent::find(tx.conn(), event)?.unwrap().webhook_status,
            "pending"
        );
        let mut agent = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
        agent.update(
            tx,
            AgentChanges {
                paused_at: Some(Some(tx.now())),
                ..Default::default()
            },
        )?;
        Ok(event)
    });
    t.sink.take();
    t.write(move |tx| {
        let message = crate::Message::create(
            tx,
            crate::NewMessage {
                room_id: id("watercooler"),
                creator_id: id("david"),
                markdown_source: Some("@[Bender Bot] during pause".into()),
                ..Default::default()
            },
        )?;
        let event_id = tx.conn().query_row(
            "SELECT id FROM agent_events WHERE message_id=? AND agent_id=?",
            rusqlite::params![message.id, id("bender_agent")],
            |r| r.get::<_, i64>(0),
        )?;
        let suppressed = AgentEvent::find(tx.conn(), event_id)?.unwrap();
        assert_eq!(suppressed.outcome.as_deref(), Some("suppressed"));
        assert_eq!(suppressed.detail.as_deref(), Some("Agent paused"));
        assert!(
            delivery::claim_webhook(
                tx,
                &EventWebhookJob {
                    event_id: event,
                    attempt: Some(0)
                }
            )?
            .is_none()
        );
        assert_eq!(
            AgentEvent::find(tx.conn(), event)?.unwrap().webhook_status,
            "none"
        );
        Ok(())
    });
    assert!(t.events().iter().all(|e|e.as_job::<DeliveryJob>().is_none() && e.as_job::<EventWebhookJob>().is_none()));
}
