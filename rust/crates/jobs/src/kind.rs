//! What a job class declares (`queue_as`, `retry_on`, `discard_on`) and what a job's perform
//! returns.

use std::fmt;
use std::time::Duration;

use campfire_db::Timestamp;

/// A job class: its arguments (the [`campfire_db::Job`] it extends), its queue, the version of
/// its payload, and how it retries. Registered with its handler in a
/// [`Registry`](crate::Registry) by the module that owns it.
pub trait JobKind: campfire_db::Job + Send + 'static {
    /// `queue_as`: the queue whose workers perform it.
    const QUEUE: &'static str = "default";

    /// The version of the serialized arguments, stored with each job. Bump it when the arguments
    /// change shape, and teach [`JobKind::upgrade`] the old shape: jobs enqueued by the previous
    /// release may still be waiting.
    const VERSION: u32 = 1;

    /// `retry_on`/`discard_on`: [`RetryPolicy::application_job`] unless the class says otherwise.
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::application_job()
    }

    /// The arguments of a job stored at an older `version`, in this version's shape. A payload
    /// that can't be upgraded (the default) is discarded, like a job whose arguments no longer
    /// deserialize (`discard_on ActiveJob::DeserializationError`).
    fn upgrade(version: u32, _arguments: serde_json::Value) -> Result<serde_json::Value, String> {
        Err(format!("no upgrade from payload version {version}"))
    }
}

/// How a job class retries (`retry_on ..., wait:, attempts:`).
#[derive(Clone, Copy)]
pub struct RetryPolicy {
    /// Executions in all, the first included (`attempts: 5`). At least 1.
    pub attempts: u32,
    /// How long to wait before each retry.
    pub wait: Wait,
    /// Which errors are worth another attempt (the `retry_on` list). Anything else fails the job
    /// for good. Errors a handler classifies itself ([`JobError::Retry`], [`JobError::Discard`],
    /// [`JobError::Fail`]) bypass it.
    pub retry_on: fn(&anyhow::Error) -> bool,
    /// Longest a single execution may run before it's abandoned and counted as a failed attempt
    /// that may be retried (a timeout). `None` (the default, as in Rails) lets it run.
    pub timeout: Option<Duration>,
}

impl RetryPolicy {
    /// The most a `Retry-After` may delay a retry.
    pub const RETRY_AFTER_CAP: Duration = Duration::from_secs(60 * 60);

    /// `ApplicationJob` (`app/jobs/application_job.rb`): `retry_on(*TRANSIENT_ERRORS, wait:
    /// :polynomially_longer)` with `retry_on`'s default 5 attempts and the app's 0.15 retry
    /// jitter (`config.active_job.retry_jitter`, a `load_defaults` value).
    pub fn application_job() -> Self {
        Self { attempts: 5, wait: Wait::PolynomiallyLonger { jitter: 0.15 }, retry_on: is_transient, timeout: None }
    }

    /// `retry_on ..., attempts: 1`: never retried.
    pub fn no_retries() -> Self {
        Self::application_job().attempts(1)
    }

    pub fn attempts(mut self, attempts: u32) -> Self {
        self.attempts = attempts.max(1);
        self
    }

    pub fn wait(mut self, wait: Wait) -> Self {
        self.wait = wait;
        self
    }

    pub fn retry_on(mut self, retry_on: fn(&anyhow::Error) -> bool) -> Self {
        self.retry_on = retry_on;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// The delay before the retry that follows the `executions`th execution, or `None` once the
    /// attempts are used up. `retry_after` (an endpoint's `Retry-After`) replaces the computed
    /// wait, capped at [`Self::RETRY_AFTER_CAP`]. `random` is in `[0, 1)`, for the jitter.
    pub fn retry_delay(&self, executions: u32, retry_after: Option<Duration>, random: f64) -> Option<Duration> {
        if executions >= self.attempts.max(1) {
            return None;
        }
        Some(match retry_after {
            Some(retry_after) => retry_after.min(Self::RETRY_AFTER_CAP),
            None => self.wait.delay(executions, random),
        })
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::application_job()
    }
}

impl fmt::Debug for RetryPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RetryPolicy").field("attempts", &self.attempts).field("wait", &self.wait).field("timeout", &self.timeout).finish()
    }
}

/// `retry_on`'s `wait:` (`ActiveJob::Exceptions#determine_delay`).
#[derive(Debug, Clone, Copy)]
pub enum Wait {
    /// `:polynomially_longer`: `executions**4 + rand * executions**4 * jitter + 2` seconds (3 s,
    /// 18 s, 83 s, 258 s... with no jitter).
    PolynomiallyLonger { jitter: f64 },
    /// A duration (`wait: 3.seconds`, `retry_on`'s default), plus `rand * duration * jitter`.
    Fixed { wait: Duration, jitter: f64 },
    /// A proc of the executions so far.
    Custom(fn(u32) -> Duration),
}

impl Wait {
    pub fn delay(&self, executions: u32, random: f64) -> Duration {
        let jittered = |seconds: f64, jitter: f64| seconds + random * seconds * jitter;
        match *self {
            Wait::PolynomiallyLonger { jitter } => {
                let base = f64::from(executions).powi(4);
                Duration::from_secs_f64(jittered(base, jitter) + 2.0)
            }
            Wait::Fixed { wait, jitter } => Duration::from_secs_f64(jittered(wait.as_secs_f64(), jitter)),
            Wait::Custom(delay) => delay(executions),
        }
    }
}

/// `ApplicationJob::TRANSIENT_ERRORS`, as they surface here: timeouts (`Timeout::Error`,
/// `Net::*Timeout`) and a busy or locked SQLite (`SQLite3::BusyException`,
/// `ActiveRecord::StatementTimeout`), anywhere in the error's chain.
pub fn is_transient(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        if cause.is::<tokio::time::error::Elapsed>() {
            return true;
        }
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            return io.kind() == std::io::ErrorKind::TimedOut;
        }
        let sqlite = match cause.downcast_ref::<campfire_db::Error>() {
            Some(campfire_db::Error::Sqlite(error)) => Some(error),
            _ => cause.downcast_ref::<rusqlite::Error>(),
        };
        matches!(
            sqlite,
            Some(rusqlite::Error::SqliteFailure(failure, _))
                if matches!(failure.code, rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
        )
    })
}

/// A job's execution, as its handler sees it.
#[derive(Debug, Clone)]
pub struct Execution {
    /// The job's row.
    pub id: i64,
    /// Executions so far, this one included (`executions` inside `perform`): 1 the first time.
    pub executions: u32,
    pub enqueued_at: Timestamp,
    /// When it was due.
    pub scheduled_at: Timestamp,
}

/// What a successful execution asks for next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Outcome {
    /// Done: the job is deleted.
    #[default]
    Done,
    /// Perform the same job again after this long, as a fresh job with all its attempts: a job
    /// that re-enqueues itself (`self.class.set(wait:).perform_later(...)`) to continue, or to
    /// wait out a rate limit without spending an attempt.
    Again(Duration),
}

pub type JobResult = Result<Outcome, JobError>;

/// Why an execution failed, and so what happens to the job.
#[derive(Debug)]
pub enum JobError {
    /// Classified by the class's [`RetryPolicy::retry_on`]: retried if it matches and attempts
    /// remain, failed for good otherwise. What `?` produces.
    Error(anyhow::Error),
    /// Worth another attempt whatever the policy's `retry_on` says, after `retry_after` (the
    /// endpoint's `Retry-After`, capped at an hour) or the policy's wait. Failed for good once
    /// the attempts are used up.
    Retry { error: anyhow::Error, retry_after: Option<Duration> },
    /// `discard_on`: the job is dropped (logged), not kept as failed.
    Discard(anyhow::Error),
    /// Failed for good, without a retry: kept as `failed`.
    Fail(anyhow::Error),
}

impl JobError {
    pub fn retry(error: impl Into<anyhow::Error>) -> Self {
        JobError::Retry { error: error.into(), retry_after: None }
    }

    pub fn retry_after(error: impl Into<anyhow::Error>, retry_after: Duration) -> Self {
        JobError::Retry { error: error.into(), retry_after: Some(retry_after) }
    }

    pub fn discard(error: impl Into<anyhow::Error>) -> Self {
        JobError::Discard(error.into())
    }

    pub fn fail(error: impl Into<anyhow::Error>) -> Self {
        JobError::Fail(error.into())
    }

    pub fn error(&self) -> &anyhow::Error {
        match self {
            JobError::Error(error) | JobError::Retry { error, .. } | JobError::Discard(error) | JobError::Fail(error) => error,
        }
    }
}

impl<E: Into<anyhow::Error>> From<E> for JobError {
    fn from(error: E) -> Self {
        JobError::Error(error.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seconds(duration: Duration) -> f64 {
        duration.as_secs_f64()
    }

    /// `determine_delay(:polynomially_longer, executions:)`: `executions**4 + 2`, plus up to
    /// `jitter` of `executions**4`.
    #[test]
    fn polynomially_longer_matches_active_job() {
        let wait = Wait::PolynomiallyLonger { jitter: 0.15 };
        assert_eq!([1, 2, 3, 4].map(|n| seconds(wait.delay(n, 0.0))), [3.0, 18.0, 83.0, 258.0]);
        assert_eq!(seconds(wait.delay(2, 0.5)), 16.0 + 0.5 * 16.0 * 0.15 + 2.0);
        assert!(seconds(wait.delay(4, 0.999_999)) < 256.0 * 1.15 + 2.0);
    }

    #[test]
    fn fixed_waits_jitter_by_their_duration() {
        let wait = Wait::Fixed { wait: Duration::from_secs(3), jitter: 0.15 };
        assert_eq!(seconds(wait.delay(7, 0.0)), 3.0);
        assert_eq!(seconds(wait.delay(7, 0.5)), 3.225);
    }

    /// `executions < attempts`: 5 attempts retry after the 1st through 4th executions only.
    #[test]
    fn retries_stop_when_the_attempts_are_used_up() {
        let policy = RetryPolicy::application_job().wait(Wait::PolynomiallyLonger { jitter: 0.0 });
        let delays: Vec<_> = (1..=5).map(|n| policy.retry_delay(n, None, 0.0).map(seconds)).collect();
        assert_eq!(delays, [Some(3.0), Some(18.0), Some(83.0), Some(258.0), None]);
        assert_eq!(RetryPolicy::no_retries().retry_delay(1, None, 0.0), None);
    }

    #[test]
    fn retry_after_replaces_the_wait_up_to_an_hour() {
        let policy = RetryPolicy::application_job();
        assert_eq!(policy.retry_delay(1, Some(Duration::from_secs(120)), 0.9), Some(Duration::from_secs(120)));
        assert_eq!(policy.retry_delay(1, Some(Duration::from_secs(86_400)), 0.9), Some(Duration::from_secs(3600)));
        assert_eq!(policy.retry_delay(5, Some(Duration::from_secs(1)), 0.9), None, "still bounded by attempts");
    }

    #[test]
    fn transient_errors_are_timeouts_and_a_busy_database() {
        let busy = rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY), None);
        assert!(is_transient(&anyhow::Error::new(campfire_db::Error::Sqlite(busy))));
        let timed_out = std::io::Error::new(std::io::ErrorKind::TimedOut, "read timed out");
        assert!(is_transient(&anyhow::Error::new(timed_out).context("delivering")));
        let constraint = rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT), None);
        assert!(!is_transient(&anyhow::Error::new(constraint)));
        assert!(!is_transient(&anyhow::anyhow!("undefined method 'deliver' for nil")));
    }
}
