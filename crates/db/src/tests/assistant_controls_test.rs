use super::*;
use crate::{Agent, AgentChanges, AgentKind, WakePolicy};

#[test]
fn assistants_migration_preserves_existing_wake_behavior_and_defaults_new_personal_agents() {
    let catalog = crate::migrations::catalog();
    let pause = catalog
        .iter()
        .position(|m| m.version == "20261109140000")
        .unwrap();
    let old = crate::schema::generate(&catalog[..pause]).unwrap();
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(&old.schema_sql).unwrap();
    for user in 1..=3 {
        conn.execute("INSERT INTO users(id,name,created_at,updated_at) VALUES(?,?,'2026-11-09','2026-11-09')", rusqlite::params![user,format!("User {user}")]).unwrap();
    }
    for (user, kind) in [(1, "personal"), (2, "workspace")] {
        conn.execute("INSERT INTO agents(user_id,owner_id,kind,created_at,updated_at) VALUES(?,2,?,'2026-11-09','2026-11-09')",rusqlite::params![user,kind]).unwrap();
    }
    conn.execute_batch("INSERT INTO agent_credentials(id,agent_id,created_by_id,name,token_digest,token_last_four,created_at,updated_at) VALUES(1,1,2,'Migration','fixture','1234','2026-11-09','2026-11-09');
        INSERT INTO agent_events(id,agent_id,event_type,created_at) VALUES(1,1,'mention','2026-11-09');
        INSERT INTO mcp_event_subscriptions(id,agent_id,agent_credential_id,name,arguments,url,secret,refresh_before,watermark_event_id,created_at,updated_at) VALUES('subscription',1,1,'mention','{}','https://events.example/hook','fixture','2026-11-10',0,'2026-11-09','2026-11-09');").unwrap();
    conn.execute("INSERT INTO mcp_event_deliveries(subscription_id,agent_event_id,webhook_id,status,next_attempt_at,created_at,updated_at,body) VALUES('subscription',1,'msg_paused','suppressed','2026-11-09','2026-11-09','2026-11-09','cached occurrence')", []).unwrap();
    conn.execute_batch(&catalog[pause].sql).unwrap();
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
    conn.execute(
        "UPDATE agents SET paused_at='2026-11-09',paused_by_id=2 WHERE user_id=1",
        [],
    )
    .unwrap();
    for migration in &catalog[pause + 1..] {
        conn.execute_batch(&migration.sql).unwrap();
    }
    assert_eq!(
        conn.query_row(
            "SELECT wake_policy,paused_at,paused_by_id FROM agents WHERE user_id=1",
            [],
            |r| Ok((
                r.get::<_, WakePolicy>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?
            ))
        )
        .unwrap(),
        (WakePolicy::Anyone, "2026-11-09".into(), 2)
    );
    assert_eq!(
        conn.query_row(
            "SELECT status,body FROM mcp_event_deliveries WHERE webhook_id='msg_paused'",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        )
        .unwrap(),
        ("suppressed".into(), "cached occurrence".into())
    );
    conn.execute("UPDATE mcp_event_deliveries SET status='expired'", [])
        .unwrap();
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
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

#[test]
fn pause_regression_work_assignment_stays_hidden_after_resume() {
    use crate::models::{agent_delivery::AgentEvent, agent_event_access};
    use crate::{AgentGrant, ChannelThread, NewChannelThread, NewGrant, Room};
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        let agent = id("bender_agent");
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        tx.conn()
            .execute("DELETE FROM agent_grants WHERE agent_id=?", [agent])?;
        tx.conn().execute(
            "UPDATE agents SET kind='personal',owner_id=?,paused_at=? WHERE id=?",
            rusqlite::params![id("david"), tx.now(), agent],
        )?;
        for capability in ["read_messages", "post_messages"] {
            AgentGrant::create(
                tx,
                NewGrant {
                    agent_id: agent,
                    room_id: Some(id("watercooler")),
                    capability: capability.into(),
                    granted_by_id: id("david"),
                    ..Default::default()
                },
            )?;
        }
        ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id: id("watercooler"),
                creator_id: id("david"),
                name: Some("Paused assignment".into()),
                work_status: Some("planned".into()),
                work_owner_id: Some(id("bender")),
                ..Default::default()
            },
            None,
        )?;
        tx.conn()
            .execute("UPDATE agents SET paused_at=NULL WHERE id=?", [agent])?;
        assert!(
            agent_event_access::readable_page(tx.conn(), agent, 0, None)?.is_empty(),
            "paused assignment leaked after resume"
        );
        let events = AgentEvent::for_agent(tx.conn(), agent)?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].outcome.as_deref(), Some("suppressed"));
        assert_eq!(events[0].detail.as_deref(), Some("Agent paused"));
        assert_eq!(events[0].webhook_status, "none");
        Ok(())
    });
    assert!(t.events().iter().all(|e| {
        e.as_job::<crate::models::agent_delivery::EventWebhookJob>()
            .is_none()
    }));
}

#[test]
fn pause_regression_slash_command_never_queues_or_revives() {
    use crate::models::{
        agent_delivery::{self as d, AgentEvent},
        agent_slash_command,
    };
    use crate::{AgentSlashCommand, NewAgentSlashCommand, Room};
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        let agent = id("bender_agent");
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        AgentSlashCommand::create(
            tx,
            NewAgentSlashCommand {
                agent_id: agent,
                room_id: id("watercooler"),
                name: "pauseprobe".into(),
                ..Default::default()
            },
        )?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        tx.conn().execute(
            "UPDATE agents SET paused_at=? WHERE id=?",
            rusqlite::params![tx.now(), agent],
        )?;
        agent_slash_command::invoke(
            tx,
            &crate::slash_commands::Context {
                user_id: id("david"),
                room_id: id("watercooler"),
                thread_id: None,
                huddles_configured: false,
            },
            "pauseprobe",
            "during pause",
        )?
        .unwrap();
        tx.conn()
            .execute("UPDATE agents SET paused_at=NULL WHERE id=?", [agent])?;
        let event = AgentEvent::for_agent(tx.conn(), agent)?.pop().unwrap();
        assert!(
            d::claim_webhook(
                tx,
                &d::EventWebhookJob {
                    event_id: event.id,
                    attempt: Some(0)
                }
            )?
            .is_none(),
            "paused slash webhook revived"
        );
        assert_eq!(event.outcome.as_deref(), Some("suppressed"));
        assert_eq!(event.webhook_status, "none");
        Ok(())
    });
    assert!(t.events().iter().all(|e| {
        e.as_job::<crate::models::agent_delivery::EventWebhookJob>()
            .is_none()
    }));
}

#[test]
fn pause_regression_claimed_completion_cannot_revive_after_pause_and_resume() {
    use crate::models::agent_delivery::{self as d, AgentEvent, NewEvent};
    let t = super::channel_thread_test::frozen();
    for retry in [false, true] {
        t.sink.take();
        t.write(move |tx| {
            let agent_id = id("bender_agent");
            let event = AgentEvent::create(
                tx,
                NewEvent {
                    agent_id,
                    event_type: "mention".into(),
                    outcome: Some("delivered".into()),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE agent_events SET webhook_status='pending' WHERE id=?",
                [event.id],
            )?;
            let claimed = d::claim_webhook(
                tx,
                &d::EventWebhookJob {
                    event_id: event.id,
                    attempt: Some(0),
                },
            )?
            .unwrap();
            let mut agent = Agent::find(tx.conn(), agent_id)?.unwrap();
            agent.update(
                tx,
                AgentChanges {
                    paused_at: Some(Some(tx.now())),
                    ..Default::default()
                },
            )?;
            agent.update(
                tx,
                AgentChanges {
                    paused_at: Some(None),
                    ..Default::default()
                },
            )?;
            d::finish_webhook(
                tx,
                &claimed,
                if retry {
                    d::AttemptOutcome::Retry("late 500".into(), None)
                } else {
                    d::AttemptOutcome::Delivered
                },
            )?;
            let saved = AgentEvent::find(tx.conn(), event.id)?.unwrap();
            assert_eq!(
                saved.webhook_status, "none",
                "late completion revived delivery"
            );
            assert_eq!(saved.outcome.as_deref(), Some("suppressed"));
            Ok(())
        });
        assert!(
            t.events()
                .iter()
                .all(|e| e.as_job::<d::EventWebhookJob>().is_none())
        );
    }
}

#[test]
fn pause_regression_all_callback_types_record_suppression() {
    use crate::models::agent_delivery::{self as d, NewEvent};
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE agents SET paused_at=? WHERE id=?",
            rusqlite::params![tx.now(), id("bender_agent")],
        )?;
        for kind in d::DELIVERABLE_TYPES {
            let event = d::record_delivered(
                tx,
                NewEvent {
                    agent_id: id("bender_agent"),
                    room_id: Some(id("watercooler")),
                    event_type: kind.into(),
                    ..Default::default()
                },
            )?;
            assert_eq!(event.outcome.as_deref(), Some("suppressed"), "{kind}");
            assert_eq!(event.detail.as_deref(), Some("Agent paused"), "{kind}");
            assert_eq!(event.webhook_status, "none", "{kind}");
        }
        Ok(())
    });
    assert!(
        t.events()
            .iter()
            .all(|e| e.as_job::<d::EventWebhookJob>().is_none())
    );
}

#[test]
fn pause_regression_deleted_work_never_queues_during_pause() {
    use crate::models::{agent_delivery::AgentEvent, agent_event_access};
    use crate::{ChannelThread, NewChannelThread, Room};
    let t = super::channel_thread_test::frozen();
    let thread = t.write(|tx| {
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        let thread = ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id: id("watercooler"),
                creator_id: id("david"),
                name: Some("Delete during pause".into()),
                work_status: Some("planned".into()),
                work_owner_id: Some(id("bender")),
                ..Default::default()
            },
            None,
        )?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        tx.conn().execute(
            "UPDATE agents SET paused_at=? WHERE id=?",
            rusqlite::params![tx.now(), id("bender_agent")],
        )?;
        Ok(thread)
    });
    t.sink.take();
    t.write(move |tx| thread.destroy(tx));
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE agents SET paused_at=NULL WHERE id=?",
            [id("bender_agent")],
        )?;
        assert!(
            agent_event_access::readable_page(tx.conn(), id("bender_agent"), 0, None)?.is_empty(),
            "deleted work revived"
        );
        let events = AgentEvent::for_agent(tx.conn(), id("bender_agent"))?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].outcome.as_deref(), Some("suppressed"));
        Ok(())
    });
    assert!(
        t.events()
            .iter()
            .all(|e| !matches!(e, Event::Job(job) if job.class == "Agent::EventWebhookJob"))
    );
}

#[test]
fn pause_regression_pending_mcp_is_discarded_but_expired_history_is_preserved() {
    use crate::models::agent_delivery::{AgentEvent, NewEvent};
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        let agent_id = id("bender_agent");
        tx.conn().execute("INSERT INTO mcp_event_subscriptions(id,agent_id,agent_credential_id,name,arguments,url,secret,refresh_before,watermark_event_id,created_at,updated_at) VALUES('pause-history',?,?,'mention','{}','https://events.example/hook','fixture',?,0,?,?)", rusqlite::params![agent_id,id("bender_main"),tx.now(),tx.now(),tx.now()])?;
        for status in ["pending", "expired"] {
            let event = AgentEvent::create(tx, NewEvent { agent_id, event_type: "mention".into(), outcome: Some("delivered".into()), ..Default::default() })?;
            tx.conn().execute("INSERT INTO mcp_event_deliveries(subscription_id,agent_event_id,webhook_id,status,next_attempt_at,created_at,updated_at,body) VALUES('pause-history',?,?,?,?,?,?, 'cached occurrence')", rusqlite::params![event.id,status,status,tx.now(),tx.now(),tx.now()])?;
        }
        let mut agent = Agent::find(tx.conn(),agent_id)?.unwrap();
        agent.update(tx,AgentChanges{paused_at:Some(Some(tx.now())),..Default::default()})?;
        agent.update(tx,AgentChanges{paused_at:Some(None),..Default::default()})?;
        for (webhook, expected) in [("pending","suppressed"),("expired","expired")] {
            let saved = tx.conn().query_row("SELECT status,body FROM mcp_event_deliveries WHERE webhook_id=?", [webhook], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?;
            assert_eq!(saved,(expected.into(),"cached occurrence".into()));
        }
        Ok(())
    });
}
