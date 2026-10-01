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

use std::cell::Cell;
use std::path::{Path, PathBuf};
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

type AfterCommitHook = Box<dyn FnOnce(&mut Tx<'_>) -> Result<()> + Send>;

enum AfterCommit {
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
    Hook(AfterCommitHook),
    Event(Event),
}

/// A write in progress: the writer connection inside a transaction (or, while after-commit
/// work runs, outside one), the environment, and the queued after-commit work.
pub struct Tx<'c> {
    conn: &'c Connection,
    env: &'c Env,
    in_transaction: bool,
    after_commit: Vec<AfterCommit>,
    /// The first error persisting an emitted event (see [`EventSink::persist`]), which rolls the
    /// transaction back.
    persist_error: Option<Error>,
}

impl<'c> Tx<'c> {
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
            self.env.sink.emit(event);
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
                persist_error: None,
            };
            if let Err(error) = hook(&mut tx) {
                tracing::error!(%error, "after_commit hook failed");
            }
        }
    }

    pub fn in_transaction(&self) -> bool {
        self.in_transaction
    }

    /// A model operation whose validation error may be rescued by its caller. Discard
    /// deferred callbacks together with rolled-back rows; queue persistence failures still
    /// fail the enclosing transaction through `persist_error`.
    pub fn savepoint<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let callbacks = self.after_commit.len();
        self.conn.execute_batch("SAVEPOINT model_operation")?;
        match f(self) {
            Ok(value) => {
                self.conn
                    .execute_batch("RELEASE SAVEPOINT model_operation")?;
                Ok(value)
            }
            Err(error) => {
                self.after_commit.truncate(callbacks);
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
/// after the rest of the queue has run (Rails raises it from the save that committed).
pub fn run_write<T>(
    conn: &Connection,
    env: &Env,
    f: impl FnOnce(&mut Tx<'_>) -> Result<T>,
) -> Result<T> {
    conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;
    let mut tx = Tx {
        conn,
        env,
        in_transaction: true,
        after_commit: Vec::new(),
        persist_error: None,
    };
    let value = match f(&mut tx).and_then(|value| match tx.persist_error.take() {
        Some(error) => Err(error),
        None => Ok(value),
    }) {
        Ok(value) => value,
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK TRANSACTION");
            return Err(error);
        }
    };
    let mut queue = std::mem::take(&mut tx.after_commit);
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

    let mut first_error = None;
    let mut after = Tx {
        conn,
        env,
        in_transaction: false,
        after_commit: Vec::new(),
        persist_error: None,
    };
    for item in queue.drain(..) {
        match item {
            AfterCommit::RecordJob { event, .. } => {
                if let Some(event) = event {
                    env.sink.emit(event);
                }
            }
            AfterCommit::Event(event) => env.sink.emit(event),
            AfterCommit::RecordBroadcast { table, id, event } => {
                // A later destroy suppresses the record's earlier update callback.
                match conn.query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?)"),
                    [id],
                    |r| r.get::<_, bool>(0),
                ) {
                    Ok(true) => env.sink.emit(event),
                    Ok(false) => (),
                    Err(error) => tracing::warn!(%error, table, id, "broadcast callback failed"),
                }
            }
            AfterCommit::Hook(hook) => {
                if let Err(error) = hook(&mut after) {
                    tracing::error!(%error, "after_commit hook failed");
                    first_error.get_or_insert(error);
                }
            }
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(value),
    }
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

type Job = Box<dyn FnOnce(&Connection, &Env) + Send>;

/// The database handle. Cheap to clone.
#[derive(Clone)]
pub struct Database {
    writer: mpsc::Sender<Job>,
    readers: Arc<ReaderPool>,
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
        std::thread::Builder::new()
            .name("campfire-db-writer".into())
            .spawn(move || {
                while let Some(job) = receiver.blocking_recv() {
                    // A panicking write must not take the writer down with it.
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        job(&conn, &writer_env)
                    }));
                    if outcome.is_err() && !conn.is_autocommit() {
                        let _ = conn.execute_batch("ROLLBACK TRANSACTION");
                    }
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
            readers: Arc::new(ReaderPool::new(readers)),
            env,
            path: config.path,
        })
    }

    pub fn env(&self) -> &Env {
        &self.env
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Runs `f` as one immediate transaction on the writer thread.
    pub async fn write<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Tx<'_>) -> Result<T> + Send + 'static,
    {
        let (reply, response) = oneshot::channel();
        self.writer
            .send(Box::new(move |conn, env| {
                let _ = reply.send(run_write(conn, env, f));
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
            .blocking_send(Box::new(move |conn, env| {
                let _ = reply.send(run_write(conn, env, f));
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
    idle: Mutex<Vec<Connection>>,
    available: Condvar,
}

impl ReaderPool {
    fn new(connections: Vec<Connection>) -> Self {
        Self {
            idle: Mutex::new(connections),
            available: Condvar::new(),
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
        f(checkout.conn.as_ref().expect("checked out"))
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
