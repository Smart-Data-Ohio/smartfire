//! One writer thread that owns the write connection and takes a bounded queue of work, plus
//! a pool of reader connections. Each write runs in `BEGIN IMMEDIATE`
//! (`default_transaction_mode: immediate` in `reference/config/database.yml`), then its
//! after-commit work runs in order, outside the transaction, the way Active Record runs
//! `after_commit` callbacks.
//!
//! WAL checkpoints run on a checkpointer thread with a connection of its own, not on the writer.
//! Rails keeps SQLite's auto-checkpoint: once a commit leaves the WAL at 1,000 pages or more,
//! that commit checkpoints (PASSIVE) before returning, fsyncing the WAL and then the database
//! (with the WAL header's fsync when the next write restarts the WAL, ~12 ms here, every ~64
//! message posts), and every write queued behind it waits. Here the writer's commits only note
//! the WAL's size, and every 1,000 pages it grows wake the checkpointer, which runs the same
//! PASSIVE checkpoint while writes carry on appending to the WAL.
//!
//! Durability is the same as Rails': `journal_mode=wal` with `synchronous=normal`, so a commit
//! doesn't fsync, and what was committed since the WAL was last synced can be lost to a power
//! failure (never to a crash of the process); every checkpoint syncs the WAL, and one runs for
//! every 1,000 pages written, as in Rails.
//!
//! The trade-off is WAL size. SQLite only restarts the WAL from its beginning once a checkpoint
//! has caught up with it entirely, which a background checkpoint never does while writes keep
//! coming. So the WAL grows past 1,000 pages under sustained writes (it restarts in the first
//! lull), and at [`WAL_LIMIT_PAGES`] (~40 MB) the writer checkpoints it itself with RESTART,
//! stalling writes as Rails' commits do, but once per 10,000 pages instead of per 1,000.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use rusqlite::{Connection, OpenFlags};
use tokio::sync::{mpsc, oneshot};

use crate::error::{Error, Result};
use crate::events::{Event, EventSink, NullSink};
use crate::rich_text::{BasicRichText, RichText};
use crate::schema;
use crate::time::{Clock, SystemClock, Timestamp};

/// Everything models need besides the connection: the clock, where side effects go, and
/// the Action Text adapter.
#[derive(Clone)]
pub struct Env {
    pub clock: Arc<dyn Clock>,
    pub sink: Arc<dyn EventSink>,
    pub rich_text: Arc<dyn RichText>,
    /// BCrypt cost for `has_secure_password`. Rails uses `BCrypt::Engine.cost` (12), or
    /// `MIN_COST` (4) in the test environment.
    pub bcrypt_cost: u32,
    /// Configured default host/protocol, supplied from the mail URL configuration.
    pub default_url_origin: String,
    /// App-owned reference domains run on Message's real save hooks, in its transaction.
    /// The flag lets importers reconcile rows without scheduling network fetches.
    pub message_reference_syncs: Vec<MessageReferenceSync>,
    /// App-owned account disconnects run inside User::deactivate, before its status save.
    pub user_deactivation_hooks: Vec<UserDeactivationHook>,
    /// Deterministic input providers for cross-runtime fixture comparisons. Production
    /// builds expose no provider; UUIDs remain random and insert_all uses SQLite's clock.
    #[cfg(feature = "test-support")]
    pub fixture_inputs: Option<Arc<FixtureInputs>>,
    /// WS16 flagged fixture-only enrollment entropy; never bypasses TOTP confirmation.
    #[cfg(feature = "test-support")]
    pub fixture_auth_inputs: Option<Arc<FixtureAuthInputs>>,
}

#[cfg(feature = "test-support")]
pub struct FixtureAuthInputs {
    pub totp_secret: String,
    pub backup_codes: Vec<String>,
}

#[cfg(feature = "test-support")]
pub struct FixtureInputs {
    pub message_uuid: Arc<dyn Fn() -> String + Send + Sync>,
    pub sqlite_now: Timestamp,
}

pub type UserDeactivationHook = fn(&mut Tx<'_>, &crate::User) -> Result<()>;

pub type MessageReferenceSync = fn(&mut Tx<'_>, &crate::Message, bool) -> Result<()>;

impl Default for Env {
    fn default() -> Self {
        Self {
            clock: Arc::new(SystemClock),
            sink: Arc::new(NullSink),
            rich_text: Arc::new(BasicRichText),
            bcrypt_cost: 12,
            default_url_origin: "http://example.com".into(),
            message_reference_syncs: Vec::new(),
            user_deactivation_hooks: Vec::new(),
            #[cfg(feature = "test-support")]
            fixture_inputs: None,
            #[cfg(feature = "test-support")]
            fixture_auth_inputs: None,
        }
    }
}

impl Env {
    pub fn now(&self) -> Timestamp {
        self.clock.now()
    }

    pub(crate) fn message_uuid(&self) -> String {
        #[cfg(feature = "test-support")]
        if let Some(inputs) = &self.fixture_inputs {
            return (inputs.message_uuid)();
        }
        crate::sql::uuid()
    }

    pub(crate) fn sqlite_now_sql(&self) -> std::borrow::Cow<'static, str> {
        #[cfg(feature = "test-support")]
        if let Some(inputs) = &self.fixture_inputs {
            let now = inputs.sqlite_now;
            return format!(
                "'{}.{:03}'",
                now.jiff().strftime("%Y-%m-%d %H:%M:%S"),
                now.subsec_microsecond() / 1000
            )
            .into();
        }
        crate::time::SQLITE_NOW.into()
    }
}

struct CommitPreparation {
    key: &'static str,
    id: i64,
    hook: AfterCommitHook,
}

type AfterCommitHook = Box<dyn FnOnce(&mut Tx<'_>) -> Result<()> + Send>;

enum AfterCommit {
    RecordHook {
        key: &'static str,
        id: i64,
        hook: AfterCommitHook,
    },
    RecordJob {
        table: &'static str,
        id: i64,
        event: Option<Event>,
        condition: Option<fn(&Connection, i64) -> Result<bool>>,
    },
    RecordBroadcast {
        table: &'static str,
        id: i64,
        event: Event,
    },
    RecordHooks {
        table: &'static str,
        id: i64,
        hooks: Vec<AfterCommitHook>,
    },
    Hook(AfterCommitHook),
    Event(Event),
    SettledBroadcast(Event),
}

impl AfterCommit {
    /// Queue rows were persisted before COMMIT. Waking their runner is not a
    /// model callback; a later callback error cannot revoke those committed jobs.
    fn wake_committed_job(self, env: &Env) {
        match self {
            Self::RecordJob {
                event: Some(event), ..
            }
            | Self::Event(
                event @ (Event::Job(_)
                | Event::PushMessage { .. }
                | Event::RemoveBannedContent { .. }
                | Event::DeliverWebhook { .. }
                | Event::PurgeBlob { .. }),
            ) => env.sink.emit(event),
            _ => (),
        }
    }
}

/// A write in progress: the writer connection inside a transaction (or, while after-commit
/// work runs, outside one), the environment, and the queued after-commit work.
pub struct Tx<'c> {
    conn: &'c Connection,
    env: &'c Env,
    in_transaction: bool,
    after_commit: Vec<AfterCommit>,
    settled_broadcasts: &'c RefCell<Vec<Event>>,
    commit_finalizers: Vec<Box<dyn FnOnce() + Send>>,
    commit_preparations: Vec<CommitPreparation>,
    /// The first error persisting an emitted event (see [`EventSink::persist`]), which rolls the
    /// transaction back.
    persist_error: Option<Error>,
}

impl<'c> Tx<'c> {
    pub fn model_callback(&mut self, phase: crate::callbacks::Phase, record_id: i64) -> Result<()> {
        let sink = self.env.sink.clone();
        sink.model_callback(self, crate::callbacks::Callback { phase, record_id })
    }

    pub fn conn(&self) -> &'c Connection {
        self.conn
    }

    pub fn env(&self) -> &'c Env {
        self.env
    }

    /// `Time.current`
    pub fn now(&self) -> Timestamp {
        self.env.now()
    }

    pub fn rich_text(&self) -> &'c dyn RichText {
        &*self.env.rich_text
    }

    /// Emits an event right away, even though the transaction may still roll back, for the
    /// side effects Rails performs mid-transaction. What the sink persists for it (a job's row)
    /// is still written in the transaction, and rolls back with it.
    pub fn emit_now(&mut self, event: Event) {
        if self.persist(&event) {
            self.env.sink.emit(event);
        }
    }

    /// Emits an event once the transaction commits (`after_commit`), or right away when
    /// already running after commit. What the sink persists for it (a job's row) is written now,
    /// in the transaction.
    pub fn emit_after_commit(&mut self, event: Event) {
        if !self.persist(&event) {
            return;
        }
        if self.in_transaction {
            self.after_commit.push(AfterCommit::Event(event));
        } else {
            self.env.sink.emit_committed(self, event);
        }
    }

    /// A record's commit callback enqueues once, even if that record was saved
    /// repeatedly. Persist surviving callbacks before COMMIT, so both the job
    /// and its triggering write roll back on queue failure. Explicit job calls
    /// outside record callbacks still use `emit_after_commit` without coalescing.
    pub(crate) fn emit_record_job_once(
        &mut self,
        table: &'static str,
        id: i64,
        job: &impl crate::Job,
    ) {
        self.emit_record_job_once_if(table, id, job, None);
    }

    /// Conditional commit callbacks evaluate the surviving record at the end of
    /// the writer transaction, as Rails evaluates a saved model at commit time.
    pub(crate) fn emit_record_job_once_if(
        &mut self,
        table: &'static str,
        id: i64,
        job: &impl crate::Job,
        condition: Option<fn(&Connection, i64) -> Result<bool>>,
    ) {
        let event = Event::job(job);
        if !self.in_transaction {
            self.emit_after_commit(event);
        } else if !self.after_commit.iter().any(|queued| {
            matches!(queued, AfterCommit::RecordJob { table: previous_table, id: previous_id, event: Some(previous), .. } if *previous_table == table && *previous_id == id && previous == &event)
        }) {
            self.after_commit.push(AfterCommit::RecordJob {
                table,
                id,
                event: Some(event),
                condition,
            });
        }
    }

    /// Active Record runs a record's commit callback once per transaction. Keep
    /// the first registration's position for repeated descriptions of its frame.
    /// This API accepts broadcasts only; job callbacks use the separate job API.
    pub fn emit_broadcast_once(
        &mut self,
        table: &'static str,
        id: i64,
        broadcast: &impl crate::events::Broadcast,
    ) {
        let event = Event::broadcast(broadcast);
        if self.in_transaction && self.after_commit.iter().any(|queued| {
            matches!(queued, AfterCommit::RecordBroadcast { table: previous_table, id: previous_id, event: previous } if *previous_table == table && *previous_id == id && previous == &event)
        }) { return; }
        if !self.persist(&event) {
            return;
        }
        if self.in_transaction {
            self.after_commit
                .push(AfterCommit::RecordBroadcast { table, id, event });
        } else {
            self.env.sink.emit(event);
        }
    }

    /// Active Record registers one commit callback per record in a transaction.
    /// Used for identical, session-independent broadcast descriptions, never jobs.
    pub fn broadcast_after_commit_once<B: crate::Broadcast>(&mut self, broadcast: &B) {
        let event = Event::broadcast(broadcast);
        if !self
            .after_commit
            .iter()
            .any(|pending| matches!(pending, AfterCommit::Event(existing) if existing == &event))
        {
            self.emit_after_commit(event);
        }
    }

    /// Coalesce identical descriptions across this commit and its explicit after-commit
    /// writes. Emit only after those callbacks finish, so rendering sees their final state.
    /// Classic callbacks retain their registration order; only opted-in broadcasts wait.
    pub fn broadcast_after_commit_settled_once<B: crate::Broadcast>(&mut self, broadcast: &B) {
        let event = Event::broadcast(broadcast);
        if self.after_commit.iter().any(|pending| {
            matches!(pending, AfterCommit::SettledBroadcast(existing) if existing == &event)
        }) || !self.persist(&event) {
            return;
        }
        if self.in_transaction {
            self.after_commit.push(AfterCommit::SettledBroadcast(event));
        } else {
            self.collect_settled_broadcast(event);
        }
    }

    fn collect_settled_broadcast(&self, event: Event) {
        let mut broadcasts = self.settled_broadcasts.borrow_mut();
        if !broadcasts.contains(&event) {
            broadcasts.push(event);
        }
    }

    /// An after-commit callback's new transaction joins its caller's settled broadcasts.
    /// Its rows, history and jobs still commit or roll back independently of the caller.
    pub(crate) fn write_after_commit<T>(
        &mut self,
        f: impl FnOnce(&mut Tx<'_>) -> Result<T>,
    ) -> Result<T> {
        debug_assert!(!self.in_transaction);
        run_write_with_settled_broadcasts(self.conn, self.env, self.settled_broadcasts, f)
    }

    /// [`EventSink::persist`]. A failure fails the write (the transaction rolls back when `f`
    /// returns); after commit it's logged, and the event dropped.
    fn persist(&mut self, event: &Event) -> bool {
        match self.env.sink.persist(self, event) {
            Ok(()) => true,
            Err(error) => {
                tracing::error!(%error, ?event, "persisting an event failed");
                if self.in_transaction {
                    self.persist_error.get_or_insert(error);
                }
                false
            }
        }
    }

    /// One record callback per transaction, at its first registration's position, with the
    /// last save's state. Append registrations so savepoint rollback restores earlier hooks.
    /// These hooks run after commit; durable jobs must use the job APIs instead.
    pub fn after_commit_record_latest(
        &mut self,
        key: &'static str,
        id: i64,
        hook: impl FnOnce(&mut Tx<'_>) -> Result<()> + Send + 'static,
    ) {
        if self.in_transaction {
            self.after_commit.push(AfterCommit::RecordHook {
                key,
                id,
                hook: Box::new(hook),
            });
        } else {
            self.after_commit(hook);
        }
    }

    pub(crate) fn has_commit_record(&self, key: &'static str, id: i64) -> bool {
        self.after_commit.iter().any(|pending| matches!(pending, AfterCommit::RecordHook {key: previous, id: previous_id, ..} if *previous == key && *previous_id == id))
    }

    /// Queues database work to run after commit, in its own implicit transaction.
    pub fn after_commit(&mut self, hook: impl FnOnce(&mut Tx<'_>) -> Result<()> + Send + 'static) {
        if self.in_transaction {
            self.after_commit.push(AfterCommit::Hook(Box::new(hook)));
        } else {
            let mut tx = Tx {
                conn: self.conn,
                env: self.env,
                in_transaction: false,
                after_commit: Vec::new(),
                settled_broadcasts: self.settled_broadcasts,
                commit_finalizers: Vec::new(),
                commit_preparations: Vec::new(),
                persist_error: None,
            };
            if let Err(error) = hook(&mut tx) {
                tracing::error!(%error, "after_commit hook failed");
            }
        }
    }

    /// Evaluate a record's durable side effects against the final writer state,
    /// immediately before COMMIT. The last registration wins in the first slot;
    /// savepoint rollback restores earlier registrations. Queue writes and claims
    /// still roll back with the triggering write if preparation or persistence fails.
    pub fn before_commit_record_latest(
        &mut self,
        key: &'static str,
        id: i64,
        hook: impl FnOnce(&mut Tx<'_>) -> Result<()> + Send + 'static,
    ) -> Result<()> {
        if self.in_transaction {
            self.commit_preparations.push(CommitPreparation {
                key,
                id,
                hook: Box::new(hook),
            });
            Ok(())
        } else {
            hook(self)
        }
    }

    fn prepare_commit(&mut self) -> Result<()> {
        while !self.commit_preparations.is_empty() {
            let mut positions = std::collections::HashMap::new();
            let mut pending = Vec::new();
            for registration in std::mem::take(&mut self.commit_preparations) {
                if let Some(&index) = positions.get(&(registration.key, registration.id)) {
                    pending[index] = registration;
                } else {
                    positions.insert((registration.key, registration.id), pending.len());
                    pending.push(registration);
                }
            }
            for preparation in pending {
                (preparation.hook)(self)?;
            }
        }
        Ok(())
    }

    /// Finalizes ownership of resources whose rows committed, before fallible model
    /// callbacks. Captured guards are dropped on rollback, including savepoint rollback.
    /// This is infallible resource bookkeeping, not an Active Record callback or DB write.
    pub fn on_commit_success(&mut self, finalize: impl FnOnce() + Send + 'static) {
        if self.in_transaction {
            self.commit_finalizers.push(Box::new(finalize));
        } else {
            finalize();
        }
    }

    /// Rails registers a record on its first save/destroy, even before it has
    /// any commit callbacks to run. Later callbacks retain that record's slot.
    pub fn register_record(&mut self, table: &'static str, id: i64) {
        if self.in_transaction && !self.after_commit.iter().any(|item| {
            matches!(item, AfterCommit::RecordHooks {table: t, id: i, ..} if *t == table && *i == id)
        }) {
            self.after_commit.push(AfterCommit::RecordHooks {table, id, hooks: Vec::new()});
        }
    }

    pub fn after_commit_record(
        &mut self,
        table: &'static str,
        id: i64,
        hook: impl FnOnce(&mut Tx<'_>) -> Result<()> + Send + 'static,
    ) {
        if !self.in_transaction {
            self.after_commit(hook);
            return;
        }
        self.register_record(table, id);
        if let Some(AfterCommit::RecordHooks {hooks, ..}) = self.after_commit.iter_mut().find(|item| {
            matches!(item, AfterCommit::RecordHooks {table: t, id: i, ..} if *t == table && *i == id)
        }) { hooks.push(Box::new(hook)); }
    }

    pub fn in_transaction(&self) -> bool {
        self.in_transaction
    }

    /// A model operation whose validation error may be rescued by its caller. Discard
    /// deferred callbacks together with rolled-back rows; queue persistence failures still
    /// fail the enclosing transaction through `persist_error`.
    pub fn savepoint<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let callbacks = self.after_commit.len();
        let finalizers = self.commit_finalizers.len();
        let preparations = self.commit_preparations.len();
        let record_hooks: Vec<_> = self
            .after_commit
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if let AfterCommit::RecordHooks { hooks, .. } = item {
                    Some((i, hooks.len()))
                } else {
                    None
                }
            })
            .collect();
        self.conn.execute_batch("SAVEPOINT model_operation")?;
        match f(self) {
            Ok(value) => {
                self.conn
                    .execute_batch("RELEASE SAVEPOINT model_operation")?;
                Ok(value)
            }
            Err(error) => {
                self.after_commit.truncate(callbacks);
                self.commit_finalizers.truncate(finalizers);
                self.commit_preparations.truncate(preparations);
                for (index, length) in record_hooks {
                    if let AfterCommit::RecordHooks { hooks, .. } = &mut self.after_commit[index] {
                        hooks.truncate(length);
                    }
                }
                self.conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT model_operation; RELEASE SAVEPOINT model_operation",
                )?;
                Err(error)
            }
        }
    }
}

/// Runs `f` in `BEGIN IMMEDIATE`, commits, then runs the after-commit queue. An error from
/// `f` rolls back and discards the queue. An error from an after-commit hook is returned
/// after finalizing committed resources and waking any remaining committed jobs,
/// discarding later model callbacks
/// (Rails raises it from the save that committed; durable enqueueing stays atomic).
/// Opted-in settled broadcasts of committed writes still emit after callbacks stop.
pub fn run_write<T>(
    conn: &Connection,
    env: &Env,
    f: impl FnOnce(&mut Tx<'_>) -> Result<T>,
) -> Result<T> {
    let settled_broadcasts = RefCell::new(Vec::new());
    let result = run_write_with_settled_broadcasts(conn, env, &settled_broadcasts, f);
    let mut after = Tx {
        conn,
        env,
        in_transaction: false,
        after_commit: Vec::new(),
        settled_broadcasts: &settled_broadcasts,
        commit_finalizers: Vec::new(),
        commit_preparations: Vec::new(),
        persist_error: None,
    };
    // Keep committed facts visible even when a later callback failed. Rolled-back writes
    // never enter this collection. Release its borrow before a sink runs more callbacks.
    loop {
        let broadcasts = settled_broadcasts.take();
        if broadcasts.is_empty() {
            break;
        }
        for event in broadcasts {
            env.sink.emit_committed(&mut after, event);
        }
    }
    result
}

fn run_write_with_settled_broadcasts<T>(
    conn: &Connection,
    env: &Env,
    settled_broadcasts: &RefCell<Vec<Event>>,
    f: impl FnOnce(&mut Tx<'_>) -> Result<T>,
) -> Result<T> {
    conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;
    let mut tx = Tx {
        conn,
        env,
        in_transaction: true,
        after_commit: Vec::new(),
        settled_broadcasts,
        commit_finalizers: Vec::new(),
        commit_preparations: Vec::new(),
        persist_error: None,
    };
    let value = match f(&mut tx)
        .and_then(|value| {
            tx.prepare_commit()?;
            Ok(value)
        })
        .and_then(|value| match tx.persist_error.take() {
            Some(error) => Err(error),
            None => Ok(value),
        }) {
        Ok(value) => value,
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK TRANSACTION");
            return Err(error);
        }
    };
    let mut queue = Vec::new();
    for pending in std::mem::take(&mut tx.after_commit) {
        if let AfterCommit::RecordHook { key, id, hook } = pending {
            if let Some(AfterCommit::RecordHook { hook: previous, .. }) = queue.iter_mut().find(|pending| matches!(pending, AfterCommit::RecordHook {key: previous, id: previous_id, ..} if *previous == key && *previous_id == id)) {
                *previous = hook;
            } else {
                queue.push(AfterCommit::RecordHook { key, id, hook });
            }
        } else {
            queue.push(pending);
        }
    }
    let persist_callbacks = (|| -> Result<()> {
        for item in &mut queue {
            if let AfterCommit::RecordJob {
                table,
                id,
                event,
                condition,
            } = item
            {
                let survives = conn.query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?)"),
                    [*id],
                    |r| r.get::<_, bool>(0),
                )?;
                let should_enqueue = survives
                    && match condition {
                        Some(test) => test(conn, *id)?,
                        None => true,
                    };
                if should_enqueue {
                    if let Some(event) = event {
                        env.sink.persist(&tx, event)?;
                    }
                } else {
                    *event = None;
                }
            }
        }
        Ok(())
    })();
    if let Err(error) = persist_callbacks {
        let _ = conn.execute_batch("ROLLBACK TRANSACTION");
        return Err(error);
    }
    if let Err(error) = conn.execute_batch("COMMIT TRANSACTION") {
        let _ = conn.execute_batch("ROLLBACK TRANSACTION");
        return Err(error.into());
    }

    // These descriptions are now committed. They wait for the entire callback tree,
    // including tag assignment's separate commit, without reordering classic events.
    queue.retain(|item| match item {
        AfterCommit::SettledBroadcast(event) => {
            tx.collect_settled_broadcast(event.clone());
            false
        }
        _ => true,
    });

    for finalize in std::mem::take(&mut tx.commit_finalizers) {
        finalize();
    }

    let mut after = Tx {
        conn,
        env,
        in_transaction: false,
        after_commit: Vec::new(),
        settled_broadcasts,
        commit_finalizers: Vec::new(),
        commit_preparations: Vec::new(),
        persist_error: None,
    };
    let mut callbacks = queue.into_iter();
    while let Some(item) = callbacks.next() {
        let callback_error = match item {
            AfterCommit::RecordJob { event, .. } => {
                if let Some(event) = event {
                    env.sink.emit_committed(&mut after, event);
                }
                None
            }
            AfterCommit::Event(event) => {
                env.sink.emit_committed(&mut after, event);
                None
            }
            AfterCommit::RecordBroadcast { table, id, event } => {
                // A later destroy suppresses the record's earlier update callback.
                match conn.query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?)"),
                    [id],
                    |r| r.get::<_, bool>(0),
                ) {
                    Ok(true) => env.sink.emit_committed(&mut after, event),
                    Ok(false) => (),
                    Err(error) => tracing::warn!(%error, table, id, "broadcast callback failed"),
                }
                None
            }
            AfterCommit::RecordHook { hook, .. } => hook(&mut after).err(),
            AfterCommit::RecordHooks { hooks, .. } => hooks
                .into_iter()
                .try_for_each(|hook| hook(&mut after))
                .err(),
            AfterCommit::Hook(hook) => hook(&mut after).err(),
            AfterCommit::SettledBroadcast(_) => unreachable!("collected after COMMIT"),
        };
        if let Some(error) = callback_error {
            for pending in callbacks {
                pending.wake_committed_job(env);
            }
            return Err(error);
        }
    }
    Ok(value)
}

#[derive(Debug, Clone)]
pub struct Config {
    pub path: PathBuf,
    pub readers: usize,
    /// Bound on queued writes; senders wait when it's full.
    pub write_queue: usize,
    /// Load the schema into an empty database (`db:prepare`).
    pub prepare: bool,
    /// `ar_internal_metadata.environment` when preparing.
    pub environment: String,
}

impl Config {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            readers: 8,
            write_queue: 256,
            prepare: true,
            environment: "production".into(),
        }
    }
}

type Job = Box<dyn WriterJob>;

// Keep the reply alive outside the operation's unwind boundary: even a panic
// must finish rollback and generation bookkeeping before notifying the caller.
trait WriterJob: Send {
    fn run(&mut self, conn: &Connection, env: &Env);
    fn complete(self: Box<Self>);
}

struct WriteJob<T, F> {
    operation: Option<F>,
    result: Option<Result<T>>,
    reply: oneshot::Sender<Result<T>>,
}

impl<T, F> WriterJob for WriteJob<T, F>
where
    T: Send,
    F: FnOnce(&Connection, &Env) -> Result<T> + Send,
{
    fn run(&mut self, conn: &Connection, env: &Env) {
        let operation = self.operation.take().expect("writer runs each job once");
        self.result = Some(operation(conn, env));
    }

    fn complete(self: Box<Self>) {
        if let Some(result) = self.result {
            let _ = self.reply.send(result);
        }
        // If run panicked, dropping the retained sender reports WriterGone.
    }
}

#[cfg(feature = "test-support")]
type QueryLog = Arc<Mutex<Vec<String>>>;

/// The database handle. Cheap to clone.
#[derive(Clone)]
pub struct Database {
    writer_generation: Arc<AtomicU64>,
    writer: mpsc::Sender<Job>,
    readers: Arc<ReaderPool>,
    #[cfg(feature = "test-support")]
    writer_query_log: Arc<Mutex<Option<QueryLog>>>,
    env: Env,
    path: PathBuf,
}

impl Database {
    pub fn open(config: Config, env: Env) -> Result<Self> {
        let mut conn = open_connection(&config.path, false)?;
        if config.prepare {
            schema::prepare(&mut conn, &config.environment, &*env.clock)?;
        }
        let mut checkpoints = Checkpoints::spawn(&config.path)?;
        // In place of the auto-checkpoint, which is itself a WAL hook (`sqlite3_wal_autocheckpoint`).
        conn.wal_hook(Some(note_wal_size));

        let (sender, mut receiver) = mpsc::channel::<Job>(config.write_queue.max(1));
        let writer_env = env.clone();
        #[cfg(feature = "test-support")]
        let writer_query_log = Arc::new(Mutex::new(None));
        #[cfg(feature = "test-support")]
        let query_log = writer_query_log.clone();
        let writer_generation = Arc::new(AtomicU64::new(0));
        let generation = writer_generation.clone();
        std::thread::Builder::new()
            .name("campfire-db-writer".into())
            .spawn(move || {
                while let Some(mut job) = receiver.blocking_recv() {
                    #[cfg(feature = "test-support")]
                    let _trace = query_log
                        .lock()
                        .unwrap()
                        .clone()
                        .map(|log| QueryTrace::enter(&conn, log));
                    generation.fetch_add(1, Ordering::SeqCst);
                    // A panicking write must not take the writer down with it.
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        job.run(&conn, &writer_env)
                    }));
                    if outcome.is_err() && !conn.is_autocommit() {
                        let _ = conn.execute_batch("ROLLBACK TRANSACTION");
                    }
                    generation.fetch_add(1, Ordering::SeqCst);
                    // Completion can drop a cancelled caller's result, whose Drop
                    // may panic. It must not kill the writer or strand the reply.
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        job.complete();
                    }));
                    match WAL_PAGES.replace(0) {
                        0 => {}
                        pages if pages >= WAL_LIMIT_PAGES => restart_wal(&conn, &checkpoints),
                        pages => checkpoints.wal_grew_to(pages),
                    }
                }
            })
            .map_err(|e| Error::Other(e.to_string()))?;

        let readers = (0..config.readers.max(1))
            .map(|_| open_connection(&config.path, true))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            writer: sender,
            writer_generation,
            readers: Arc::new(ReaderPool::new(readers)),
            #[cfg(feature = "test-support")]
            writer_query_log,
            env,
            path: config.path,
        })
    }

    pub fn env(&self) -> &Env {
        &self.env
    }

    /// Changes before and after every writer job, including its after-commit callbacks.
    /// Odd values mean work is in progress. Request-local read snapshots may be reused
    /// only while this value stays equal and even; this is not an external DB version.
    /// Each job restores an even value before notifying its caller, including on panic.
    /// A subsequent queued job can make it odd again before that caller resumes.
    pub fn write_generation(&self) -> u64 {
        self.writer_generation.load(Ordering::SeqCst)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Runs `f` as one immediate transaction on the writer thread.
    /// Work queued behind the SQLite writer, excluding its currently running transaction.
    /// Allows runtime tests and diagnostics to observe a real blocked critical section.
    pub fn queued_writes(&self) -> usize {
        self.writer.max_capacity() - self.writer.capacity()
    }

    pub async fn write<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Tx<'_>) -> Result<T> + Send + 'static,
    {
        self.write_scoped(|| (), f).await
    }

    /// Holds a caller-owned runtime guard across the write AND its ordered after-commit
    /// callbacks. Models remain unaware of request/rendering context.
    pub async fn write_scoped<T, F, G>(
        &self,
        scope: impl FnOnce() -> G + Send + 'static,
        f: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Tx<'_>) -> Result<T> + Send + 'static,
        G: 'static,
    {
        let (reply, response) = oneshot::channel();
        self.writer
            .send(Box::new(WriteJob {
                operation: Some(move |conn: &Connection, env: &Env| {
                    let _scope = scope();
                    run_write(conn, env, f)
                }),
                result: None,
                reply,
            }))
            .await
            .map_err(|_| Error::WriterGone)?;
        response.await.map_err(|_| Error::WriterGone)?
    }

    /// [`Database::write`] for synchronous callers (not from inside an async task).
    pub fn write_blocking<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Tx<'_>) -> Result<T> + Send + 'static,
    {
        let (reply, response) = oneshot::channel();
        self.writer
            .blocking_send(Box::new(WriteJob {
                operation: Some(move |conn: &Connection, env: &Env| run_write(conn, env, f)),
                result: None,
                reply,
            }))
            .map_err(|_| Error::WriterGone)?;
        response.blocking_recv().map_err(|_| Error::WriterGone)?
    }

    /// Runs `f` on a reader connection, on the blocking pool.
    pub async fn read<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> Result<T> + Send + 'static,
    {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || readers.with(f))
            .await
            .map_err(|e| Error::Other(e.to_string()))?
    }

    /// Record actual reader SQL across this database's blocking workers. Test-only;
    /// each database owns its capture, so parallel app tests cannot mix queries.
    #[cfg(feature = "test-support")]
    pub fn capture_read_queries(&self) -> Arc<Mutex<Vec<String>>> {
        let log = Arc::new(Mutex::new(Vec::new()));
        *self.readers.query_log.lock().unwrap() = Some(log.clone());
        log
    }

    #[cfg(feature = "test-support")]
    pub fn stop_capturing_read_queries(&self) {
        *self.readers.query_log.lock().unwrap() = None;
    }

    /// Lower SQLite's real bind-variable limit on idle reader connections for boundary tests.
    /// Tests call this only on an otherwise idle app with its job runner stopped.
    #[cfg(feature = "test-support")]
    pub fn set_reader_parameter_limit(&self, limit: i32) -> Vec<i32> {
        let readers = self.readers.idle.lock().unwrap();
        assert!(!readers.is_empty(), "parameter-limit test needs idle readers");
        readers.iter().map(|conn| conn.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_VARIABLE_NUMBER, limit).expect("valid SQLite limit")).collect()
    }

    /// Trace the pooled readers and SELECTs issued from writer transactions.
    #[cfg(feature = "test-support")]
    pub fn capture_queries(&self) -> Arc<Mutex<Vec<String>>> {
        let log = self.capture_read_queries();
        *self.writer_query_log.lock().unwrap() = Some(log.clone());
        log
    }

    #[cfg(feature = "test-support")]
    pub fn stop_capturing_queries(&self) {
        self.stop_capturing_read_queries();
        *self.writer_query_log.lock().unwrap() = None;
    }

    /// [`Database::read`] for synchronous callers.
    pub fn read_blocking<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        self.readers.with(f)
    }
}

/// Prepared statements each connection keeps.
const STATEMENT_CACHE_CAPACITY: usize = 256;

/// SQLite's default `wal_autocheckpoint`, which Rails keeps: a checkpoint per 1,000 WAL pages.
const AUTOCHECKPOINT_PAGES: i32 = 1000;

/// The WAL size at which the writer checkpoints and restarts the WAL itself, because writes never
/// paused long enough for a background checkpoint to catch up. Below `journal_size_limit`.
const WAL_LIMIT_PAGES: i32 = 10_000;

thread_local! {
    /// The WAL's size in pages after the writer thread's latest commit.
    static WAL_PAGES: Cell<i32> = const { Cell::new(0) };
}

/// The writer connection's WAL hook: runs on the writer thread after each commit.
fn note_wal_size(_: &rusqlite::hooks::Wal, pages: std::os::raw::c_int) -> rusqlite::Result<()> {
    WAL_PAGES.set(pages);
    Ok(())
}

/// The writer's side of the checkpointer thread, which runs a PASSIVE checkpoint on its own
/// connection each time it's woken. It stops with the writer (the sender's owner).
struct Checkpoints {
    wake: std::sync::mpsc::SyncSender<()>,
    /// Held while the checkpointer runs, so the writer's RESTART waits for it rather than being
    /// refused (SQLite runs one checkpoint at a time) and letting the WAL grow past its limit.
    running: Arc<Mutex<()>>,
    /// The WAL's size in pages when the checkpointer was last woken.
    woken_at: i32,
}

impl Checkpoints {
    fn spawn(path: &Path) -> Result<Self> {
        let conn = open_connection(path, false)?;
        let (wake, woken) = std::sync::mpsc::sync_channel::<()>(1);
        let running = Arc::new(Mutex::new(()));
        let checkpointer_running = running.clone();
        std::thread::Builder::new()
            .name("campfire-db-checkpointer".into())
            .spawn(move || {
                while woken.recv().is_ok() {
                    let _running = checkpointer_running
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    checkpoint(&conn, "PASSIVE");
                }
            })
            .map_err(|e| Error::Other(e.to_string()))?;
        Ok(Self {
            wake,
            running,
            woken_at: 0,
        })
    }

    /// Wakes the checkpointer for every [`AUTOCHECKPOINT_PAGES`] the WAL grows.
    fn wal_grew_to(&mut self, pages: i32) {
        if pages < self.woken_at {
            self.woken_at = 0; // the WAL restarted
        }
        // While a checkpoint is still due (the channel is full), the next commit tries again.
        if pages - self.woken_at >= AUTOCHECKPOINT_PAGES && self.wake.try_send(()).is_ok() {
            self.woken_at = pages;
        }
    }
}

/// A RESTART checkpoint on the writer connection, between writes: it copies what the
/// checkpointer hasn't, and waits for readers so that the next write restarts the WAL. It waits
/// for a running PASSIVE checkpoint first, which would otherwise make SQLite refuse it.
fn restart_wal(conn: &Connection, checkpoints: &Checkpoints) {
    let _running = checkpoints
        .running
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    checkpoint(conn, "RESTART");
}

/// `PRAGMA wal_checkpoint`, which reports a checkpoint it couldn't finish (another checkpoint
/// running, or readers still on old frames past the busy timeout) in its `busy` column, not as an
/// error.
fn checkpoint(conn: &Connection, mode: &str) {
    match conn.query_row(&format!("PRAGMA wal_checkpoint({mode})"), [], |row| {
        row.get::<_, i64>(0)
    }) {
        Ok(0) => {}
        Ok(_) => tracing::warn!(mode, "WAL checkpoint couldn't finish"),
        Err(error) => tracing::warn!(%error, mode, "WAL checkpoint failed"),
    }
}

fn open_connection(path: &Path, reader: bool) -> Result<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
        | OpenFlags::SQLITE_OPEN_URI;
    let conn = Connection::open_with_flags(path, flags)?;
    // rusqlite's default of 16 is fewer statements than a page like the room show runs.
    conn.set_prepared_statement_cache_capacity(STATEMENT_CACHE_CAPACITY);
    schema::configure_connection(&conn)?;
    if reader {
        conn.pragma_update(None, "query_only", true)?;
    }
    Ok(conn)
}

struct ReaderPool {
    #[cfg(feature = "test-support")]
    query_log: Mutex<Option<QueryLog>>,
    idle: Mutex<Vec<Connection>>,
    available: Condvar,
}

impl ReaderPool {
    fn new(connections: Vec<Connection>) -> Self {
        Self {
            idle: Mutex::new(connections),
            available: Condvar::new(),
            #[cfg(feature = "test-support")]
            query_log: Mutex::new(None),
        }
    }

    fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = {
            let mut idle = self
                .idle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            loop {
                if let Some(conn) = idle.pop() {
                    break conn;
                }
                idle = self
                    .available
                    .wait(idle)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
        };
        let checkout = Checkout {
            pool: self,
            conn: Some(conn),
        };
        let conn = checkout.conn.as_ref().expect("checked out");
        #[cfg(feature = "test-support")]
        let _trace = self
            .query_log
            .lock()
            .unwrap()
            .clone()
            .map(|log| QueryTrace::enter(conn, log));
        f(conn)
    }
}

#[cfg(feature = "test-support")]
thread_local! {
    static READ_QUERIES: std::cell::RefCell<Option<Arc<Mutex<Vec<String>>>>> = const { std::cell::RefCell::new(None) };
}

#[cfg(feature = "test-support")]
struct QueryTrace<'a>(&'a Connection, Option<QueryLog>);

#[cfg(feature = "test-support")]
impl<'a> QueryTrace<'a> {
    fn enter(conn: &'a Connection, log: Arc<Mutex<Vec<String>>>) -> Self {
        fn record(event: rusqlite::trace::TraceEvent<'_>) {
            if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event {
                READ_QUERIES.with(|log| {
                    if let Some(log) = log.borrow().as_ref() {
                        log.lock().unwrap().push(sql.into());
                    }
                });
            }
        }
        let previous = READ_QUERIES.with(|slot| slot.replace(Some(log)));
        conn.trace_v2(
            rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
            Some(record),
        );
        Self(conn, previous)
    }
}

#[cfg(feature = "test-support")]
impl Drop for QueryTrace<'_> {
    fn drop(&mut self) {
        self.0
            .trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
        READ_QUERIES.with(|slot| {
            slot.replace(self.1.take());
        });
    }
}

/// A reader connection out of the pool, returned when dropped: also when the read panics, which
/// would otherwise lose the connection for good (and after as many panics as there are readers,
/// hang every read).
struct Checkout<'a> {
    pool: &'a ReaderPool,
    conn: Option<Connection>,
}

impl Drop for Checkout<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.pool
                .idle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(conn);
            self.pool.available.notify_one();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct SettledSnapshotSink {
        events: Mutex<Vec<(Event, Option<i64>)>>,
    }

    impl EventSink for SettledSnapshotSink {
        fn emit(&self, event: Event) {
            self.events.lock().unwrap().push((event, None));
        }

        fn emit_committed(&self, after: &mut Tx<'_>, event: Event) {
            let value = match &event {
                Event::Broadcast(request) => request
                    .decode::<crate::models::channel_thread::ThreadWorkChange>()
                    .map(|change| {
                        let change = change.unwrap();
                        after
                            .conn()
                            .query_row(
                                "SELECT value FROM state WHERE id=?",
                                [change.thread_id],
                                |r| r.get::<_, i64>(0),
                            )
                            .unwrap()
                    }),
                _ => None,
            };
            self.events.lock().unwrap().push((event, value));
        }
    }

    #[test]
    fn settled_broadcasts_coalesce_nested_commits_after_all_hooks() {
        use crate::models::channel_thread::ThreadWorkChange;

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE state(id INTEGER PRIMARY KEY,value INTEGER); INSERT INTO state VALUES(1,0),(2,0)").unwrap();
        let sink = Arc::new(SettledSnapshotSink::default());
        let env = Env {
            sink: sink.clone(),
            ..Env::default()
        };
        run_write(&conn, &env, |tx| {
            tx.conn().execute("UPDATE state SET value=1 WHERE id=1", [])?;
            ThreadWorkChange::emit(tx, 1);
            tx.emit_after_commit(Event::PurgeBlob { blob_id: 11 });
            tx.after_commit_record_latest("tag_assignment", 1, |after| {
                after.write_after_commit(|tx| {
                    tx.conn().execute("UPDATE state SET value=2", [])?;
                    ThreadWorkChange::emit(tx, 1);
                    ThreadWorkChange::emit(tx, 2);
                    ThreadWorkChange::emit(tx, 2);
                    tx.emit_after_commit(Event::PurgeBlob { blob_id: 12 });
                    tx.after_commit(|after| {
                        after.write_after_commit(|tx| {
                            tx.conn().execute("UPDATE state SET value=3 WHERE id=1", [])?;
                            ThreadWorkChange::emit(tx, 1);
                            Ok(())
                        })?;
                        // Opted-in broadcasts emitted directly after commit share the scope.
                        ThreadWorkChange::emit(after, 2);
                        Ok(())
                    });
                    Ok(())
                })
            });
            tx.emit_after_commit(Event::PurgeBlob { blob_id: 13 });
            Ok(())
        })
        .unwrap();
        assert_eq!(
            *sink.events.lock().unwrap(),
            [
                (Event::PurgeBlob { blob_id: 11 }, None),
                (Event::PurgeBlob { blob_id: 12 }, None),
                (Event::PurgeBlob { blob_id: 13 }, None),
                (Event::broadcast(&ThreadWorkChange { thread_id: 1 }), Some(3)),
                (Event::broadcast(&ThreadWorkChange { thread_id: 2 }), Some(2)),
            ]
        );
    }

    #[test]
    fn settled_broadcasts_follow_savepoint_and_transaction_rollbacks() {
        use crate::models::channel_thread::ThreadWorkChange;

        let conn = Connection::open_in_memory().unwrap();
        let sink = crate::events::RecordingSink::new();
        let env = Env {
            sink: Arc::new(sink.clone()),
            ..Env::default()
        };
        run_write(&conn, &env, |tx| {
            ThreadWorkChange::emit(tx, 1);
            let rolled_back = tx.savepoint(|tx| -> Result<()> {
                ThreadWorkChange::emit(tx, 2);
                Err(Error::Other("savepoint rollback".into()))
            });
            assert!(rolled_back.is_err());
            tx.after_commit(|after| {
                let rolled_back = after.write_after_commit(|tx| -> Result<()> {
                    ThreadWorkChange::emit(tx, 3);
                    Err(Error::Other("nested transaction rollback".into()))
                });
                assert!(rolled_back.is_err());
                Ok(())
            });
            Ok(())
        })
        .unwrap();
        assert_eq!(
            sink.take(),
            [Event::broadcast(&ThreadWorkChange { thread_id: 1 })]
        );
        let rolled_back = run_write(&conn, &env, |tx| -> Result<()> {
            ThreadWorkChange::emit(tx, 4);
            Err(Error::Other("outer transaction rollback".into()))
        });
        assert!(rolled_back.is_err());
        assert!(sink.events().is_empty());
    }

    #[test]
    fn settled_broadcasts_survive_later_callback_errors() {
        use crate::models::channel_thread::ThreadWorkChange;

        let conn = Connection::open_in_memory().unwrap();
        let sink = crate::events::RecordingSink::new();
        let env = Env {
            sink: Arc::new(sink.clone()),
            ..Env::default()
        };
        let result = run_write(&conn, &env, |tx| {
            ThreadWorkChange::emit(tx, 1);
            tx.after_commit(|after| {
                let result = after.write_after_commit(|tx| {
                    ThreadWorkChange::emit(tx, 1);
                    ThreadWorkChange::emit(tx, 2);
                    tx.after_commit(|_| Err(Error::Other("nested callback failure".into())));
                    Ok(())
                });
                assert!(result.is_err());
                Err(Error::Other("outer callback failure".into()))
            });
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(
            sink.events(),
            [
                Event::broadcast(&ThreadWorkChange { thread_id: 1 }),
                Event::broadcast(&ThreadWorkChange { thread_id: 2 }),
            ]
        );
    }

    struct CompletionWake {
        generation: Arc<AtomicU64>,
        observed: Mutex<Option<std::sync::mpsc::Sender<u64>>>,
    }

    impl std::task::Wake for CompletionWake {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            if let Some(reply) = self.observed.lock().unwrap().take() {
                reply.send(self.generation.load(Ordering::SeqCst)).unwrap();
            }
        }
    }

    // Observe the generation synchronously inside oneshot's notification, before
    // the writer can advance it again. A scheduler-dependent check after await
    // would miss the race, or observe an unrelated subsequent write.
    fn observe_completion<F, G>(
        db: &Database,
        scope: impl FnOnce() -> G + Send + 'static,
        f: F,
    ) -> (Result<()>, u64)
    where
        F: FnOnce(&mut Tx<'_>) -> Result<()> + Send + 'static,
        G: 'static,
    {
        let (release, blocked) = std::sync::mpsc::channel();
        let (reply, observed) = std::sync::mpsc::channel();
        let waker = std::task::Waker::from(Arc::new(CompletionWake {
            generation: db.writer_generation.clone(),
            observed: Mutex::new(Some(reply)),
        }));
        let mut context = std::task::Context::from_waker(&waker);
        let mut write = std::pin::pin!(db.write_scoped(
            move || {
                blocked.recv().unwrap();
                scope()
            },
            f
        ));
        assert!(write.as_mut().poll(&mut context).is_pending());
        release.send(()).unwrap();
        let generation = observed
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("accepted write must notify its caller, even after a panic");
        let std::task::Poll::Ready(result) = write.as_mut().poll(&mut context) else {
            panic!("reply must be ready when its waker runs");
        };
        (result, generation)
    }

    #[test]
    fn write_completion_notifies_only_after_generation_is_even() {
        struct Scope {
            generation: Arc<AtomicU64>,
            panic_on_drop: bool,
            // Guards may be !Send: they are constructed and dropped on the writer.
            _local: std::rc::Rc<()>,
        }
        impl Drop for Scope {
            fn drop(&mut self) {
                assert_eq!(self.generation.load(Ordering::SeqCst) % 2, 1);
                assert!(!self.panic_on_drop, "injected scope cleanup panic");
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(
            Config::new(dir.path().join("completion.sqlite3")),
            Env::default(),
        )
        .unwrap();
        db.write_blocking(|tx| Ok(tx.conn().execute_batch("CREATE TABLE probe(id INTEGER)")?))
            .unwrap();
        for case in 0..7 {
            let before = db.write_generation();
            let generation = db.writer_generation.clone();
            let snapshot = db.clone();
            let (result, notified) = observe_completion(
                &db,
                move || {
                    assert_ne!(case, 6, "injected scope construction panic");
                    Scope {
                        generation,
                        panic_on_drop: case == 5,
                        _local: std::rc::Rc::new(()),
                    }
                },
                move |tx| {
                    assert_eq!(snapshot.write_generation(), before + 1);
                    tx.conn().execute("INSERT INTO probe VALUES(?)", [case])?;
                    match case {
                        1 => return Err(Error::Other("injected transaction error".into())),
                        2 => panic!("injected transaction panic"),
                        _ => {}
                    }
                    tx.after_commit(move |tx| {
                        assert_eq!(snapshot.write_generation(), before + 1);
                        assert!(tx.conn().is_autocommit());
                        match case {
                            3 => Err(Error::Other("injected callback error".into())),
                            4 => panic!("injected callback panic"),
                            _ => Ok(()),
                        }
                    });
                    Ok(())
                },
            );
            assert!(notified.is_multiple_of(2), "notification must observe even generation");
            assert_eq!(notified, before + 2, "completion case {case}");
            match case {
                0 => result.unwrap(),
                1 | 3 => assert!(matches!(result, Err(Error::Other(_)))),
                _ => assert!(matches!(result, Err(Error::WriterGone))),
            }
            let persisted = db
                .read_blocking(move |conn| {
                    Ok(
                        conn.query_row("SELECT count(*) FROM probe WHERE id=?", [case], |r| {
                            r.get::<_, i64>(0)
                        })?,
                    )
                })
                .unwrap();
            assert_eq!(persisted, i64::from(!matches!(case, 1 | 2 | 6)));
        }
        // Scope construction also panicked; the writer still accepts work.
        db.write_blocking(|tx| {
            assert!(!tx.conn().is_autocommit());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn cancelled_write_drops_its_result_after_even_and_survives_a_panicking_drop() {
        struct Value {
            generation: Arc<AtomicU64>,
            observed: std::sync::mpsc::Sender<u64>,
            panic_on_drop: bool,
        }
        impl Drop for Value {
            fn drop(&mut self) {
                self.observed
                    .send(self.generation.load(Ordering::SeqCst))
                    .unwrap();
                assert!(!self.panic_on_drop, "injected cancelled result panic");
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(
            Config::new(dir.path().join("cancelled.sqlite3")),
            Env::default(),
        )
        .unwrap();
        db.write_blocking(|tx| Ok(tx.conn().execute_batch("CREATE TABLE probe(id INTEGER)")?))
            .unwrap();
        for panic_on_drop in [false, true] {
            let before = db.write_generation();
            let generation = db.writer_generation.clone();
            let (release, blocked) = std::sync::mpsc::channel();
            let (observed, dropped) = std::sync::mpsc::channel();
            let mut write = Box::pin(db.write(move |tx| {
                blocked.recv().unwrap();
                tx.conn().execute("INSERT INTO probe VALUES(1)", [])?;
                Ok(Value {
                    generation,
                    observed,
                    panic_on_drop,
                })
            }));
            let mut context = std::task::Context::from_waker(std::task::Waker::noop());
            assert!(write.as_mut().poll(&mut context).is_pending());
            drop(write);
            release.send(()).unwrap();
            let disposed = dropped.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
            assert!(disposed.is_multiple_of(2), "cancelled result must be disposed after even");
            assert_eq!(disposed, before + 2);
            db.write_blocking(|tx| {
                assert!(!tx.conn().is_autocommit());
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(
            db.read_blocking(|conn| Ok(
                conn.query_row("SELECT count(*) FROM probe", [], |r| r.get::<_, i64>(0))?
            ))
            .unwrap(),
            2
        );
    }

    #[cfg(feature = "test-support")]
    #[tokio::test]
    async fn query_capture_includes_writer_and_reader_statements_and_can_stop() {
        let dir = tempfile::tempdir().unwrap();
        let db =
            Database::open(Config::new(dir.path().join("test.sqlite3")), Env::default()).unwrap();
        let log = db.capture_queries();
        db.write(|tx| {
            Ok(tx
                .conn()
                .query_row("SELECT 11", [], |row| row.get::<_, i64>(0))?)
        })
        .await
        .unwrap();
        db.read(|conn| Ok(conn.query_row("SELECT 22", [], |row| row.get::<_, i64>(0))?))
            .await
            .unwrap();
        db.stop_capturing_queries();
        let before = log.lock().unwrap().clone();
        assert!(before.iter().any(|sql| sql == "SELECT 11"));
        assert!(before.iter().any(|sql| sql == "SELECT 22"));
        db.write(|tx| {
            Ok(tx
                .conn()
                .query_row("SELECT 33", [], |row| row.get::<_, i64>(0))?)
        })
        .await
        .unwrap();
        db.read(|conn| Ok(conn.query_row("SELECT 44", [], |row| row.get::<_, i64>(0))?))
            .await
            .unwrap();
        assert_eq!(*log.lock().unwrap(), before);
    }

    #[test]
    fn before_commit_uses_final_state_and_restores_latest_hook_on_savepoint_rollback() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE state(value INTEGER); INSERT INTO state VALUES(1); CREATE TABLE observations(label TEXT,value INTEGER)").unwrap();
        let env = Env::default();
        run_write(&conn, &env, |tx| {
            tx.before_commit_record_latest("record", 1, |tx| {
                tx.conn()
                    .execute("INSERT INTO observations SELECT 'old',value FROM state", [])?;
                Ok(())
            })?;
            tx.before_commit_record_latest("record", 1, |tx| {
                tx.conn().execute(
                    "INSERT INTO observations SELECT 'latest',value FROM state",
                    [],
                )?;
                Ok(())
            })?;
            let error: Result<()> = tx.savepoint(|tx| {
                tx.before_commit_record_latest("record", 1, |tx| {
                    tx.conn().execute(
                        "INSERT INTO observations SELECT 'rolled-back',value FROM state",
                        [],
                    )?;
                    Ok(())
                })?;
                Err(Error::Other("savepoint rollback".into()))
            });
            assert!(error.is_err());
            tx.conn().execute("UPDATE state SET value=2", [])?;
            Ok(())
        })
        .unwrap();
        let observations = conn
            .prepare("SELECT label,value FROM observations")
            .unwrap()
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(observations, vec![("latest".to_string(), 2)]);
    }

    #[test]
    fn before_commit_failures_roll_back_every_write_and_discard_callbacks() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE state(value INTEGER); INSERT INTO state VALUES(1)")
            .unwrap();
        let sink = crate::events::RecordingSink::new();
        let env = Env {
            sink: Arc::new(sink.clone()),
            ..Env::default()
        };
        let result = run_write(&conn, &env, |tx| {
            tx.conn().execute("UPDATE state SET value=2", [])?;
            tx.before_commit_record_latest("record", 1, |tx| {
                tx.conn().execute("UPDATE state SET value=3", [])?;
                tx.emit_after_commit(Event::PurgeBlob { blob_id: 1 });
                Err(Error::Other("preparation failure".into()))
            })?;
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(
            conn.query_row("SELECT value FROM state", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(sink.events().is_empty());
    }

    #[test]
    fn commit_finalizers_precede_fallible_model_callbacks() {
        let conn = Connection::open_in_memory().unwrap();
        let env = Env::default();
        let trace = Arc::new(Mutex::new(Vec::new()));
        let observations = trace.clone();
        let result = run_write(&conn, &env, move |tx| {
            let first = observations.clone();
            tx.after_commit(move |_| {
                first.lock().unwrap().push("callback");
                Err(Error::Other("injected callback failure".into()))
            });
            tx.on_commit_success(move || observations.lock().unwrap().push("finalized"));
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(*trace.lock().unwrap(), ["finalized", "callback"]);
    }

    #[test]
    fn commit_finalizers_follow_savepoint_rollback_boundaries() {
        let conn = Connection::open_in_memory().unwrap();
        let env = Env::default();
        let trace = Arc::new(Mutex::new(Vec::new()));
        let observations = trace.clone();
        run_write(&conn, &env, move |tx| {
            let outer = observations.clone();
            tx.on_commit_success(move || outer.lock().unwrap().push("outer"));
            let nested = tx.savepoint(move |tx| -> Result<()> {
                tx.on_commit_success(move || observations.lock().unwrap().push("rolled-back"));
                Err(Error::Other("injected savepoint rollback".into()))
            });
            assert!(nested.is_err());
            Ok(())
        })
        .unwrap();
        assert_eq!(*trace.lock().unwrap(), ["outer"]);
    }

    #[test]
    fn commit_finalizers_do_not_run_when_commit_itself_fails() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(parent_id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED)").unwrap();
        let env = Env::default();
        let finalized = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let observed = finalized.clone();
        let result = run_write(&conn, &env, move |tx| {
            tx.conn().execute("INSERT INTO child VALUES(1)", [])?;
            tx.on_commit_success(move || observed.store(true, Ordering::SeqCst));
            Ok(())
        });
        assert!(result.is_err(), "deferred foreign key must reject COMMIT");
        assert!(!finalized.load(Ordering::SeqCst));
        assert!(conn.is_autocommit());
        assert_eq!(
            conn.query_row("SELECT count(*) FROM child", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn identity_snapshot_generation_covers_rollbacks_and_after_commit() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(
            Config::new(dir.path().join("generation.sqlite3")),
            Env::default(),
        )
        .unwrap();
        let snapshot = db.clone();
        let first = db
            .write_blocking(move |tx| {
                let version = snapshot.write_generation();
                assert_eq!(version % 2, 1, "writer is active");
                tx.after_commit(move |tx| {
                    assert_eq!(
                        snapshot.write_generation(),
                        version,
                        "after-commit callbacks are still writer work"
                    );
                    run_write(tx.conn(), tx.env, |nested| {
                        assert_eq!(snapshot.write_generation(), version);
                        nested.after_commit(move |_| {
                            assert_eq!(snapshot.write_generation(), version);
                            Ok(())
                        });
                        Ok(())
                    })
                });
                Ok(version)
            })
            .unwrap();
        let snapshot = db.clone();
        let rejected = db.write_blocking(move |_| -> Result<()> {
            assert_eq!(snapshot.write_generation(), first + 2);
            Err(Error::Other("intentional rollback".into()))
        });
        assert!(rejected.is_err());
        let snapshot = db.clone();
        db.write_blocking(move |_| {
            assert_eq!(
                snapshot.write_generation(),
                first + 4,
                "rollback also invalidates the snapshot"
            );
            Ok(())
        })
        .unwrap();
    }
    fn main_file_len(path: &Path) -> u64 {
        std::fs::metadata(path).unwrap().len()
    }

    #[test]
    fn a_panicking_read_returns_its_connection() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::new(dir.path().join("test.sqlite3"));
        config.readers = 1;
        let db = Database::open(config, Env::default()).unwrap();
        for _ in 0..3 {
            let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                db.read_blocking(|_| -> Result<()> { panic!("a bug in a read") })
            }));
            assert!(panicked.is_err());
        }
        // With one reader, a lost connection would make this wait forever.
        assert_eq!(
            db.read_blocking(|conn| Ok(conn.query_row("SELECT 1", [], |r| r.get::<_, i64>(0))?))
                .unwrap(),
            1
        );
    }

    /// Commits never checkpoint on the writer: the WAL reaching the auto-checkpoint threshold
    /// wakes the checkpointer, which copies it into the database file on its own.
    #[test]
    fn the_checkpointer_copies_the_wal_into_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        let mut config = Config::new(&path);
        config.readers = 1;
        let db = Database::open(config, Env::default()).unwrap();
        db.write_blocking(|tx| Ok(tx.conn().execute_batch("CREATE TABLE filler (data BLOB)")?))
            .unwrap();
        let before = main_file_len(&path);

        // ~1,200 pages of 4 KiB, over a few commits.
        for _ in 0..6 {
            db.write_blocking(|tx| {
                Ok(tx.conn().execute_batch("WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 200) INSERT INTO filler SELECT randomblob(3900) FROM n")?)
            })
            .unwrap();
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while main_file_len(&path) < before + 1000 * 4096 {
            assert!(
                std::time::Instant::now() < deadline,
                "the WAL was never checkpointed"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// Writes that never pause still get the WAL restarted, at WAL_LIMIT_PAGES.
    #[test]
    fn the_wal_stays_bounded_under_sustained_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        let mut config = Config::new(&path);
        config.readers = 1;
        let db = Database::open(config, Env::default()).unwrap();
        db.write_blocking(|tx| Ok(tx.conn().execute_batch("CREATE TABLE filler (data BLOB)")?))
            .unwrap();

        // ~25,000 pages, 500 per commit.
        for _ in 0..50 {
            db.write_blocking(|tx| {
                Ok(tx.conn().execute_batch(
                    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 500) INSERT INTO filler SELECT randomblob(3900) FROM n",
                )?)
            })
            .unwrap();
        }
        let wal = std::fs::metadata(path.with_extension("sqlite3-wal"))
            .unwrap()
            .len();
        assert!(
            wal < (WAL_LIMIT_PAGES as u64 + 1000) * 4200,
            "WAL of {wal} bytes"
        );
    }
}
