//! Rails `test/models/slack_import_test.rb`, plus real independent-writer races.
use std::sync::{Arc, Barrier};

use rusqlite::params;
use serde_json::json;

use super::{TestDb, id};
use crate::models::slack_import::{
    self, IssueLevel, Kind, Mode, NewImport, SlackImport, StepJob, StepStatus, UndoJob,
};
use crate::{
    Clock, Connection, Database, Env, Error, Event, EventSink, Result, TestClock, Timestamp, Tx,
};

fn setup() -> TestDb {
    let t = TestDb::with_clock(
        TestClock::frozen_at(Timestamp::parse_db("2026-03-02 16:00:00.123456").unwrap()),
        4,
    );
    t.write(|tx| {
        tx.conn().execute("INSERT INTO slack_workspaces (id, client_id, client_secret, created_at, updated_at) VALUES (1, 'fixture-client', NULL, ?, ?)", params![tx.now(), tx.now()])?;
        tx.conn().execute("INSERT INTO slack_connections (id, slack_workspace_id, user_id, slack_user_id, created_at, updated_at) VALUES (1, 1, ?, 'UADMIN', ?, ?)", params![id("david"), tx.now(), tx.now()])?;
        Ok(())
    });
    t
}
fn start(t: &TestDb) -> i64 {
    t.write(|tx| {
        Ok(SlackImport::create(
            tx,
            NewImport {
                workspace_id: 1,
                connection_id: Some(1),
                user_id: super::id("david"),
                kind: Kind::Workspace,
                mode: Mode::Import,
                options: json!({}),
            },
        )?
        .id)
    })
}
fn run(t: &TestDb, id: i64) -> SlackImport {
    t.read(|c| Ok(SlackImport::find(c, id)?.unwrap()))
}
fn status(t: &TestDb, id: i64, status: &str) {
    let status = status.to_owned();
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE slack_imports SET status = ? WHERE id = ?",
            params![status, id],
        )?;
        Ok(())
    });
}
fn lease(t: &TestDb, id: i64) -> String {
    t.write(move |tx| Ok(SlackImport::acquire_step_lease(tx, id, StepStatus::Running)?.unwrap()))
}

#[test]
fn slack_import_atomic_undo_claim_refuses_queue_arriving_after_precheck() {
    let t = setup();
    let first = start(&t);
    status(&t, first, "completed");
    let original = run(&t, first);
    assert!(
        t.read(|c| original.undo_blocked_reason(c, original.created_at))
            .is_none()
    );
    let queued = start(&t);
    let before = t.events().len();
    assert!(!t.write(move |tx| SlackImport::claim_undo(tx, first)));
    assert_eq!(run(&t, first).status, "completed");
    assert_eq!(run(&t, queued).status, "queued");
    assert_eq!(t.events().len(), before);
}
fn race<T: Send + 'static>(
    a: Database,
    b: Database,
    f: impl Fn(Database) -> T + Send + Sync + 'static,
) -> Vec<T> {
    let barrier = Arc::new(Barrier::new(2));
    let f = Arc::new(f);
    [a, b]
        .into_iter()
        .map(|db| {
            let barrier = barrier.clone();
            let f = f.clone();
            std::thread::spawn(move || {
                barrier.wait();
                f(db)
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect()
}

#[test]
fn slack_import_start_stamps_and_enqueues_with_required_associations() {
    let t = setup();
    let id = start(&t);
    let r = run(&t, id);
    assert!(r.active());
    assert!(r.cancellable());
    assert_eq!(r.status, "queued");
    assert_eq!(r.state["enqueued_at"], "2026-03-02T16:00:00.123456Z");
    assert!(
        t.events()
            .iter()
            .any(|e| e.as_job::<StepJob>().is_some_and(|j| j.import_id == id))
    );
    assert!(
        t.try_write(|tx| SlackImport::create(
            tx,
            NewImport {
                workspace_id: -1,
                connection_id: None,
                user_id: super::id("david"),
                kind: Kind::Workspace,
                mode: Mode::Import,
                options: json!({})
            }
        ))
        .is_err()
    );
}

#[test]
fn slack_import_two_writers_claim_one_run() {
    let t = setup();
    let first = start(&t);
    let second = start(&t);
    let ids = [first, second];
    let outcomes = race(t.db.clone(), t.another_process(), move |db| {
        db.write_blocking(move |tx| {
            let mut won = 0;
            for id in ids {
                won += usize::from(SlackImport::claim_running(tx, id)?);
            }
            Ok(won)
        })
        .unwrap()
    });
    assert_eq!(outcomes.iter().sum::<usize>(), 1);
    assert_eq!(run(&t, first).status, "running");
    assert_eq!(run(&t, second).status, "queued");
}

#[test]
fn slack_import_two_workers_execute_a_step_once() {
    let t = setup();
    let id = start(&t);
    status(&t, id, "running");
    t.write(|tx| {
        tx.conn()
            .execute_batch("CREATE TABLE executed_steps (run_id INTEGER)")?;
        Ok(())
    });
    let outcomes = race(t.db.clone(), t.another_process(), move |db| {
        db.write_blocking(move |tx| {
            let token = SlackImport::acquire_step_lease(tx, id, StepStatus::Running)?;
            if token.is_some() {
                tx.conn()
                    .execute("INSERT INTO executed_steps VALUES (?)", [id])?;
            }
            Ok(token)
        })
        .unwrap()
    });
    assert_eq!(outcomes.iter().filter(|v| v.is_some()).count(), 1);
    assert_eq!(
        t.read(
            |c| Ok(c.query_row("SELECT COUNT(*) FROM executed_steps", [], |r| r
                .get::<_, i64>(0))?)
        ),
        1
    );
}

#[test]
fn slack_import_cancelled_and_failed_leases_block_until_release_or_staleness() {
    for terminal in ["cancelled", "failed"] {
        let t = setup();
        let first = start(&t);
        let second = start(&t);
        assert!(t.write(move |tx| SlackImport::claim_running(tx, first)));
        let token = lease(&t, first);
        status(&t, first, terminal);
        assert!(!t.write(move |tx| SlackImport::claim_running(tx, second)));
        if terminal == "cancelled" {
            assert!(t.write(move |tx| SlackImport::release_step_lease(tx, first, &token)));
        } else {
            t.travel(300);
        }
        assert!(t.write(move |tx| SlackImport::claim_running(tx, second)));
    }
}

#[test]
fn slack_import_completed_lease_looking_state_does_not_block_claims() {
    let t = setup();
    let first = start(&t);
    let second = start(&t);
    status(&t, first, "running");
    lease(&t, first);
    status(&t, first, "completed");
    assert!(t.write(move |tx| SlackImport::claim_running(tx, second)));
}

#[test]
fn slack_import_undoing_blocks_claims_and_wrong_status_cannot_acquire() {
    let t = setup();
    let first = start(&t);
    let second = start(&t);
    status(&t, first, "undoing");
    assert!(!t.write(move |tx| SlackImport::claim_running(tx, second)));
    assert!(
        t.write(move |tx| SlackImport::acquire_step_lease(tx, second, StepStatus::Running))
            .is_none()
    );
    assert!(
        t.write(move |tx| SlackImport::acquire_step_lease(tx, first, StepStatus::Undoing))
            .is_some()
    );
}

#[test]
fn slack_import_stale_takeover_preserves_progress_and_fences_old_holder() {
    let t = setup();
    let id = start(&t);
    status(&t, id, "running");
    let first = lease(&t, id);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE slack_imports SET state = json_set(state, '$.convo_index', 3) WHERE id = ?",
            [id],
        )?;
        Ok(())
    });
    t.travel(300);
    let second = lease(&t, id);
    assert_ne!(first, second);
    let old = first.clone();
    assert!(!t.write(move |tx| SlackImport::refresh_step_lease(tx, id, &old)));
    assert!(!t.write(move |tx| SlackImport::release_step_lease(tx, id, &first)));
    assert_eq!(run(&t, id).state["convo_index"], 3);
    assert_eq!(run(&t, id).state["step_lease_token"], second);
}

#[test]
fn slack_import_refresh_and_release_only_touch_lease_keys() {
    let t = setup();
    let id = start(&t);
    status(&t, id, "running");
    let token = lease(&t, id);
    t.travel(299);
    let held = token.clone();
    assert!(t.write(move |tx| SlackImport::refresh_step_lease(tx, id, &held)));
    t.travel(2);
    assert!(run(&t, id).step_lease_fresh(t.now()));
    assert!(!t.write(move |tx| SlackImport::release_step_lease(tx, id, "wrong")));
    assert!(!t.write(move |tx| SlackImport::refresh_step_lease(tx, id, " ")));
    assert!(t.write(move |tx| SlackImport::release_step_lease(tx, id, &token)));
    let r = run(&t, id);
    assert!(!r.step_lease_fresh(t.now()));
    assert!(r.state.get("step_lease_token").is_none());
    assert!(r.state.get("enqueued_at").is_some());
}

#[test]
fn slack_import_cancel_is_idempotent_and_keeps_inflight_lease() {
    let t = setup();
    let first = start(&t);
    assert!(t.write(move |tx| SlackImport::claim_running(tx, first)));
    lease(&t, first);
    let second = start(&t);
    t.sink.take();
    assert!(t.write(move |tx| SlackImport::cancel(tx, first)));
    assert!(!t.write(move |tx| SlackImport::cancel(tx, first)));
    assert!(run(&t, first).step_lease_fresh(t.now()));
    assert!(!run(&t, first).active());
    assert!(run(&t, first).finished_at.is_some());
    assert!(t.events().is_empty());
    assert_eq!(run(&t, second).status, "queued");
}

#[test]
fn slack_import_pending_stamp_cooldown_and_oldest_queue_handoff() {
    let t = setup();
    let first = start(&t);
    let second = start(&t);
    t.sink.take();
    assert_eq!(t.write(SlackImport::kick_next_queued), None);
    t.write(move |tx| SlackImport::clear_pending_step_job(tx, second));
    // The oldest run still has its pending job, so don't kick the younger one.
    assert_eq!(t.write(SlackImport::kick_next_queued), None);
    t.travel(300);
    assert_eq!(t.write(SlackImport::kick_next_queued), Some(first));
    assert_eq!(t.write(SlackImport::kick_next_queued), None);
    assert_eq!(t.events().len(), 1);
}

#[test]
fn slack_import_overlapping_sweeps_only_kick_queued_run_once() {
    let t = setup();
    let id = start(&t);
    t.sink.take();
    t.travel(301);
    race(t.db.clone(), t.another_process(), |db| {
        db.write_blocking(SlackImport::sweep_stalled).unwrap()
    });
    let jobs: Vec<_> = t
        .events()
        .iter()
        .filter_map(Event::as_job::<StepJob>)
        .collect();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].import_id, id);
}

#[test]
fn slack_import_overlapping_stalled_sweeps_cannot_double_execute_step() {
    let t = setup();
    let id = start(&t);
    status(&t, id, "running");
    let first = lease(&t, id);
    t.travel(301);
    t.sink.take();
    race(t.db.clone(), t.another_process(), |db| {
        db.write_blocking(SlackImport::sweep_stalled).unwrap()
    });
    assert_eq!(
        t.events()
            .iter()
            .filter_map(Event::as_job::<StepJob>)
            .count(),
        2
    );
    let outcomes = race(t.db.clone(), t.another_process(), move |db| {
        db.write_blocking(move |tx| SlackImport::acquire_step_lease(tx, id, StepStatus::Running))
            .unwrap()
    });
    assert_eq!(outcomes.iter().filter(|v| v.is_some()).count(), 1);
    assert!(!t.write(move |tx| SlackImport::release_step_lease(tx, id, &first)));
}

#[test]
fn slack_import_sweep_reenqueues_only_stalled_running_and_undoing() {
    let t = setup();
    let running = start(&t);
    let undoing = start(&t);
    status(&t, running, "running");
    status(&t, undoing, "undoing");
    t.sink.take();
    t.write(SlackImport::sweep_stalled);
    let e = t.sink.take();
    assert_eq!(e.iter().filter_map(Event::as_job::<StepJob>).count(), 1);
    assert_eq!(e.iter().filter_map(Event::as_job::<UndoJob>).count(), 1);
    lease(&t, running);
    t.travel(300);
    t.write(SlackImport::sweep_stalled);
    // Strict < cutoff: exactly five minutes is not a stale heartbeat yet.
    assert_eq!(
        t.events()
            .iter()
            .filter_map(Event::as_job::<StepJob>)
            .count(),
        0
    );
}

#[test]
fn slack_import_two_undo_claims_serialize_and_reset_progress() {
    let t = setup();
    let first = start(&t);
    let second = start(&t);
    status(&t, first, "completed");
    status(&t, second, "completed");
    t.sink.take();
    let outcomes = race(t.db.clone(), t.another_process(), move |db| {
        db.write_blocking(move |tx| {
            Ok(usize::from(SlackImport::undo(tx, first)?)
                + usize::from(SlackImport::undo(tx, second)?))
        })
        .unwrap()
    });
    assert_eq!(outcomes.iter().sum::<usize>(), 1);
    let r = run(&t, first);
    assert_eq!(r.status, "undoing");
    assert_eq!(r.state, json!({"phase":"undo"}));
    assert_eq!(r.stats["phase"], "undo");
    assert!(r.finished_at.is_none());
    assert_eq!(
        t.events()
            .iter()
            .filter_map(Event::as_job::<UndoJob>)
            .count(),
        1
    );
}

#[test]
fn slack_import_undo_refuses_dry_active_and_fresh_finishing_leases() {
    for (other_status, with_lease) in [
        ("queued", false),
        ("running", false),
        ("undoing", false),
        ("cancelled", true),
        ("failed", true),
    ] {
        let t = setup();
        let first = start(&t);
        status(&t, first, "completed");
        let second = start(&t);
        if with_lease {
            status(&t, second, "running");
            lease(&t, second);
        }
        status(&t, second, other_status);
        assert!(!t.write(move |tx| SlackImport::undo(tx, first)));
        assert_eq!(run(&t, first).status, "completed");
    }
    let t = setup();
    let id = start(&t);
    assert!(!t.write(move |tx| SlackImport::undo(tx, id)));
    status(&t, id, "completed");
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE slack_imports SET mode = 'dry_run' WHERE id = ?",
            [id],
        )?;
        Ok(())
    });
    assert!(!t.write(move |tx| SlackImport::undo(tx, id)));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE slack_imports SET mode = 'import' WHERE id = ?",
            [id],
        )?;
        Ok(())
    });
    status(&t, id, "running");
    lease(&t, id);
    status(&t, id, "cancelled");
    assert_eq!(
        run(&t, id)
            .undo_blocked_reason(&Connection::open(t.db.path()).unwrap(), t.now())
            .unwrap()
            .unwrap(),
        "This import is still finishing. Wait for it to finish, then undo."
    );
    assert!(!t.write(move |tx| SlackImport::undo(tx, id)));
    t.travel(300);
    assert!(t.write(move |tx| SlackImport::undo(tx, id)));
}

#[test]
fn slack_import_undo_lifo_uses_started_at_and_names_other_importer() {
    let t = setup();
    let first = start(&t);
    let second = start(&t);
    t.write(move |tx| {
        for (run, action) in [(first, "create"), (second, "merge")] {
            tx.conn().execute("UPDATE slack_imports SET status = 'completed', started_at = ?, stats = ? WHERE id = ?", params![tx.now(), json!({"conversations":[{"id":"C1","target":{"action":action}}]}).to_string(), run])?;
        }
        Ok(())
    });
    let reason = t.read(move |c| {
        run_reason(
            c,
            first,
            Clock::now(&TestClock::frozen_at(
                Timestamp::parse_db("2026-03-02 16:00:00.123456").unwrap(),
            )),
        )
    });
    assert_eq!(
        reason.unwrap(),
        format!(
            "A later import (#{second}) also imported some of these conversations; undo that one first."
        )
    );
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE slack_imports SET user_id = ? WHERE id = ?",
            params![id("kevin"), second],
        )?;
        Ok(())
    });
    let now = t.now();
    assert_eq!(
        t.read(move |c| run_reason(c, first, now)).unwrap(),
        "A later import by Kevin also imported some of these conversations. It has to be undone first; ask them or an administrator."
    );
    assert!(!t.write(move |tx| SlackImport::undo(tx, first)));
    status(&t, second, "undone");
    assert!(t.write(move |tx| SlackImport::undo(tx, first)));
}
fn run_reason(c: &Connection, id: i64, now: Timestamp) -> Result<Option<String>> {
    SlackImport::find(c, id)?
        .unwrap()
        .undo_blocked_reason(c, now)
}

#[test]
fn slack_import_undo_overlap_covers_crash_mapping_but_not_skip() {
    let t = setup();
    let first = start(&t);
    let second = start(&t);
    t.write(move |tx| {
        tx.conn().execute("UPDATE slack_imports SET status = 'completed', started_at = ?", [tx.now()])?;
        tx.conn().execute("INSERT INTO slack_import_records (slack_workspace_id, slack_import_id, slack_kind, slack_key, record_type, record_id, created_record, created_at, updated_at) VALUES (1, ?, 'conversation', 'C1', 'Room', 1, 1, ?, ?)", params![first, tx.now(), tx.now()])?;
        tx.conn().execute("UPDATE slack_imports SET stats = ? WHERE id = ?", params![json!({"conversations":[{"id":"C1","target":{"action":"skip"}}]}).to_string(), second])?;
        Ok(())
    });
    assert!(
        t.read(move |c| SlackImport::find(c, first)?
            .unwrap()
            .later_overlapping_import(c))
            .is_none()
    );
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE slack_imports SET stats = ? WHERE id = ?",
            params![
                json!({"conversations":[{"id":"C1","target":{"action":"merge"}}]}).to_string(),
                second
            ],
        )?;
        Ok(())
    });
    assert_eq!(
        t.read(move |c| Ok(SlackImport::find(c, first)?
            .unwrap()
            .later_overlapping_import(c)?
            .unwrap()
            .id)),
        second
    );
}

#[test]
fn slack_import_issue_cap_adds_one_suppression_notice_and_validates_messages() {
    let t = setup();
    let id = start(&t);
    assert!(
        t.try_write(move |tx| SlackImport::record_issue(tx, id, IssueLevel::Error, None, " "))
            .is_err()
    );
    t.write(move |tx| {
        for _ in 0..1005 {
            SlackImport::record_issue(tx, id, IssueLevel::Error, Some("C1"), "bad message")?;
        }
        Ok(())
    });
    assert_eq!(
        t.read(|c| Ok(
            c.query_row("SELECT COUNT(*) FROM slack_import_issues", [], |r| r
                .get::<_, i64>(0))?
        )),
        1001
    );
    assert_eq!(
        t.read(|c| Ok(c.query_row(
            "SELECT level || ':' || message FROM slack_import_issues ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0)
        )?)),
        "warning:Further issues suppressed (over 1000)"
    );
}

struct RejectJob;
impl EventSink for RejectJob {
    fn emit(&self, _: Event) {}
    fn persist(&self, _: &Tx<'_>, event: &Event) -> Result<()> {
        if matches!(event, Event::Job(_)) {
            Err(Error::Other("injected enqueue rejection".into()))
        } else {
            Ok(())
        }
    }
}
#[test]
fn slack_import_start_and_undo_roll_back_when_enqueue_fails() {
    let t = setup();
    let id = start(&t);
    status(&t, id, "completed");
    let mut config = crate::Config::new(t.db.path());
    config.prepare = false;
    let other = Database::open(
        config,
        Env {
            clock: Arc::new(t.clock.clone()),
            sink: Arc::new(RejectJob),
            ..Env::default()
        },
    )
    .unwrap();
    assert!(
        other
            .write_blocking(move |tx| SlackImport::undo(tx, id))
            .is_err()
    );
    assert_eq!(run(&t, id).status, "completed");
    assert!(
        other
            .write_blocking(|tx| SlackImport::create(
                tx,
                NewImport {
                    workspace_id: 1,
                    connection_id: Some(1),
                    user_id: super::id("david"),
                    kind: Kind::Workspace,
                    mode: Mode::Import,
                    options: json!({})
                }
            ))
            .is_err()
    );
    assert_eq!(
        t.read(
            |c| Ok(c.query_row("SELECT COUNT(*) FROM slack_imports", [], |r| r
                .get::<_, i64>(0))?)
        ),
        1
    );
}

#[test]
fn slack_import_lease_stamp_is_utc_with_microseconds() {
    let local: jiff::Timestamp = "2026-03-03T01:00:00.123456+09:00".parse().unwrap();
    assert_eq!(
        slack_import::lease_stamp(Timestamp::from_jiff(local)),
        "2026-03-02T16:00:00.123456Z"
    );
}
