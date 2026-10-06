//! PR214: returned schedules must equal the exact timestamp written to SQLite.
use super::*;
use crate::models::{
    agent_delivery::{self, AgentEvent, NewEvent},
    agent_work_events,
};
use crate::{ChannelThread, Clock, Timestamp};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};
struct AdvancingClock(AtomicI64);
impl Clock for AdvancingClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_microsecond(self.0.fetch_add(1, Ordering::SeqCst))
    }
}
fn compare(kind: &str) {
    let t = super::channel_thread_test::frozen();
    t.write(|tx|{
        tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=?",[id("bender_agent")])?;
        tx.conn().execute("INSERT OR IGNORE INTO memberships(room_id,user_id,created_at,updated_at) VALUES(?,?,?,?)",rusqlite::params![id("watercooler"),id("bender"),tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,last_activity_at,created_at,updated_at) VALUES(2106930000,?,?, 'Dynamic clock','planned',?,?,?,?)",rusqlite::params![id("watercooler"),id("david"),id("bender"),tx.now(),tx.now(),tx.now()])?;
        Ok(())
    });
    let mut env = t.db.env().clone();
    env.clock = Arc::new(AdvancingClock(AtomicI64::new(
        Timestamp::parse_db("2027-01-15 08:00:00")
            .unwrap()
            .as_microsecond(),
    )));
    let mut config = crate::Config::new(t.db.path());
    config.prepare = false;
    let db = crate::Database::open(config, env).unwrap();
    let kind = kind.to_owned();
    let returned = db
        .write_blocking(move |tx| {
            if kind == "approval_decided" {
                agent_delivery::create_delivered(
                    tx,
                    NewEvent {
                        agent_id: id("bender_agent"),
                        event_type: kind,
                        ..Default::default()
                    },
                )
            } else {
                let thread = ChannelThread::find(tx.conn(), 2106930000)?;
                Ok(agent_work_events::record_owner_change(
                    tx,
                    &thread,
                    None,
                    Some(id("bender")),
                    Some(id("david")),
                )?
                .remove(0))
            }
        })
        .unwrap();
    let stored = t.read(|conn| AgentEvent::find(conn, returned.id).map(|v| v.unwrap()));
    let actual = json!({"kind":returned.event_type,"webhook_status":returned.webhook_status,"equal":returned.webhook_next_attempt_at==stored.webhook_next_attempt_at,"delta_microseconds":returned.webhook_next_attempt_at.unwrap().as_microsecond()-stored.webhook_next_attempt_at.unwrap().as_microsecond()});
    println!(
        "PR214 advancing-clock returned={:?} stored={:?} receipt={actual}",
        returned.webhook_next_attempt_at, stored.webhook_next_attempt_at
    );
    let gold: Value =
        serde_json::from_str(include_str!("../../../../vectors/pr214_event_clock.json")).unwrap();
    let expected = gold["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == actual["kind"])
        .unwrap();
    for key in ["kind", "webhook_status", "equal", "delta_microseconds"] {
        assert_eq!(actual[key], expected[key], "{key}");
    }
    assert_eq!(
        returned.webhook_next_attempt_at,
        stored.webhook_next_attempt_at
    );
    assert_eq!(returned.created_at, stored.created_at);
    assert_eq!(returned.metadata, stored.metadata);
}
#[test]
fn pr214_delivered_schedule_matches_persisted_timestamp() {
    compare("approval_decided");
}
#[test]
fn pr214_work_schedule_matches_persisted_timestamp() {
    compare("work_assigned");
}
