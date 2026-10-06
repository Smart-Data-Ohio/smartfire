//! `reference/app/models/workspace_presence_lease.rb`: the channel side. A lease per open
//! `WorkspacePresenceChannel` subscription, established on subscribe, refreshed by its
//! heartbeat and deleted on unsubscribe. The periodic prune and the presence reads belong to the
//! presence workstream.
//!
//! Validations (`connection_id` present and unique, `expires_at` present, `belongs_to :session`
//! and `:user`) hold by construction: establishing writes a fresh UUID, an expiry, and the ids of
//! a session and user it has just checked exist.

use jiff::SignedDuration;
use rusqlite::{Connection, Row, params};

use crate::database::Tx;
use crate::error::{Error, Result};
use crate::sql::{self, CachedStatements, query_one, query_all, placeholders};
use crate::time::Timestamp;

/// `WorkspacePresenceLease::TTL`
pub const TTL: SignedDuration = SignedDuration::from_secs(90);
/// `WorkspacePresenceLease::IDLE_AFTER`
pub const IDLE_AFTER: SignedDuration = SignedDuration::from_mins(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Presence { Online, Idle, Offline, Dnd }

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspacePresenceLease {
    pub id: i64,
    pub connection_id: String,
    pub expires_at: Timestamp,
    pub last_active_at: Option<Timestamp>,
    pub session_id: i64,
    pub user_id: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    /// `destroyed?`: set by [`Self::delete`]. Active Record refuses `update_columns` on it.
    pub destroyed: bool,
}

impl WorkspacePresenceLease {
    /// Reads never prune: the hot presence poll must not take SQLite's write lock.
    pub fn presence_by_user_id(conn: &Connection, ids: &[i64], now: Timestamp) -> Result<std::collections::HashMap<i64, Presence>> {
        if ids.is_empty() { return Ok(std::collections::HashMap::new()); }
        let sql = format!("SELECT leases.user_id, leases.last_active_at FROM workspace_presence_leases leases JOIN sessions ON sessions.id=leases.session_id JOIN users ON users.id=leases.user_id WHERE leases.expires_at>=? AND users.status=0 AND sessions.user_id=leases.user_id AND leases.user_id IN ({})", placeholders(ids.len()));
        let mut values: Vec<rusqlite::types::Value> = vec![now.to_db().into()];
        values.extend(ids.iter().map(|id| (*id).into()));
        let rows: Vec<(i64, Option<Timestamp>)> = query_all(conn, &sql, rusqlite::params_from_iter(values), |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut states = std::collections::HashMap::new();
        for (user_id, last_active_at) in rows {
            let state = if last_active_at.is_none_or(|at| at >= now.ago(IDLE_AFTER)) { Presence::Online } else { Presence::Idle };
            states.entry(user_id).and_modify(|old| { if state == Presence::Online { *old = state; } }).or_insert(state);
        }
        Ok(states)
    }

    pub fn prune(tx: &mut Tx<'_>, limit: usize) -> Result<usize> {
        Ok(tx.conn().execute_cached("DELETE FROM workspace_presence_leases WHERE id IN (SELECT id FROM workspace_presence_leases WHERE expires_at < ? OR NOT EXISTS (SELECT 1 FROM sessions WHERE sessions.id=workspace_presence_leases.session_id AND sessions.user_id=workspace_presence_leases.user_id) LIMIT ?)", params![tx.now(), limit as i64])?)
    }
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            connection_id: row.get("connection_id")?,
            expires_at: row.get("expires_at")?,
            last_active_at: row.get("last_active_at")?,
            session_id: row.get("session_id")?,
            user_id: row.get("user_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            destroyed: false,
        })
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(conn, r#"SELECT * FROM "workspace_presence_leases" WHERE "workspace_presence_leases"."id" = ? LIMIT 1"#, [id], Self::from_row)
    }

    /// `WorkspacePresenceLease.establish(user:, session:)`: a new lease, or `None` when the user
    /// isn't active or the session is gone or someone else's.
    pub fn establish(tx: &mut Tx<'_>, user_id: i64, session_id: i64) -> Result<Option<Self>> {
        if !Self::identity_valid(tx.conn(), user_id, session_id)? {
            return Ok(None);
        }
        let now = tx.now();
        let connection_id = sql::uuid();
        let expires_at = now.since(TTL);
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "workspace_presence_leases" ("connection_id", "created_at", "expires_at", "last_active_at", "session_id", "updated_at", "user_id") VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![connection_id, now, expires_at, now, session_id, now, user_id],
            |row| row.get(0),
        )?;
        Ok(Some(Self { id, connection_id, expires_at, last_active_at: Some(now), session_id, user_id, created_at: now, updated_at: now, destroyed: false }))
    }

    /// `identity_valid?(user:, session:)`: `User.active.exists?(id:)` and
    /// `Session.exists?(id:, user_id:)`.
    pub fn identity_valid(conn: &Connection, user_id: i64, session_id: i64) -> Result<bool> {
        let active = sql::count(conn, r#"SELECT COUNT(*) FROM (SELECT 1 FROM "users" WHERE "users"."status" = 0 AND "users"."id" = ? LIMIT 1)"#, [user_id])? > 0;
        let session = sql::count(
            conn,
            r#"SELECT COUNT(*) FROM (SELECT 1 FROM "sessions" WHERE "sessions"."id" = ? AND "sessions"."user_id" = ? LIMIT 1)"#,
            [session_id, user_id],
        )? > 0;
        Ok(active && session)
    }

    /// `refresh(active:)`: with the identity still valid, extends the lease (and its activity,
    /// when the heartbeat reports recent input) with `update_columns`, which is true only if the
    /// row is still there; otherwise deletes it and returns false. `update_columns` raises on a
    /// lease this value deleted.
    pub fn refresh(&mut self, tx: &mut Tx<'_>, active: bool) -> Result<bool> {
        if !Self::identity_valid(tx.conn(), self.user_id, self.session_id)? {
            self.delete(tx)?;
            return Ok(false);
        }
        if self.destroyed {
            return Err(Error::Other("cannot update a destroyed record".into()));
        }
        let now = tx.now();
        let expires_at = now.since(TTL);
        let updated = if active {
            tx.conn().execute_cached(
                r#"UPDATE "workspace_presence_leases" SET "expires_at" = ?, "last_active_at" = ? WHERE "workspace_presence_leases"."id" = ?"#,
                params![expires_at, now, self.id],
            )?
        } else {
            tx.conn().execute_cached(
                r#"UPDATE "workspace_presence_leases" SET "expires_at" = ? WHERE "workspace_presence_leases"."id" = ?"#,
                params![expires_at, self.id],
            )?
        };
        self.expires_at = expires_at;
        if active {
            self.last_active_at = Some(now);
        }
        Ok(updated == 1)
    }

    /// `delete`: no callbacks.
    pub fn delete(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn().execute_cached(r#"DELETE FROM "workspace_presence_leases" WHERE "workspace_presence_leases"."id" = ?"#, [self.id])?;
        self.destroyed = true;
        Ok(())
    }
}
