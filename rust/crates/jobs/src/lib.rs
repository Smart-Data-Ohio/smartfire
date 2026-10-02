//! The durable job queue and the periodic scheduler, in place of Resque, resque-scheduler,
//! `bin/periodic` and `bin/huddle-reconcile` (decisions.md §1–§3: one process, no Redis).
//!
//! # The queue
//!
//! Jobs are rows of `background_jobs` in the main database (a Rails migration creates the table;
//! Rails never touches it). A job is enqueued by emitting [`campfire_db::Event::Job`] (or one of
//! upstream's job events, which the app maps to a [`JobRequest`]) from a write: the database's
//! [`EventSink::persist`](campfire_db::EventSink::persist) hook inserts the row **in that write's
//! transaction**, so the job commits or rolls back with the write that enqueued it, and no worker
//! can see it before the commit. After the commit, [`EventSink::emit`](campfire_db::EventSink)
//! wakes the job's queue ([`JobQueue::wake`]), so a job starts within a writer round trip of its
//! commit. That is `after_commit { SomeJob.perform_later }`, minus the window in which a crash
//! between the commit and the enqueue loses the job.
//!
//! Each job class is a [`JobKind`] with its queue (`queue_as`), payload version and
//! [`RetryPolicy`] (`retry_on`/`discard_on`), registered with its handler in a [`Registry`] by
//! the module that owns it. Nothing central lists them.
//!
//! The [`Runner`] claims due jobs with one conditional `UPDATE ... RETURNING` per queue (on the
//! single writer, so claims never race), holding a lease it extends with a heartbeat while the job
//! runs. Each queue runs at most its concurrency of jobs at once, counted in the database, so a
//! serial queue (`slack_import`) stays serial even with several runners over one database. A
//! successful job's row is deleted; a failing one is retried with backoff until its attempts run
//! out, then kept as `failed` for inspection ([`inspect`]). A lease that expires (its process
//! died) is recovered: the job is retried if it has attempts left.
//!
//! # The scheduler
//!
//! [`periodic::Periodic`] is `Periodic::Runner`: named tasks with intervals, each isolated from
//! the others' failures, ticking in one loop. The huddle reconciler's 5-second loop is a
//! `Periodic` of its own.
//!
//! # Claims and leases
//!
//! [`claims`] holds the helpers for the conditional-UPDATE claim pattern domain code uses on its
//! own tables (stuck-claim sweeps, JSON-state step leases).

pub mod claims;
pub mod inspect;
pub mod periodic;

mod kind;
mod queue;
mod registry;
mod retry;
mod runner;
mod store;

pub use campfire_db::{Job, JobRequest};
pub use kind::{Execution, JobError, JobKind, JobResult, Outcome, RetryPolicy, Wait, is_transient};
pub use queue::JobQueue;
pub use registry::Registry;
pub use runner::{QueueConfig, Runner, RunnerConfig, start};
pub use store::{FAILED, READY, RUNNING, TABLE};

#[cfg(test)]
mod tests;
