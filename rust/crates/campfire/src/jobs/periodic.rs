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
//! | clear plaintext bot tokens | 24 h, its work once per process | here |
//! | retention prune | `RETENTION_PRUNE_INTERVAL` (24 h) | WS8 |
//! | presence leases | 1 min | WS17 |
//! | meeting status | 1 min | WS14 |
//! | out of office | 1 min | WS14 |
//! | board sla nudges | 5 min | WS12 |
//! | board stale digests | 1 h | WS12 |
//! | streaming messages | 30 s | WS11 |
//! | slack imports (`SlackImport.sweep_stalled!`) | 30 s | WS16 |
//!
//! The huddle reconciler's steps (overdue invitations, stale streams) belong to WS13, and are
//! registered in [`huddle_reconciler`].

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

/// `bin/periodic`'s and `bin/huddle-reconcile`'s intervals, read at boot. An invalid one fails
/// the boot, as it aborts those scripts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intervals {
    /// `EVENT_REMINDERS_INTERVAL`: reminders, scheduled messages and poll closing.
    pub reminders: Duration,
    /// `RETENTION_PRUNE_INTERVAL`
    pub retention: Duration,
    /// `HUDDLE_RECONCILE_INTERVAL`
    pub huddle: Duration,
}

impl Intervals {
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        Ok(Self {
            reminders: interval_from_env(&lookup, "EVENT_REMINDERS_INTERVAL", Duration::from_secs(30))?,
            retention: interval_from_env(&lookup, "RETENTION_PRUNE_INTERVAL", Duration::from_secs(24 * HOUR))?,
            huddle: interval_from_env(&lookup, "HUDDLE_RECONCILE_INTERVAL", HUDDLE_RECONCILE_INTERVAL)?,
        })
    }

    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }
}

/// The loops [`super::start`] runs.
pub struct Loops {
    pub periodic: Periodic<App>,
    pub huddle: Periodic<App>,
}

impl Loops {
    pub fn new(intervals: Intervals) -> Self {
        Self { periodic: periodic(intervals), huddle: huddle_reconciler(intervals) }
    }

    /// Runs each loop with tasks until `stopping`.
    pub(super) fn spawn(self, app: &App, stopping: &watch::Sender<bool>) -> Vec<JoinHandle<()>> {
        [self.periodic, self.huddle]
            .into_iter()
            .filter(|periodic| !periodic.is_empty())
            .map(|periodic| periodic.spawn(app.clone(), app.db.env().clock.clone(), stopping.subscribe()))
            .collect()
    }
}

/// `Periodic::Runner`'s tasks that have been ported.
pub fn periodic(_intervals: Intervals) -> Periodic<App> {
    let mut periodic = Periodic::new("Periodic");
    periodic.task(Task::once("clear plaintext bot tokens", BOT_TOKEN_CLEAR_INTERVAL, |app: App| async move {
        clear_plaintext_bot_tokens(&app.db).await.map(drop)
    }));
    periodic
}

/// `Huddle::Reconciler`'s steps, every `HUDDLE_RECONCILE_INTERVAL`, each isolated from the
/// others' failures. None is ported yet.
pub fn huddle_reconciler(_intervals: Intervals) -> Periodic<App> {
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
