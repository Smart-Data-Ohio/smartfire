//! `bin/periodic` (`app/services/periodic/runner.rb`) and `bin/huddle-reconcile`
//! (`app/services/huddle/reconciler.rb`), which run in this process beside the job runner.
//!
//! `Periodic::Runner`'s tasks, and where each lives in the port. Only the ones with a Rust home
//! are registered in [`periodic`]; each workstream appends its own as it lands (a name, an
//! interval and an idempotent function, as in Ruby):
//!
//! | Task | Interval | Owner |
//! |---|---|---|
//! | delayed jobs (`Periodic::DelayedJobDrain`) | 30 s | none: the queue's runner claims jobs when `run_at` comes |
//! | event reminders | `EVENT_REMINDERS_INTERVAL` (30 s) | WS14 |
//! | saved item reminders | `EVENT_REMINDERS_INTERVAL` | WS8 |
//! | scheduled messages | `EVENT_REMINDERS_INTERVAL` | WS8 |
//! | poll closing | `EVENT_REMINDERS_INTERVAL` | WS8 |
//! | stuck rooms (`Room::DestroyJob.reenqueue_stuck!`) | 5 min | WS8 |
//! | stranded agent webhooks | 30 s | WS11 |
//! | stuck GitHub claims | 30 s | WS15 |
//! | stuck Fizzy claims | 30 s | WS15 |
//! | calendar push channels | 1 h (`Calendar::PushChannel::RENEW_INTERVAL`) | WS14 |
//! | clear plaintext bot tokens | 24 h, its work once per process | registered ([`clear_plaintext_bot_tokens_task`]) |
//! | retention prune | `RETENTION_PRUNE_INTERVAL` (24 h) | WS8 |
//! | presence leases | 1 min | WS17 |
//! | meeting status | 1 min | WS17 (WS14 refresh execution) |
//! | out of office | 1 min | WS17 (WS14 refresh execution) |
//! | board sla nudges | 5 min | WS12 |
//! | board stale digests | 1 h | WS12 |
//! | streaming messages | 30 s | WS11 |
//! | slack imports (`SlackImport.sweep_stalled!`) | 30 s | WS16 |
//!
//! The huddle reconciler's steps (overdue invitations, stale streams) belong to WS13, and are
//! registered in [`huddle_reconciler`].
//!
//! The plaintext scrub runs once per process, retrying failures. Parity seeds store only
//! digests, so booting the scheduler preserves their deterministic test keys.
//!
//! An invalid interval disables only the loop that reads it, as it aborts only the Rails process
//! that reads it (`bin/periodic` for `EVENT_REMINDERS_INTERVAL` and `RETENTION_PRUNE_INTERVAL`,
//! `bin/huddle-reconcile` for `HUDDLE_RECONCILE_INTERVAL`): the error is logged at boot, and the
//! web server, cable, jobs and the other loop keep running.

use std::time::Duration;

use campfire_db::{CachedStatements, Database};
use campfire_jobs::periodic::{Periodic, Task, interval_from_env};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::app::App;

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;

pub const BOT_TOKEN_CLEAR_INTERVAL: Duration = Duration::from_secs(24 * HOUR);
/// `Huddle::Reconciler::DEFAULT_INTERVAL`
pub const HUDDLE_RECONCILE_INTERVAL: Duration = Duration::from_secs(5);

/// `bin/periodic`'s and `bin/huddle-reconcile`'s intervals, read at boot. A loop whose
/// interval is invalid is `None`: disabled, as the script that reads it aborts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intervals {
    pub periodic: Option<PeriodicIntervals>,
    /// `HUDDLE_RECONCILE_INTERVAL`
    pub huddle: Option<Duration>,
}

/// `bin/periodic`'s intervals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeriodicIntervals {
    /// `EVENT_REMINDERS_INTERVAL`: reminders, scheduled messages and poll closing.
    pub reminders: Duration,
    /// `RETENTION_PRUNE_INTERVAL`
    pub retention: Duration,
}

impl Intervals {
    /// Logs an error for each loop an invalid interval disables.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let periodic = interval_from_env(&lookup, "EVENT_REMINDERS_INTERVAL", Duration::from_secs(30)).and_then(|reminders| {
            let retention = interval_from_env(&lookup, "RETENTION_PRUNE_INTERVAL", Duration::from_secs(24 * HOUR))?;
            Ok(PeriodicIntervals { reminders, retention })
        });
        let huddle = interval_from_env(&lookup, "HUDDLE_RECONCILE_INTERVAL", HUDDLE_RECONCILE_INTERVAL);
        Self { periodic: enabled("Periodic", periodic), huddle: enabled("Huddle reconciliation", huddle) }
    }

    pub fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }
}

fn enabled<T>(label: &str, interval: anyhow::Result<T>) -> Option<T> {
    interval.inspect_err(|error| tracing::error!(%error, "{label} disabled")).ok()
}

/// The loops [`super::start`] runs; `None` when disabled.
pub struct Loops {
    pub periodic: Option<Periodic<App>>,
    pub huddle: Option<Periodic<App>>,
}

impl Loops {
    pub fn new(intervals: Intervals) -> Self {
        Self { periodic: intervals.periodic.map(periodic), huddle: intervals.huddle.map(huddle_reconciler) }
    }

    /// Runs each loop with tasks until `stopping`.
    pub(super) fn spawn(self, app: &App, stopping: &watch::Sender<bool>) -> Vec<JoinHandle<()>> {
        [self.periodic, self.huddle]
            .into_iter()
            .flatten()
            .filter(|periodic| !periodic.is_empty())
            .map(|periodic| periodic.spawn(app.clone(), app.db.env().clock.clone(), stopping.subscribe()))
            .collect()
    }
}

/// Messaging tasks with domain implementations; notification policy is owned by WS17.
pub fn periodic(intervals: PeriodicIntervals) -> Periodic<App> {
    let mut periodic = Periodic::new("Periodic");
    periodic.task(clear_plaintext_bot_tokens_task());
    periodic.task(Task::new("stranded agent webhooks", Duration::from_secs(30), |app: App| async move {
        stranded_agent_webhooks(&app.db).await
    }));
    periodic.task(Task::new("event reminders", intervals.reminders, |app: App| async move {
        event_reminders(&app.db).await
    }));
    periodic.task(Task::new(
        "saved item reminders",
        intervals.reminders,
        |app: App| async move { saved_item_reminders(&app.db).await },
    ));
    periodic.task(Task::new(
        "scheduled messages",
        intervals.reminders,
        |app: App| async move { scheduled_messages(&app.db).await },
    ));
    periodic.task(Task::new(
        "poll closing",
        intervals.reminders,
        |app: App| async move { poll_closing(&app.db).await },
    ));
    periodic.task(Task::new("stuck rooms", Duration::from_secs(5*MINUTE), |app: App| async move {
        app.db.write(|tx|campfire_db::models::room_delete::reenqueue_stuck(tx,600)).await?;
        Ok(())
    }));
    periodic.task(Task::new("calendar push channels", Duration::from_secs(HOUR), |app: App| async move {
        crate::integrations::google::calendar::renew(&app).await?;
        Ok(())
    }));
    use crate::integrations::action_claims;
    periodic.task(Task::new("stuck GitHub claims", action_claims::SWEEP_INTERVAL, |app: App| async move {
        action_claims::recover_stuck_claims(&app.db, action_claims::GITHUB, app.db.env().now()).await;
        Ok(())
    }));
    periodic.task(Task::new("stuck Fizzy claims", action_claims::SWEEP_INTERVAL, |app: App| async move {
        action_claims::recover_stuck_claims(&app.db, action_claims::FIZZY, app.db.env().now()).await;
        Ok(())
    }));
    periodic.task(Task::new("retention prune", intervals.retention, |app: App| async move {
        app.db.write(|tx| { tx.emit_after_commit(campfire_db::Event::job(&campfire_db::models::retention::PruneJob{})); Ok(()) }).await?;
        Ok(())
    }));
    periodic.task(Task::new("presence leases", Duration::from_secs(MINUTE), |app: App| async move {
        app.db.write(|tx| campfire_db::WorkspacePresenceLease::prune(tx, 100)).await?;
        Ok(())
    }));
    periodic.task(Task::new("meeting status", Duration::from_secs(MINUTE), |app: App| async move {
        campfire_db::models::calendar_dispatch::dispatch_meetings(&app.db, app.db.env().now()).await?;
        Ok(())
    }));
    periodic.task(Task::new("out of office", Duration::from_secs(MINUTE), |app: App| async move {
        campfire_db::models::calendar_dispatch::dispatch_ooo(&app.db, app.db.env().now()).await?;
        Ok(())
    }));
    periodic
}
pub(super) async fn saved_item_reminders(db: &Database) -> anyhow::Result<()> {
    let now = db.env().now();
    let ids = db
        .read(move |conn| campfire_db::SavedItem::due_reminder_ids(conn, now))
        .await?;
    for id in ids {
        if let Err(error) = db
            .write(move |tx| campfire_db::SavedItem::dispatch_reminder(tx, id, now))
            .await
        {
            tracing::error!(id,%error,"Saved item reminder failed");
        }
    }
    Ok(())
}

pub(super) async fn event_reminders(db: &Database) -> anyhow::Result<()> {
    let now = db.env().now();
    let ids = db.read(move |conn| campfire_db::CalendarEvent::due_reminder_ids(conn, now)).await?;
    for id in ids {
        if let Err(error) = db.write(move |tx| campfire_db::CalendarEvent::dispatch_reminder(tx, id, now)).await {
            tracing::error!(id, %error, "Event reminder failed");
        }
    }
    Ok(())
}
pub(super) async fn scheduled_messages(db: &Database) -> anyhow::Result<()> {
    let now = db.env().now();
    let ids = db
        .read(move |conn| campfire_db::ScheduledMessage::due_candidate_ids(conn, now))
        .await?;
    for id in ids {
        if let Err(error) = db
            .write(move |tx| campfire_db::ScheduledMessage::dispatch(tx, id, now, false))
            .await
        {
            tracing::error!(id,%error,"Scheduled message failed");
        }
    }
    Ok(())
}
pub(super) async fn poll_closing(db: &Database) -> anyhow::Result<()> {
    let now = db.env().now();
    let ids = db
        .read(move |conn| campfire_db::Poll::due(conn, now))
        .await?;
    for id in ids {
        if let Err(error) = db
            .write(move |tx| campfire_db::Poll::close_by_id(tx, id, now))
            .await
        {
            tracing::error!(id,%error,"Poll closing failed");
        }
    }
    Ok(())
}

/// `Task.new("clear plaintext bot tokens", BOT_TOKEN_CLEAR_INTERVAL, clear_bot_tokens_once)`.
pub fn clear_plaintext_bot_tokens_task() -> Task<App> {
    Task::once("clear plaintext bot tokens", BOT_TOKEN_CLEAR_INTERVAL, |app: App| async move { clear_plaintext_bot_tokens(&app.db).await.map(drop) })
}

/// `Huddle::Reconciler`'s steps, every `HUDDLE_RECONCILE_INTERVAL`, each isolated from the
/// others' failures. None is ported yet.
pub fn huddle_reconciler(_interval: Duration) -> Periodic<App> {
    Periodic::new("Huddle reconciliation")
}

/// `Bots::ClearPlaintextTokens.run!`: for every user that still has a plaintext `bot_token`, the
/// digest is recomputed from it and the plaintext nulled, one conditional update per row (a key
/// reset between the read and the write wins). Returns how many rows it healed.
pub async fn clear_plaintext_bot_tokens(db: &Database) -> anyhow::Result<usize> {
    let tokens: Vec<(i64, String)> = db
        .read(|conn| {
            let mut statement = conn.prepare_cached(r#"SELECT "id", "bot_token" FROM "users" WHERE "bot_token" IS NOT NULL"#)?;
            Ok(statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await?;
    let mut healed = 0;
    for (id, plaintext) in tokens {
        let digest = campfire_db::user::digest_bot_token(&plaintext);
        let updated = db
            .write(move |tx| {
                Ok(tx.conn().execute_cached(
                    r#"UPDATE "users" SET "bot_token_digest" = ?1, "bot_token" = NULL WHERE "id" = ?2 AND "bot_token" = ?3"#,
                    rusqlite::params![digest, id, plaintext],
                )?)
            })
            .await?;
        healed += updated;
    }
    Ok(healed)
}

/// Rails rescues each recovery enqueue independently. Each row's job and stamp still commit
/// atomically on the durable queue, including when a different candidate's queue write fails.
pub(crate) async fn stranded_agent_webhooks(db:&Database)->anyhow::Result<()> {
    use campfire_db::models::agent_delivery as domain;
    let now=db.env().now();
    let candidates=db.read(move |c|domain::recovery_candidates(c,now)).await?;
    db.write(move |tx|domain::fail_exhausted(tx,now)).await?;
    for candidate in candidates {
        if let Err(error)=db.write(move |tx|domain::recover_one(tx,candidate)).await {
            tracing::error!(%error,"Stranded agent delivery recovery failed");
        }
    }
    Ok(())
}

#[cfg(test)]
mod ws17_tests {
    use super::*;
    #[test]
    fn ws17_calendar_sweeps_are_registered_once_each_minute() {
        let tasks = periodic(PeriodicIntervals {
            reminders: Duration::from_secs(30),
            retention: Duration::from_secs(24 * HOUR),
        });
        for name in ["meeting status", "out of office"] {
            let matching: Vec<_> = tasks.tasks().filter(|task| task.name() == name).collect();
            assert_eq!(matching.len(), 1, "{name}");
            assert_eq!(matching[0].interval(), Duration::from_secs(MINUTE));
        }
    }
}
