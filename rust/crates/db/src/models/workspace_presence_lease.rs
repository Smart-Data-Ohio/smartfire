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
use crate::error::Result;
use crate::sql::{self, CachedStatements, query_one};
use crate::time::Timestamp;

/// `WorkspacePresenceLease::TTL`
pub const TTL: SignedDuration = SignedDuration::from_secs(90);
/// `WorkspacePresenceLease::IDLE_AFTER`
pub const IDLE_AFTER: SignedDuration = SignedDuration::from_mins(10);

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
}

impl WorkspacePresenceLease {
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
        Ok(Some(Self { id, connection_id, expires_at, last_active_at: Some(now), session_id, user_id, created_at: now, updated_at: now }))
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
    /// row is still there; otherwise deletes it and returns false.
    pub fn refresh(&mut self, tx: &mut Tx<'_>, active: bool) -> Result<bool> {
        if !Self::identity_valid(tx.conn(), self.user_id, self.session_id)? {
            self.delete(tx)?;
            return Ok(false);
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
    pub fn delete(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn().execute_cached(r#"DELETE FROM "workspace_presence_leases" WHERE "workspace_presence_leases"."id" = ?"#, [self.id])?;
        Ok(())
    }
}
