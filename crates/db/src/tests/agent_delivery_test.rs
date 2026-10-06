use super::*;
use crate::{Message, NewMessage, Room};
use rusqlite::params;

#[test]
fn ws11_agent_message_chain_records_pending_and_rate_suppression() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_events", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        for i in 0..28 {
            Message::create(
                tx,
                NewMessage {
                    room_id: id("watercooler"),
                    creator_id: id("david"),
                    markdown_source: Some(format!("Hey @[Bender Bot] {i}")),
                    ..Default::default()
                },
            )?;
        }
        let count = |outcome| {
            tx.conn().query_row(
                "SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND outcome=?",
                params![id("bender_agent"), outcome],
                |r| r.get::<_, i64>(0),
            )
        };
        assert_eq!(count("pending")?, 20);
        assert_eq!(count("suppressed")?, 8);
        Ok(())
    });
}

#[test]
fn ws11_agent_delivery_claims_revocation_ack_and_recovery() {
    use crate::models::agent_delivery::{self as d, AgentEvent, NewEvent};
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_events", [])?;
        Room::find(tx.conn(),id("watercooler"))?.grant_to(tx,&[id("bender")])?;
        let message=Message::create(tx,NewMessage {room_id:id("watercooler"),creator_id:id("david"),body:Some("Trigger".into()),..Default::default()})?;
        let new=||NewEvent {agent_id:id("bender_agent"),room_id:Some(message.room_id),message_id:Some(message.id),actor_id:Some(id("david")),event_type:"mention".into(),outcome:Some("pending".into()),..Default::default()};
        let e=AgentEvent::create(tx,new())?;
        tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,revoked_at,created_at,updated_at) VALUES (?,'read_messages',?,?,?,?)",params![e.agent_id,id("david"),tx.now(),tx.now(),tx.now()])?;
        d::perform_delivery(tx,e.id)?;d::perform_delivery(tx,e.id)?;
        assert_eq!(AgentEvent::find(tx.conn(),e.id)?.unwrap().outcome.as_deref(),Some("suppressed"));
        let n:i64=tx.conn().query_row("SELECT COUNT(*) FROM agent_events WHERE event_type='delivery_suppressed_revoked'",[],|r|r.get(0))?;assert_eq!(n,1);
        let ack=AgentEvent::create(tx,new())?;
        tx.conn().execute("UPDATE agent_events SET outcome='acknowledged' WHERE id=?",[ack.id])?;
        d::perform_delivery(tx,ack.id)?;d::perform_delivery(tx,ack.id)?;
        let ack=AgentEvent::find(tx.conn(),ack.id)?.unwrap();assert_eq!(ack.webhook_status,"pending");assert_eq!(ack.outcome.as_deref(),Some("acknowledged"));
        let claimed=d::claim_webhook(tx,&d::EventWebhookJob {event_id:ack.id,attempt:Some(0)})?.unwrap();
        assert!(d::claim_webhook(tx,&d::EventWebhookJob {event_id:ack.id,attempt:Some(0)})?.is_none());
        d::finish_webhook(tx,&claimed,d::AttemptOutcome::Retry("429".into(),Some(std::time::Duration::from_secs(600))))?;
        let future=AgentEvent::find(tx.conn(),ack.id)?.unwrap().webhook_next_attempt_at;
        d::fail_exhausted(tx,tx.now())?;for candidate in d::recovery_candidates(tx.conn(),tx.now())? {d::recover_one(tx,candidate)?;}assert_eq!(AgentEvent::find(tx.conn(),ack.id)?.unwrap().webhook_next_attempt_at,future);
        tx.conn().execute("UPDATE agent_events SET webhook_attempts=5,webhook_next_attempt_at=? WHERE id=?",params![tx.now().ago(jiff::SignedDuration::from_mins(8)),ack.id])?;
        d::fail_exhausted(tx,tx.now())?;for candidate in d::recovery_candidates(tx.conn(),tx.now())? {d::recover_one(tx,candidate)?;}let ack=AgentEvent::find(tx.conn(),ack.id)?.unwrap();assert_eq!(ack.webhook_status,"failed");assert_eq!(ack.webhook_last_error.as_deref(),Some("Delivery attempts exhausted without a recorded outcome"));
        Ok(())
    });
    let attempts: Vec<_> = t
        .events()
        .iter()
        .filter_map(|e| e.as_job::<crate::models::agent_delivery::EventWebhookJob>())
        .collect();
    assert_eq!(attempts.len(), 2); // Ack's first POST and its scheduled retry; no duplicate or early recovery.
}

#[test]
fn ws11_agent_rate_limit_is_atomic_across_concurrent_posts() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_events", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        Ok(())
    });
    let mut handles = vec![];
    for worker in 0..4 {
        let db = t.db.clone();
        handles.push(std::thread::spawn(move || {
            for i in 0..7 {
                db.write_blocking(move |tx| {
                    Message::create(
                        tx,
                        NewMessage {
                            room_id: id("watercooler"),
                            creator_id: id("david"),
                            markdown_source: Some(format!("@[Bender Bot] {worker}-{i}")),
                            ..Default::default()
                        },
                    )
                })
                .unwrap();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    let counts = t.read(|c| {
        Ok((
            c.query_row(
                "SELECT COUNT(*) FROM agent_events WHERE outcome='pending'",
                [],
                |r| r.get::<_, i64>(0),
            )?,
            c.query_row(
                "SELECT COUNT(*) FROM agent_events WHERE outcome='suppressed'",
                [],
                |r| r.get::<_, i64>(0),
            )?,
        ))
    });
    assert_eq!(counts, (20, 8));
}

#[test]
fn ws11_five_claimed_attempts_match_rails_exhaustion_vectors() {
    use crate::models::agent_delivery::{self as d, AgentEvent, NewEvent};
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_delivery_contract.json"
    ))
    .unwrap();
    let t = super::channel_thread_test::frozen();
    t.write(move |tx| {
        let e = AgentEvent::create(
            tx,
            NewEvent {
                agent_id: id("bender_agent"),
                event_type: "github_action_completed".into(),
                ..Default::default()
            },
        )?;
        tx.conn().execute(
            "UPDATE agent_events SET webhook_status='pending' WHERE id=?",
            [e.id],
        )?;
        for case in vectors["exhaustion"].as_array().unwrap() {
            let attempt = case["attempts"].as_i64().unwrap();
            let claimed = d::claim_webhook(
                tx,
                &d::EventWebhookJob {
                    event_id: e.id,
                    attempt: Some(attempt - 1),
                },
            )?
            .unwrap();
            d::finish_webhook(
                tx,
                &claimed,
                d::AttemptOutcome::Retry("Retryable failure".into(), None),
            )?;
            let actual = AgentEvent::find(tx.conn(), e.id)?.unwrap();
            assert_eq!(actual.webhook_status, case["status"].as_str().unwrap());
            assert_eq!(actual.webhook_attempts, attempt);
            assert_eq!(
                actual
                    .webhook_next_attempt_at
                    .unwrap()
                    .jiff()
                    .as_microsecond()
                    - tx.now().jiff().as_microsecond(),
                (case["delay"].as_f64().unwrap() * 1e6) as i64
            );
        }
        assert!(
            d::claim_webhook(
                tx,
                &d::EventWebhookJob {
                    event_id: e.id,
                    attempt: Some(5)
                }
            )?
            .is_none()
        );
        Ok(())
    });
    assert_eq!(
        t.events()
            .iter()
            .filter(|e| e
                .as_job::<crate::models::agent_delivery::EventWebhookJob>()
                .is_some())
            .count(),
        4
    );
}
