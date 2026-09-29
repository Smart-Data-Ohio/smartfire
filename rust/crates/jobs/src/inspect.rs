//! Looking at the queue by hand: what's waiting, running and failed, and putting failed jobs back
//! (Resque's failed queue and its "retry"/"remove"). Reads take a reader connection; changes take
//! a write.

use campfire_db::{CachedStatements, Connection, Result, Timestamp, Tx};
use rusqlite::params;

/// A job's row.
#[derive(Debug, Clone, PartialEq)]
pub struct JobRow {
    pub id: i64,
    pub queue: String,
    pub class: String,
    pub arguments: serde_json::Value,
    pub payload_version: u32,
    pub status: String,
    pub attempts: u32,
    pub run_at: Timestamp,
    pub claimed_by: Option<String>,
    pub lease_expires_at: Option<Timestamp>,
    pub last_error: Option<String>,
    pub failed_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

const COLUMNS: &str = r#""id", "queue_name", "job_class", "arguments", "payload_version", "status", "attempts", "run_at", "claimed_by", "lease_expires_at", "last_error", "failed_at", "created_at", "updated_at""#;

fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRow> {
    let arguments: String = row.get(3)?;
    Ok(JobRow {
        id: row.get(0)?,
        queue: row.get(1)?,
        class: row.get(2)?,
        arguments: serde_json::from_str(&arguments).unwrap_or(serde_json::Value::String(arguments)),
        payload_version: row.get(4)?,
        status: row.get(5)?,
        attempts: row.get(6)?,
        run_at: row.get(7)?,
        claimed_by: row.get(8)?,
        lease_expires_at: row.get(9)?,
        last_error: row.get(10)?,
        failed_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub fn find(conn: &Connection, id: i64) -> Result<Option<JobRow>> {
    use rusqlite::OptionalExtension;
    Ok(conn.query_row_cached(&format!(r#"SELECT {COLUMNS} FROM "background_jobs" WHERE "id" = ?1"#), [id], from_row).optional()?)
}

/// Every job with `status` ([`crate::READY`], [`crate::RUNNING`] or [`crate::FAILED`]), oldest
/// first.
pub fn with_status(conn: &Connection, status: &str) -> Result<Vec<JobRow>> {
    let mut statement = conn.prepare_cached(&format!(r#"SELECT {COLUMNS} FROM "background_jobs" WHERE "status" = ?1 ORDER BY "id""#))?;
    Ok(statement.query_map([status], from_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Every job, oldest first.
pub fn all(conn: &Connection) -> Result<Vec<JobRow>> {
    let mut statement = conn.prepare_cached(&format!(r#"SELECT {COLUMNS} FROM "background_jobs" ORDER BY "id""#))?;
    Ok(statement.query_map([], from_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// How many jobs each queue has in each status: `(queue, status, count)`.
pub fn counts(conn: &Connection) -> Result<Vec<(String, String, i64)>> {
    let mut statement = conn.prepare_cached(r#"SELECT "queue_name", "status", count(*) FROM "background_jobs" GROUP BY 1, 2 ORDER BY 1, 2"#)?;
    Ok(statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Puts a failed job back in its queue, due now, with all its attempts. Returns whether it was
/// failed.
pub fn retry_failed(tx: &Tx<'_>, id: i64) -> Result<bool> {
    let now = tx.now();
    Ok(tx.conn().execute_cached(
        r#"UPDATE "background_jobs" SET "status" = 'ready', "attempts" = 0, "run_at" = ?2, "failed_at" = NULL, "updated_at" = ?2
            WHERE "id" = ?1 AND "status" = 'failed'"#,
        params![id, now],
    )? == 1)
}

/// Deletes a failed job. Returns whether it was failed.
pub fn delete_failed(tx: &Tx<'_>, id: i64) -> Result<bool> {
    Ok(tx.conn().execute_cached(r#"DELETE FROM "background_jobs" WHERE "id" = ?1 AND "status" = 'failed'"#, [id])? == 1)
}
