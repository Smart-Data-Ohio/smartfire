use super::*;
use crate::{Agent, AgentApproval, Clock, NewApproval, Timestamp, User};
use std::sync::Mutex;

struct AdvancingClock(Mutex<Timestamp>);

impl Clock for AdvancingClock {
    fn now(&self) -> Timestamp {
        let mut at = self.0.lock().unwrap();
        *at = at.since(jiff::SignedDuration::from_micros(1));
        *at
    }
}

fn advancing_db() -> (tempfile::TempDir, Database) {
    let dir = tempfile::tempdir().unwrap();
    let env = Env {
        clock: Arc::new(AdvancingClock(Mutex::new(
            Timestamp::parse_db("2026-03-02 16:00:00").unwrap(),
        ))),
        bcrypt_cost: 4,
        ..Default::default()
    };
    let mut config = Config::new(dir.path().join("test.sqlite3"));
    config.environment = "test".into();
    let db = Database::open(config, env).unwrap();
    db.write_blocking(|tx| {
        fixtures::load(
            tx.conn(),
            &fixtures::reference_dir(),
            &fixtures::Options {
                now: tx.now(),
                bcrypt_cost: 4,
            },
        )
    })
    .unwrap();
    (dir, db)
}

fn approval(tx: &mut Tx<'_>) -> Result<AgentApproval> {
    AgentApproval::create(
        tx,
        NewApproval {
            agent_id: id("bender_agent"),
            action: "deploy".into(),
            summary: "Check revision".into(),
            ..Default::default()
        },
    )
}

#[test]
fn approval_expiry_and_cancellation_return_the_persisted_revision_on_advancing_clock() {
    let (_dir, db) = advancing_db();
    db.write_blocking(|tx| {
        let mut expired = approval(tx)?;
        tx.conn().execute(
            "UPDATE agent_approvals SET expires_at=? WHERE id=?",
            rusqlite::params![tx.now(), expired.id],
        )?;
        assert!(expired.expire_if_due(tx)?);
        let stored = AgentApproval::find(tx.conn(), expired.id)?.unwrap();
        assert_eq!(expired.updated_at, stored.updated_at, "expiry");

        let mut cancelled = approval(tx)?;
        assert!(cancelled.cancel_by_agent(tx)?.is_empty());
        let stored = AgentApproval::find(tx.conn(), cancelled.id)?.unwrap();
        assert_eq!(cancelled.updated_at, stored.updated_at, "cancellation");
        Ok(())
    })
    .unwrap();
}

#[test]
fn agent_status_returns_persisted_timestamps_on_advancing_clock() {
    let (_dir, db) = advancing_db();
    db.write_blocking(|tx| {
        let mut agent = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
        agent.status = "working".into();
        agent.save(tx)?;
        let stored = Agent::find(tx.conn(), agent.id)?.unwrap();
        assert_eq!(agent.updated_at, stored.updated_at);
        assert_eq!(agent.status_changed_at, stored.status_changed_at);
        agent.touch_last_seen(tx)?;
        let stored = Agent::find(tx.conn(), agent.id)?.unwrap();
        assert_eq!(agent.last_seen_at, stored.last_seen_at);
        Ok(())
    })
    .unwrap();
}

#[test]
fn agent_status_writes_advance_the_revision_with_a_frozen_clock() {
    let t = channel_thread_test::frozen();
    t.write(|tx| {
        let mut agent = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
        agent.status = "working".into();
        agent.save(tx)?;
        let working = agent.updated_at;
        agent.status = "idle".into();
        agent.save(tx)?;
        assert!(agent.updated_at > working);
        Ok(())
    });
}

#[test]
fn approval_decisions_advance_the_revision_with_a_frozen_clock() {
    let t = channel_thread_test::frozen();
    t.write(|tx| {
        let mut approval = approval(tx)?;
        let pending = approval.updated_at;
        assert!(
            approval
                .decide(tx, "approved", &User::find(tx.conn(), id("david"))?, None)?
                .is_empty()
        );
        assert!(approval.updated_at > pending);
        Ok(())
    });
}

#[test]
fn work_fact_writes_advance_the_revision_with_a_frozen_clock() {
    use crate::models::channel_thread::WorkChanges;
    use crate::{ChannelThread, NewChannelThread, Room, RoomType};
    let t = channel_thread_test::frozen();
    t.write(|tx| {
        let actor = User::find(tx.conn(), id("david"))?;
        let room = Room::create_for(
            tx,
            RoomType::Board,
            Some("Revisions"),
            actor.id,
            &[actor.id],
        )?;
        let mut thread = ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: actor.id,
                name: Some("Check revision".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
            None,
        )?;
        let mut previous = thread.updated_at;
        for status in ["in_progress", "blocked", "planned"] {
            thread.update_work(
                tx,
                &actor,
                WorkChanges {
                    status: Some(Some(status.into())),
                    ..Default::default()
                },
            )?;
            assert!(thread.updated_at > previous);
            previous = thread.updated_at;
        }
        Ok(())
    });
}

#[test]
fn agent_steps_return_persisted_revisions_on_an_advancing_clock() {
    use crate::{AgentStep, AgentStepChanges, Message, NewAgentStep, NewMessage};
    let (_dir, db) = advancing_db();
    db.write_blocking(|tx| {
        let message = Message::create(
            tx,
            NewMessage {
                room_id: id("watercooler"),
                creator_id: id("bender"),
                markdown_source: Some("Check revisions".into()),
                ..Default::default()
            },
        )?;
        let mut step = AgentStep::create(
            tx,
            NewAgentStep {
                agent_id: id("bender_agent"),
                message_id: Some(message.id),
                name: "Check revision".into(),
                ..Default::default()
            },
        )?;
        step.update(
            tx,
            AgentStepChanges {
                status: Some("done".into()),
                ..Default::default()
            },
        )?;
        let stored = AgentStep::find(tx.conn(), step.id)?.unwrap();
        assert_eq!(step.updated_at, stored.updated_at);
        Ok(())
    })
    .unwrap();
}

#[test]
fn agent_step_writes_advance_the_revision_with_a_frozen_clock() {
    use crate::{AgentStep, AgentStepChanges, Message, NewAgentStep, NewMessage};
    let t = channel_thread_test::frozen();
    t.write(|tx| {
        let message = Message::create(
            tx,
            NewMessage {
                room_id: id("watercooler"),
                creator_id: id("bender"),
                markdown_source: Some("Check revisions".into()),
                ..Default::default()
            },
        )?;
        let mut step = AgentStep::create(
            tx,
            NewAgentStep {
                agent_id: id("bender_agent"),
                message_id: Some(message.id),
                name: "Check revision".into(),
                ..Default::default()
            },
        )?;
        let before = step.updated_at;
        step.update(
            tx,
            AgentStepChanges {
                status: Some("done".into()),
                ..Default::default()
            },
        )?;
        assert!(step.updated_at > before);
        Ok(())
    });
}

#[test]
fn bulk_thread_expiry_cannot_regress_a_work_revision() {
    use crate::{ChannelThread, NewChannelThread};
    let t = channel_thread_test::frozen();
    t.write(|tx| {
        let thread = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("watercooler"),
                creator_id: id("david"),
                name: Some("Check revision".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
        )?;
        let previous = tx.now().since(jiff::SignedDuration::from_micros(1));
        tx.conn().execute(
            "UPDATE channel_threads SET updated_at=?,last_activity_at=? WHERE id=?",
            rusqlite::params![
                previous,
                tx.now().ago(jiff::SignedDuration::from_hours(100)),
                thread.id
            ],
        )?;
        assert!(ChannelThread::close_stale_in(tx, Some(thread.room_id))? > 0);
        let stored = ChannelThread::find(tx.conn(), thread.id)?;
        assert!(stored.updated_at > previous);
        assert_eq!(stored.closed_at, Some(tx.now()));
        Ok(())
    });
}

#[test]
fn a_stale_settings_save_returns_the_work_state_of_its_new_revision() {
    use crate::models::channel_thread::WorkChanges;
    use crate::{ChannelThread, NewChannelThread, Room, RoomType};
    let t = channel_thread_test::frozen();
    t.write(|tx| {
        let actor = User::find(tx.conn(), id("david"))?;
        let room = Room::create_for(
            tx,
            RoomType::Board,
            Some("Revisions"),
            actor.id,
            &[actor.id],
        )?;
        let mut thread = ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: actor.id,
                name: Some("Check revision".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
            None,
        )?;
        let mut stale = thread.clone();
        thread.update_work(
            tx,
            &actor,
            WorkChanges {
                status: Some(Some("in_progress".into())),
                ..Default::default()
            },
        )?;
        stale.update_settings(tx, Some("Renamed"), None)?;
        let stored = ChannelThread::find(tx.conn(), thread.id)?;
        assert_eq!(stale.updated_at, stored.updated_at);
        assert_eq!(stale.work_status, Some("in_progress".into()));
        assert_eq!(stale.work_status_changed_at, stored.work_status_changed_at);
        Ok(())
    });
}

#[test]
fn an_agent_save_returns_untouched_timestamps_from_its_new_revision() {
    let t = channel_thread_test::frozen();
    t.write(|tx| {
        let mut agent = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
        let mut stale = agent.clone();
        agent.status = "working".into();
        agent.save(tx)?;
        agent.touch_last_seen(tx)?;
        stale.status = "working".into();
        stale.status_note = Some("Still working".into());
        stale.save(tx)?;
        let stored = Agent::find(tx.conn(), agent.id)?.unwrap();
        assert_eq!(stale.updated_at, stored.updated_at);
        assert_eq!(stale.status_changed_at, stored.status_changed_at);
        assert_eq!(stale.last_seen_at, stored.last_seen_at);
        Ok(())
    });
}
