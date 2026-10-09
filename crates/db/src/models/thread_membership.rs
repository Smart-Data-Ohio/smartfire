//! `reference/app/models/thread_membership.rb`: who follows a thread, and whether it changed
//! since they last read it.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{Connection, Row, params};

use crate::database::Tx;
use crate::error::{Errors, OptionalExt, Result};
use crate::models::{ChannelThread, Membership};
use crate::sql::{self, CachedStatements, query_all, query_one};
use crate::time::Timestamp;

/// `enum :involvement, %w[ nothing mentions everything ]`, prefixed `involved_in_`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThreadInvolvement {
    /// Muted: nothing at all, not even mentions.
    Nothing,
    /// Unfollowed (the default): mentions only.
    Mentions,
    /// Followed: every reply.
    Everything,
}

impl ThreadInvolvement {
    pub fn name(self) -> &'static str {
        match self {
            ThreadInvolvement::Nothing => "nothing",
            ThreadInvolvement::Mentions => "mentions",
            ThreadInvolvement::Everything => "everything",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "nothing" => Some(ThreadInvolvement::Nothing),
            "mentions" => Some(ThreadInvolvement::Mentions),
            "everything" => Some(ThreadInvolvement::Everything),
            _ => None,
        }
    }
}

impl ToSql for ThreadInvolvement {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.name()))
    }
}

impl FromSql for ThreadInvolvement {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let name = value.as_str()?;
        ThreadInvolvement::from_name(name)
            .ok_or_else(|| FromSqlError::Other(format!("unknown thread involvement {name:?}").into()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThreadMembership {
    pub id: i64,
    pub thread_id: i64,
    pub user_id: i64,
    pub involvement: ThreadInvolvement,
    pub joined_at: Timestamp,
    /// When a followed conversation last changed unread for this member; nil once read.
    pub unread_at: Option<Timestamp>,
    /// The newest message id when the member last read the thread (none until then): a ping at
    /// or below it was read, so a later reply making the thread unread doesn't count it again.
    pub last_read_message_id: Option<i64>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl ThreadMembership {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            thread_id: row.get("thread_id")?,
            user_id: row.get("user_id")?,
            involvement: row.get("involvement")?,
            joined_at: row.get("joined_at")?,
            unread_at: row.get("unread_at")?,
            last_read_message_id: row.get("last_read_message_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(conn, r#"SELECT * FROM "thread_memberships" WHERE "thread_memberships"."id" = ? LIMIT 1"#, [id], Self::from_row)?
            .or_not_found("ThreadMembership")
    }

    /// `thread.memberships.find_by(user_id:)` (`ChannelThread#membership_for`)
    pub fn find_by_thread_and_user(conn: &Connection, thread_id: i64, user_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT "thread_memberships".* FROM "thread_memberships" WHERE "thread_memberships"."thread_id" = ? AND "thread_memberships"."user_id" = ? LIMIT 1"#,
            [thread_id, user_id],
            Self::from_row,
        )
    }

    /// `thread.memberships`, in id order.
    pub fn for_thread(conn: &Connection, thread_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT "thread_memberships".* FROM "thread_memberships" WHERE "thread_memberships"."thread_id" = ? ORDER BY "thread_memberships"."id" ASC"#,
            [thread_id],
            Self::from_row,
        )
    }

    /// The viewer's memberships for a page of threads, read together.
    pub fn for_user_threads(conn: &Connection, user_id: i64, thread_ids: &[i64]) -> Result<Vec<Self>> {
        if thread_ids.is_empty() {
            return Ok(Vec::new());
        }
        query_all(
            conn,
            "SELECT * FROM thread_memberships WHERE user_id=? AND thread_id IN (SELECT value FROM json_each(?))",
            params![user_id, serde_json::json!(thread_ids).to_string()],
            Self::from_row,
        )
    }

    /// `user.thread_memberships.unread`
    pub fn unread_for_user(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT "thread_memberships".* FROM "thread_memberships" WHERE "thread_memberships"."user_id" = ? AND "thread_memberships"."unread_at" IS NOT NULL ORDER BY "thread_memberships"."id" ASC"#,
            [user_id],
            Self::from_row,
        )
    }

    /// `ThreadMembership.join!(thread, user)`: the user must be a member of the thread's room
    /// (`find_by!`, so `RecordNotFound` otherwise); then `find_or_create_by!`.
    pub fn join(tx: &mut Tx<'_>, thread_id: i64, user_id: i64) -> Result<Self> {
        let thread = ChannelThread::find(tx.conn(), thread_id)?;
        Membership::find_by_room_and_user(tx.conn(), thread.room_id, user_id)?.or_not_found("Membership")?;
        if let Some(existing) = Self::find_by_thread_and_user(tx.conn(), thread_id, user_id)? {
            return Ok(existing);
        }
        Self::create(tx, thread_id, user_id, ThreadInvolvement::Mentions)
    }

    /// `ThreadMembership.create!(thread:, user:, involvement:)`: `joined_at` defaults to now.
    pub fn create(tx: &mut Tx<'_>, thread_id: i64, user_id: i64, involvement: ThreadInvolvement) -> Result<Self> {
        Self::validate(tx.conn(), thread_id, user_id)?.into_result()?;
        let now = tx.now();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "thread_memberships" ("created_at", "involvement", "joined_at", "thread_id", "updated_at", "user_id") VALUES (?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![now, involvement, now, thread_id, now, user_id],
            |r| r.get(0),
        )?;
        Self::find(tx.conn(), id)
    }

    /// `user_is_a_parent_room_member`
    pub fn validate(conn: &Connection, thread_id: i64, user_id: i64) -> Result<Errors> {
        let mut errors = Errors::default();
        let member = sql::exists(
            conn,
            r#"SELECT 1 AS one FROM "memberships" INNER JOIN "channel_threads" ON "channel_threads"."room_id" = "memberships"."room_id" WHERE "channel_threads"."id" = ? AND "memberships"."user_id" = ? LIMIT 1"#,
            [thread_id, user_id],
        )?;
        let thread_exists = sql::exists(conn, r#"SELECT 1 AS one FROM "channel_threads" WHERE "id" = ? LIMIT 1"#, [thread_id])?;
        if thread_exists && !member {
            errors.add("user", "must belong to the parent room");
        }
        Ok(errors)
    }

    /// `update!(involvement:)`, validated like any save.
    pub fn update_involvement(&mut self, tx: &mut Tx<'_>, involvement: ThreadInvolvement) -> Result<()> {
        if involvement == self.involvement {
            return Ok(());
        }
        Self::validate(tx.conn(), self.thread_id, self.user_id)?.into_result()?;
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "thread_memberships" SET "involvement" = ?, "updated_at" = ? WHERE "thread_memberships"."id" = ?"#,
            params![involvement, now, self.id],
        )?;
        self.involvement = involvement;
        self.updated_at = now;
        Ok(())
    }

    /// `read`: `update!(unread_at: nil)`, a no-op when already read. It also records the newest
    /// message read through (`last_read_message_id`, never moving back): every message committed
    /// so far has a lower id than any later one, so the pings it read stay read.
    pub fn read(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        if self.unread_at.is_none() {
            return Ok(());
        }
        Self::validate(tx.conn(), self.thread_id, self.user_id)?.into_result()?;
        let now = tx.now();
        let read_through: Option<i64> = tx.conn().query_row_cached(
            r#"UPDATE "thread_memberships" SET "unread_at" = NULL, "last_read_message_id" = MAX(COALESCE("last_read_message_id", 0), COALESCE((SELECT MAX("id") FROM "messages"), 0)), "updated_at" = ? WHERE "thread_memberships"."id" = ? RETURNING "last_read_message_id""#,
            params![now, self.id],
            |r| r.get(0),
        )?;
        self.unread_at = None;
        self.last_read_message_id = read_through;
        self.updated_at = now;
        Ok(())
    }

    pub fn unread(&self) -> bool {
        self.unread_at.is_some()
    }

    pub fn involved_in(&self, involvement: ThreadInvolvement) -> bool {
        self.involvement == involvement
    }

    pub fn reload(&mut self, conn: &Connection) -> Result<()> {
        *self = Self::find(conn, self.id)?;
        Ok(())
    }

    /// `Membership#remove_thread_membership` (`after_destroy_commit`): leaving a room leaves
    /// every thread in it, without callbacks (`delete_all`).
    pub(crate) fn delete_for_room_member(tx: &Tx<'_>, room_id: i64, user_id: i64) -> Result<()> {
        tx.conn().execute_cached(
            r#"DELETE FROM "thread_memberships" WHERE "thread_memberships"."id" IN (SELECT "thread_memberships"."id" FROM "thread_memberships" INNER JOIN "channel_threads" ON "channel_threads"."id" = "thread_memberships"."thread_id" WHERE "thread_memberships"."user_id" = ? AND "channel_threads"."room_id" = ?)"#,
            params![user_id, room_id],
        )?;
        Ok(())
    }
}
