//! The `background_jobs` table (`db/migrate/20260929193000_create_background_jobs.rb`). Every
//! statement runs on the writer connection, inside the write that calls it: claims and state
//! changes are conditional UPDATEs, so a stale runner (one whose lease was recovered) never
//! overwrites another's claim.

use campfire_db::{CachedStatements, Connection, Result, Timestamp};
use rusqlite::{OptionalExtension, params};

pub const TABLE: &str = "background_jobs";

/// Waiting for its `run_at`, or due.
pub const READY: &str = "ready";
/// Claimed by a runner (`claimed_by`), under a lease until `lease_expires_at`.
pub const RUNNING: &str = "running";
/// Failed for good: out of attempts, or an error that isn't retried. Kept for inspection
/// ([`crate::inspect`]) until retried or deleted by hand.
pub const FAILED: &str = "failed";

/// The longest `last_error` kept, in bytes.
const ERROR_LIMIT: usize = 8 * 1024;

pub(crate) struct NewJob<'a> {
    pub queue: &'a str,
    pub class: &'a str,
    pub arguments: &'a serde_json::Value,
    pub version: u32,
    pub run_at: Timestamp,
}

pub(crate) fn insert(conn: &Connection, job: NewJob<'_>, now: Timestamp) -> Result<i64> {
    let arguments = serde_json::to_string(job.arguments).map_err(|e| campfire_db::Error::Other(e.to_string()))?;
    Ok(conn.query_row_cached(
        r#"INSERT INTO "background_jobs" ("queue_name", "job_class", "arguments", "payload_version", "status", "attempts", "run_at", "created_at", "updated_at")
           VALUES (?1, ?2, ?3, ?4, 'ready', 0, ?5, ?6, ?6) RETURNING "id""#,
        params![job.queue, job.class, arguments, job.version, job.run_at, now],
        |row| row.get(0),
    )?)
}

/// A job a runner claimed.
#[derive(Debug, Clone)]
pub(crate) struct Claimed {
    pub id: i64,
    pub class: String,
    pub arguments: String,
    pub version: u32,
    /// Executions so far, the claimed one included.
    pub attempts: u32,
    pub run_at: Timestamp,
    pub created_at: Timestamp,
}

/// Claims up to `limit` due jobs of `queue` for `runner`, oldest due first, leaving the queue with
/// no more than `concurrency` jobs running under live leases (across every runner on this
/// database). One statement, so it's atomic; the writer serializes it with every other claim.
pub(crate) fn claim(
    conn: &Connection,
    queue: &str,
    concurrency: usize,
    limit: usize,
    runner: &str,
    lease_until: Timestamp,
    now: Timestamp,
) -> Result<Vec<Claimed>> {
    let mut statement = conn.prepare_cached(
        r#"UPDATE "background_jobs"
              SET "status" = 'running', "claimed_by" = ?1, "lease_expires_at" = ?2, "attempts" = "attempts" + 1, "updated_at" = ?3
            WHERE "status" = 'ready' AND "id" IN (
              SELECT "id" FROM "background_jobs"
               WHERE "status" = 'ready' AND "queue_name" = ?4 AND "run_at" <= ?3
               ORDER BY "run_at", "id"
               LIMIT max(0, min(?5, ?6 - (SELECT count(*) FROM "background_jobs"
                                           WHERE "status" = 'running' AND "queue_name" = ?4 AND "lease_expires_at" > ?3))))
           RETURNING "id", "job_class", "arguments", "payload_version", "attempts", "run_at", "created_at""#,
    )?;
    let mut claimed = statement
        .query_map(params![runner, lease_until, now, queue, limit as i64, concurrency as i64], |row| {
            Ok(Claimed {
                id: row.get(0)?,
                class: row.get(1)?,
                arguments: row.get(2)?,
                version: row.get(3)?,
                attempts: row.get(4)?,
                run_at: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    // RETURNING comes back in no particular order.
    claimed.sort_by_key(|job| (job.run_at, job.id));
    Ok(claimed)
}

/// When `queue`'s next waiting job is due.
pub(crate) fn next_run_at(conn: &Connection, queue: &str) -> Result<Option<Timestamp>> {
    Ok(conn
        .prepare_cached(r#"SELECT min("run_at") FROM "background_jobs" WHERE "status" = 'ready' AND "queue_name" = ?1"#)?
        .query_row([queue], |row| row.get::<_, Option<Timestamp>>(0))
        .optional()?
        .flatten())
}

/// Deletes a job `runner` still holds: it's done, or discarded.
pub(crate) fn delete(conn: &Connection, id: i64, runner: &str) -> Result<bool> {
    Ok(conn.prepare_cached(r#"DELETE FROM "background_jobs" WHERE "id" = ?1 AND "status" = 'running' AND "claimed_by" = ?2"#)?.execute(params![id, runner])? == 1)
}

/// Puts a job `runner` holds back in its queue, due at `run_at`: a retry (keeping its attempts and
/// recording the error), or a fresh run ([`crate::Outcome::Again`], which resets them).
pub(crate) fn reschedule(conn: &Connection, id: i64, runner: &str, run_at: Timestamp, error: Option<&str>, reset_attempts: bool, now: Timestamp) -> Result<bool> {
    Ok(conn
        .prepare_cached(
            r#"UPDATE "background_jobs"
                  SET "status" = 'ready', "run_at" = ?3, "claimed_by" = NULL, "lease_expires_at" = NULL,
                      "last_error" = coalesce(?4, "last_error"), "attempts" = CASE WHEN ?5 THEN 0 ELSE "attempts" END, "updated_at" = ?6
                WHERE "id" = ?1 AND "status" = 'running' AND "claimed_by" = ?2"#,
        )?
        .execute(params![id, runner, run_at, error.map(truncate), reset_attempts, now])?
        == 1)
}

/// Marks a job `runner` holds as failed for good.
pub(crate) fn fail(conn: &Connection, id: i64, runner: &str, error: &str, now: Timestamp) -> Result<bool> {
    Ok(conn
        .prepare_cached(
            r#"UPDATE "background_jobs"
                  SET "status" = 'failed', "failed_at" = ?3, "last_error" = ?4, "claimed_by" = NULL, "lease_expires_at" = NULL, "updated_at" = ?3
                WHERE "id" = ?1 AND "status" = 'running' AND "claimed_by" = ?2"#,
        )?
        .execute(params![id, runner, now, truncate(error)])?
        == 1)
}

/// Extends the leases of the jobs `runner` is performing.
pub(crate) fn heartbeat(conn: &Connection, runner: &str, ids: &[i64], lease_until: Timestamp, now: Timestamp) -> Result<usize> {
    let ids = serde_json::to_string(ids).expect("ids serialize");
    Ok(conn
        .prepare_cached(
            r#"UPDATE "background_jobs" SET "lease_expires_at" = ?2, "updated_at" = ?3
                WHERE "status" = 'running' AND "claimed_by" = ?1 AND "id" IN (SELECT "value" FROM json_each(?4))"#,
        )?
        .execute(params![runner, lease_until, now, ids])?)
}

/// A running job whose lease expired: whoever claimed it stopped heartbeating (the process died).
#[derive(Debug, Clone)]
pub(crate) struct Orphan {
    pub id: i64,
    pub queue: String,
    pub class: String,
    pub attempts: u32,
}

/// Running jobs whose lease has expired, other than those `runner` is still `performing`: its
/// own claims whose execution is over (their outcome couldn't be written) are orphans too.
pub(crate) fn orphans(conn: &Connection, runner: &str, performing: &[i64], now: Timestamp) -> Result<Vec<Orphan>> {
    let performing = serde_json::to_string(performing).expect("ids serialize");
    let mut statement = conn.prepare_cached(
        r#"SELECT "id", "queue_name", "job_class", "attempts" FROM "background_jobs"
            WHERE "status" = 'running' AND "lease_expires_at" <= ?1
              AND NOT ("claimed_by" IS ?2 AND "id" IN (SELECT "value" FROM json_each(?3))) ORDER BY "id""#,
    )?;
    let orphans = statement
        .query_map(params![now, runner, performing], |row| Ok(Orphan { id: row.get(0)?, queue: row.get(1)?, class: row.get(2)?, attempts: row.get(3)? }))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(orphans)
}

/// Takes an orphan back from its dead runner: due again now, or failed for good. Conditional on
/// its lease still being expired, so a runner that was only slow to heartbeat keeps it.
pub(crate) fn recover(conn: &Connection, id: i64, retry: bool, error: &str, now: Timestamp) -> Result<bool> {
    let sql = if retry {
        r#"UPDATE "background_jobs"
              SET "status" = 'ready', "run_at" = ?2, "claimed_by" = NULL, "lease_expires_at" = NULL, "last_error" = ?3, "updated_at" = ?2
            WHERE "id" = ?1 AND "status" = 'running' AND "lease_expires_at" <= ?2"#
    } else {
        r#"UPDATE "background_jobs"
              SET "status" = 'failed', "failed_at" = ?2, "claimed_by" = NULL, "lease_expires_at" = NULL, "last_error" = ?3, "updated_at" = ?2
            WHERE "id" = ?1 AND "status" = 'running' AND "lease_expires_at" <= ?2"#
    };
    Ok(conn.prepare_cached(sql)?.execute(params![id, now, error])? == 1)
}

/// Hands back the jobs `runner` still holds when it stops without finishing them: due again now,
/// without counting the interrupted execution as an attempt.
pub(crate) fn release(conn: &Connection, runner: &str, now: Timestamp) -> Result<usize> {
    Ok(conn
        .prepare_cached(
            r#"UPDATE "background_jobs"
                  SET "status" = 'ready', "run_at" = ?2, "claimed_by" = NULL, "lease_expires_at" = NULL,
                      "attempts" = max("attempts" - 1, 0), "updated_at" = ?2
                WHERE "status" = 'running' AND "claimed_by" = ?1"#,
        )?
        .execute(params![runner, now])?)
}

fn truncate(error: &str) -> &str {
    if error.len() <= ERROR_LIMIT {
        return error;
    }
    let mut end = ERROR_LIMIT;
    while !error.is_char_boundary(end) {
        end -= 1;
    }
    &error[..end]
}
