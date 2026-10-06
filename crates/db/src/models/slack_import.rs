//! `app/models/slack_import.rb`: persisted run lifecycle, single-flight claims and step leases.
//! Call every mutation inside the writer transaction. Rendering and Slack HTTP stay outside it.
use std::collections::HashSet;

use jiff::SignedDuration;
use rand::RngCore;
use rusqlite::{Connection, Row, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::sql::{query_all, query_one};
use crate::{Error, Errors, Event, Job, Result, Timestamp, Tx};

pub const STALE_HEARTBEAT: SignedDuration = SignedDuration::from_secs(300);
pub const ISSUE_CAP: i64 = 1000;

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Workspace,
    Personal,
}
impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Personal => "personal",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Mode {
    DryRun,
    Import,
}
impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry_run",
            Self::Import => "import",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum StepStatus {
    Running,
    Undoing,
}
impl StepStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Undoing => "undoing",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StepJob {
    pub import_id: i64,
}
impl Job for StepJob {
    const CLASS: &'static str = "SlackImport::StepJob";
}
#[derive(Debug, Serialize, Deserialize)]
pub struct UndoJob {
    pub import_id: i64,
}
impl Job for UndoJob {
    const CLASS: &'static str = "SlackImport::UndoJob";
}

/// Options must have already gone through SlackImport.normalize_options. This layer never
/// interprets untrusted controller parameters; the controller/domain boundary owns coercions.
pub struct NewImport {
    pub workspace_id: i64,
    pub connection_id: Option<i64>,
    pub user_id: i64,
    pub kind: Kind,
    pub mode: Mode,
    pub options: Value,
}

#[derive(Debug, Clone)]
pub struct SlackImport {
    pub id: i64,
    pub slack_workspace_id: i64,
    pub slack_connection_id: Option<i64>,
    pub user_id: i64,
    pub kind: String,
    pub mode: String,
    pub status: String,
    pub options: Value,
    pub state: Value,
    pub stats: Value,
    pub error: Option<String>,
    pub started_at: Option<Timestamp>,
    pub heartbeat_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

pub fn lease_stamp(at: Timestamp) -> String {
    at.jiff().strftime("%Y-%m-%dT%H:%M:%S.%6fZ").to_string()
}

fn json_column(row: &Row<'_>, name: &str) -> rusqlite::Result<Value> {
    let text: String = row.get(name)?;
    serde_json::from_str(&text).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            row.as_ref().column_index(name).unwrap_or(0),
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })
}

// Only cancelled/failed runs can hold a finishing lease outside an active status. A stale
// lease-looking state on a completed/undone run must not prevent future claims.
const BLOCKING: &str = "(status IN ('running','undoing') OR (status IN ('cancelled','failed') AND json_extract(state, '$.step_started_at') > ?))";

impl SlackImport {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            slack_workspace_id: row.get("slack_workspace_id")?,
            slack_connection_id: row.get("slack_connection_id")?,
            user_id: row.get("user_id")?,
            kind: row.get("kind")?,
            mode: row.get("mode")?,
            status: row.get("status")?,
            options: json_column(row, "options")?,
            state: json_column(row, "state")?,
            stats: json_column(row, "stats")?,
            error: row.get("error")?,
            started_at: row.get("started_at")?,
            heartbeat_at: row.get("heartbeat_at")?,
            finished_at: row.get("finished_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM slack_imports WHERE id = ?",
            [id],
            Self::from_row,
        )
    }

    pub fn create(tx: &mut Tx<'_>, new: NewImport) -> Result<Self> {
        Self::create_with_enqueued_at(tx, new, None)
    }

    /// Rails `start!` stamps pending jobs with the request's Time.current zone. Lease stamps
    /// remain UTC; this stamp is parsed as an instant and never compared lexically in SQL.
    pub fn create_with_enqueued_at(
        tx: &mut Tx<'_>,
        new: NewImport,
        enqueued_at: Option<&str>,
    ) -> Result<Self> {
        require_transaction(tx)?;
        let mut errors = Errors::default();
        for (table, attribute, id) in [
            (
                "slack_workspaces",
                "slack_workspace",
                Some(new.workspace_id),
            ),
            ("users", "user", Some(new.user_id)),
            ("slack_connections", "slack_connection", new.connection_id),
        ] {
            if let Some(id) = id {
                let exists: bool = tx.conn().query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?)"),
                    [id],
                    |r| r.get(0),
                )?;
                if !exists {
                    errors.add(attribute, "must exist");
                }
            }
        }
        errors.into_result()?;
        let now = tx.now();
        let id = tx.conn().query_row("INSERT INTO slack_imports (slack_workspace_id, slack_connection_id, user_id, kind, mode, options, state, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
            params![new.workspace_id, new.connection_id, new.user_id, new.kind.as_str(), new.mode.as_str(), new.options.to_string(), json!({"enqueued_at": enqueued_at.map(str::to_owned).unwrap_or_else(||lease_stamp(now))}).to_string(), now, now], |r| r.get(0))?;
        tx.emit_after_commit(Event::job(&StepJob { import_id: id }));
        Self::find(tx.conn(), id)?.ok_or(Error::RecordNotFound("SlackImport"))
    }

    pub fn active(&self) -> bool {
        matches!(self.status.as_str(), "queued" | "running" | "undoing")
    }
    pub fn cancellable(&self) -> bool {
        matches!(self.status.as_str(), "queued" | "running")
    }
    pub fn undo_eligible(&self) -> bool {
        self.mode == "import"
            && matches!(self.status.as_str(), "completed" | "failed" | "cancelled")
    }
    pub fn touched_conversation_ids(&self) -> HashSet<String> {
        self.stats["conversations"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                matches!(entry["target"]["action"].as_str(), Some("create" | "merge"))
                    .then(|| entry["id"].as_str().map(str::to_owned))
                    .flatten()
            })
            .collect()
    }
    pub fn step_lease_fresh(&self, now: Timestamp) -> bool {
        fresh_stamp(&self.state["step_started_at"], now)
    }
    pub fn step_job_pending(&self, now: Timestamp) -> bool {
        fresh_stamp(&self.state["enqueued_at"], now)
    }

    pub fn claim_running(tx: &mut Tx<'_>, id: i64) -> Result<bool> {
        require_transaction(tx)?;
        let now = tx.now();
        Ok(tx.conn().execute(&format!("UPDATE slack_imports SET status = 'running', started_at = ?, heartbeat_at = ?, updated_at = ? WHERE id = ? AND status = 'queued' AND NOT EXISTS (SELECT 1 FROM slack_imports WHERE {BLOCKING})"),
            params![now, now, now, id, lease_stamp(now.ago(STALE_HEARTBEAT))])? == 1)
    }

    pub fn acquire_step_lease(
        tx: &mut Tx<'_>,
        id: i64,
        status: StepStatus,
    ) -> Result<Option<String>> {
        require_transaction(tx)?;
        let now = tx.now();
        let mut bytes = [0_u8; 8];
        rand::rng().fill_bytes(&mut bytes);
        let token = hex::encode(bytes);
        let claimed = tx.conn().execute("UPDATE slack_imports SET state = json_set(state, '$.step_started_at', ?, '$.step_lease_token', ?), heartbeat_at = ?, updated_at = ? WHERE id = ? AND status = ? AND (json_extract(state, '$.step_started_at') IS NULL OR json_extract(state, '$.step_started_at') <= ?)",
            params![lease_stamp(now), token, now, now, id, status.as_str(), lease_stamp(now.ago(STALE_HEARTBEAT))])? == 1;
        Ok(claimed.then_some(token))
    }

    pub fn refresh_step_lease(tx: &mut Tx<'_>, id: i64, token: &str) -> Result<bool> {
        require_transaction(tx)?;
        if token.trim().is_empty() {
            return Ok(false);
        }
        let now = tx.now();
        Ok(tx.conn().execute("UPDATE slack_imports SET state = json_set(state, '$.step_started_at', ?), heartbeat_at = ?, updated_at = ? WHERE id = ? AND json_extract(state, '$.step_lease_token') = ?",
            params![lease_stamp(now), now, now, id, token])? == 1)
    }

    pub fn release_step_lease(tx: &mut Tx<'_>, id: i64, token: &str) -> Result<bool> {
        require_transaction(tx)?;
        if token.trim().is_empty() {
            return Ok(false);
        }
        Ok(tx.conn().execute("UPDATE slack_imports SET state = json_remove(state, '$.step_started_at', '$.step_lease_token'), updated_at = ? WHERE id = ? AND json_extract(state, '$.step_lease_token') = ?",
            params![tx.now(), id, token])? == 1)
    }

    pub fn clear_pending_step_job(tx: &mut Tx<'_>, id: i64) -> Result<()> {
        require_transaction(tx)?;
        tx.conn().execute("UPDATE slack_imports SET state = json_remove(state, '$.enqueued_at'), updated_at = ? WHERE id = ? AND json_extract(state, '$.enqueued_at') IS NOT NULL", params![tx.now(), id])?;
        Ok(())
    }

    pub fn kick_next_queued(tx: &mut Tx<'_>) -> Result<Option<i64>> {
        require_transaction(tx)?;
        let now = tx.now();
        let blocked: bool = tx.conn().query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM slack_imports WHERE {BLOCKING})"),
            [lease_stamp(now.ago(STALE_HEARTBEAT))],
            |r| r.get(0),
        )?;
        if blocked {
            return Ok(None);
        }
        let Some(oldest) = query_one(
            tx.conn(),
            "SELECT * FROM slack_imports WHERE status = 'queued' ORDER BY created_at, id LIMIT 1",
            [],
            Self::from_row,
        )?
        else {
            return Ok(None);
        };
        if oldest.step_job_pending(now) {
            return Ok(None);
        }
        tx.conn().execute("UPDATE slack_imports SET state = json_set(state, '$.enqueued_at', ?), updated_at = ? WHERE id = ?", params![lease_stamp(now), now, oldest.id])?;
        tx.emit_after_commit(Event::job(&StepJob {
            import_id: oldest.id,
        }));
        Ok(Some(oldest.id))
    }

    /// Rails may enqueue two stale-running jobs on overlapping sweeps. The persisted step
    /// lease, rather than enqueue deduplication, prevents both from executing a step.
    pub fn sweep_stalled(tx: &mut Tx<'_>) -> Result<()> {
        require_transaction(tx)?;
        let stale = tx.now().ago(STALE_HEARTBEAT);
        let runs = query_all(
            tx.conn(),
            "SELECT * FROM slack_imports WHERE status IN ('running','undoing') AND (heartbeat_at IS NULL OR heartbeat_at < ?) ORDER BY id",
            [stale],
            Self::from_row,
        )?;
        for run in &runs {
            if run.status == "running" {
                tx.emit_after_commit(Event::job(&StepJob { import_id: run.id }));
            }
        }
        Self::kick_next_queued(tx)?;
        for run in runs {
            if run.status == "undoing" {
                tx.emit_after_commit(Event::job(&UndoJob { import_id: run.id }));
            }
        }
        Ok(())
    }

    pub fn cancel(tx: &mut Tx<'_>, id: i64) -> Result<bool> {
        require_transaction(tx)?;
        let changed = tx.conn().execute("UPDATE slack_imports SET status = 'cancelled', finished_at = ?, updated_at = ? WHERE id = ? AND status IN ('queued','running')", params![tx.now(), tx.now(), id])? == 1;
        if changed {
            Self::kick_next_queued(tx)?;
        }
        Ok(changed)
    }

    pub fn mark_failed(tx: &mut Tx<'_>, id: i64, message: &str) -> Result<()> {
        require_transaction(tx)?;
        tx.conn().execute("UPDATE slack_imports SET status = 'failed', error = ?, finished_at = ?, updated_at = ? WHERE id = ?", params![message, tx.now(), tx.now(), id])?;
        Self::kick_next_queued(tx)?;
        Ok(())
    }

    pub fn later_overlapping_import(&self, conn: &Connection) -> Result<Option<Self>> {
        let Some(started) = self.started_at else {
            return Ok(None);
        };
        let mut mine = self.touched_conversation_ids();
        let keys = query_all(
            conn,
            "SELECT slack_key FROM slack_import_records WHERE slack_import_id = ? AND slack_kind = 'conversation'",
            [self.id],
            |r| r.get::<_, String>(0),
        )?;
        mine.extend(keys);
        if mine.is_empty() {
            return Ok(None);
        }
        let later = query_all(
            conn,
            "SELECT * FROM slack_imports WHERE slack_workspace_id = ? AND mode = 'import' AND id != ? AND status != 'undone' AND (started_at > ? OR (started_at = ? AND id > ?)) ORDER BY started_at DESC, id DESC",
            params![self.slack_workspace_id, self.id, started, started, self.id],
            Self::from_row,
        )?;
        Ok(later.into_iter().find(|run| {
            run.touched_conversation_ids()
                .iter()
                .any(|id| mine.contains(id))
        }))
    }

    pub fn undo_blocked_reason(&self, conn: &Connection, now: Timestamp) -> Result<Option<String>> {
        if !self.undo_eligible() {
            return Ok(None);
        }
        if let Some(later) = self.later_overlapping_import(conn)? {
            return Ok(Some(if later.user_id == self.user_id {
                format!(
                    "A later import (#{}) also imported some of these conversations; undo that one first.",
                    later.id
                )
            } else {
                let name: String = conn.query_row(
                    "SELECT name FROM users WHERE id = ?",
                    [later.user_id],
                    |r| r.get(0),
                )?;
                format!(
                    "A later import by {name} also imported some of these conversations. It has to be undone first; ask them or an administrator."
                )
            }));
        }
        if self.step_lease_fresh(now) {
            return Ok(Some(
                "This import is still finishing. Wait for it to finish, then undo.".into(),
            ));
        }
        let active: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM slack_imports WHERE id != ? AND status IN ('queued','running','undoing'))", [self.id], |r| r.get(0))?;
        if active {
            return Ok(Some(
                "Another import is queued or running. Wait for it to finish, then undo.".into(),
            ));
        }
        let finishing: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM slack_imports WHERE id != ? AND status IN ('cancelled','failed') AND json_extract(state, '$.step_started_at') > ?)", params![self.id, lease_stamp(now.ago(STALE_HEARTBEAT))], |r| r.get(0))?;
        Ok(finishing
            .then(|| "Another import is still finishing. Wait for it to finish, then undo.".into()))
    }

    pub fn undo(tx: &mut Tx<'_>, id: i64) -> Result<bool> {
        require_transaction(tx)?;
        let Some(run) = Self::find(tx.conn(), id)? else {
            return Ok(false);
        };
        // BEGIN IMMEDIATE also makes the overlap scan and this claim atomic with new runs.
        if !run.undo_eligible() || run.undo_blocked_reason(tx.conn(), tx.now())?.is_some() {
            return Ok(false);
        }
        Self::claim_undo(tx, id)
    }

    /// The status claim rechecks queue/lease exclusion even after a caller's pre-check.
    pub(crate) fn claim_undo(tx: &mut Tx<'_>, id: i64) -> Result<bool> {
        require_transaction(tx)?;
        let now = tx.now();
        let claimed = tx.conn().execute(&format!("UPDATE slack_imports SET status = 'undoing', state = '{{\"phase\":\"undo\"}}', stats = json_set(stats, '$.phase', 'undo'), heartbeat_at = ?, finished_at = NULL, updated_at = ? WHERE id = ? AND status IN ('completed','failed','cancelled') AND NOT EXISTS (SELECT 1 FROM slack_imports WHERE id != ? AND (status = 'queued' OR {BLOCKING}))"),
            params![now, now, id, id, lease_stamp(now.ago(STALE_HEARTBEAT))])? == 1;
        if claimed {
            tx.emit_after_commit(Event::job(&UndoJob { import_id: id }));
        }
        Ok(claimed)
    }

    pub fn record_issue(
        tx: &mut Tx<'_>,
        id: i64,
        level: IssueLevel,
        slack_ref: Option<&str>,
        message: &str,
    ) -> Result<bool> {
        require_transaction(tx)?;
        let count: i64 = tx.conn().query_row(
            "SELECT COUNT(*) FROM slack_import_issues WHERE slack_import_id = ?",
            [id],
            |r| r.get(0),
        )?;
        if count > ISSUE_CAP {
            return Ok(false);
        }
        let suppression = format!("Further issues suppressed (over {ISSUE_CAP})");
        let (level, slack_ref, message) = if count == ISSUE_CAP {
            (IssueLevel::Warning, None, suppression.as_str())
        } else {
            (level, slack_ref, message)
        };
        let mut errors = Errors::default();
        if message.chars().all(char::is_whitespace) {
            errors.add("message", "can't be blank");
        }
        if Self::find(tx.conn(), id)?.is_none() {
            errors.add("slack_import", "must exist");
        }
        errors.into_result()?;
        tx.conn().execute("INSERT INTO slack_import_issues (slack_import_id, level, slack_ref, message, created_at) VALUES (?, ?, ?, ?, ?)", params![id, level.as_str(), slack_ref, message, tx.now()])?;
        Ok(true)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IssueLevel {
    Warning,
    Error,
}
impl IssueLevel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}
fn fresh_stamp(value: &Value, now: Timestamp) -> bool {
    value
        .as_str()
        .and_then(|s| s.parse::<jiff::Timestamp>().ok())
        .is_some_and(|at| at > now.ago(STALE_HEARTBEAT).jiff())
}
fn require_transaction(tx: &Tx<'_>) -> Result<()> {
    if tx.in_transaction() {
        Ok(())
    } else {
        Err(Error::Other(
            "Slack import mutations require a transaction".into(),
        ))
    }
}
