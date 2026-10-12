use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use super::ChannelThread;
use crate::database::Tx;
use crate::error::{Error, Result};
use crate::sql::{query_all, query_one};
use crate::{Membership, Room, Timestamp, User};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThreadVisibility {
    #[default]
    Public,
    Private,
}

impl ToSql for ThreadVisibility {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(match self {
            Self::Public => "public",
            Self::Private => "private",
        }))
    }
}
impl FromSql for ThreadVisibility {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "public" => Ok(Self::Public),
            "private" => Ok(Self::Private),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ThreadAccessError {
    #[error("Only private threads have access members")]
    NotPrivate,
    #[error("Only the creator, access members and administrators can invite people")]
    InviteForbidden,
    #[error("Only the creator and administrators can remove others")]
    RemoveForbidden,
    #[error("The invited person must have access to the parent room")]
    ParentRoomRequired,
    #[error("Thread visibility cannot change after creation")]
    VisibilityImmutable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadAccessMembership {
    pub thread_id: i64,
    pub user_id: i64,
    pub added_by_id: Option<i64>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadAccessChange {
    pub thread_id: i64,
    pub room_id: i64,
    pub user_id: i64,
    pub added: bool,
}
impl crate::events::Broadcast for ThreadAccessChange {
    const KIND: &'static str = "ChannelThread#access_changed";
}

/// The SQL form of `readable_by`, also used before pagination and aggregate counts.
/// Expressions come from source code, never request input.
pub fn readable_by_sql(thread_id: &str, user_id: &str) -> String {
    format!(
        "EXISTS (SELECT 1 FROM channel_threads access_thread JOIN rooms access_room ON access_room.id=access_thread.room_id JOIN memberships access_parent ON access_parent.room_id=access_room.id JOIN users access_user ON access_user.id=access_parent.user_id WHERE access_thread.id=({thread_id}) AND access_user.id=({user_id}) AND access_room.deleted_at IS NULL AND access_user.status=0 AND access_user.role!=3 AND (access_thread.visibility='public' OR access_user.role=1 OR EXISTS (SELECT 1 FROM thread_access_memberships access_member WHERE access_member.thread_id=access_thread.id AND access_member.user_id=access_user.id)))"
    )
}

impl ChannelThread {
    /// Parent-room access is always required, including for administrators.
    /// Bots and agents receive no exception; private threads require an explicit grant.
    pub fn readable_by(&self, conn: &Connection, user_id: i64) -> Result<bool> {
        Ok(conn.query_row(
            &format!("SELECT {}", readable_by_sql("?1", "?2")),
            [self.id, user_id],
            |row| row.get(0),
        )?)
    }

    pub fn access_members(&self, conn: &Connection) -> Result<Vec<ThreadAccessMembership>> {
        if self.visibility != ThreadVisibility::Private {
            return Err(ThreadAccessError::NotPrivate.into());
        }
        query_all(
            conn,
            "SELECT thread_id,user_id,added_by_id,created_at FROM thread_access_memberships WHERE thread_id=? ORDER BY created_at,user_id",
            [self.id],
            |row| {
                Ok(ThreadAccessMembership {
                    thread_id: row.get(0)?,
                    user_id: row.get(1)?,
                    added_by_id: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
    }

    pub fn add_access_member(
        &self,
        tx: &mut Tx<'_>,
        actor_id: i64,
        user_id: i64,
    ) -> Result<ThreadAccessMembership> {
        let current = Self::find(tx.conn(), self.id)?;
        if current.visibility != ThreadVisibility::Private {
            return Err(ThreadAccessError::NotPrivate.into());
        }
        let actor = User::find(tx.conn(), actor_id)?;
        let parent = Room::find(tx.conn(), current.room_id)?;
        if parent.deleted_at.is_some()
            || !actor.is_active()
            || Membership::find_by_room_and_user(tx.conn(), current.room_id, actor_id)?.is_none()
        {
            return Err(ThreadAccessError::InviteForbidden.into());
        }
        if current.creator_id != actor_id
            && !actor.is_administrator()
            && !current.readable_by(tx.conn(), actor_id)?
        {
            return Err(ThreadAccessError::InviteForbidden.into());
        }
        let target = User::find_by_id(tx.conn(), user_id)?;
        if target.is_none_or(|user| !user.is_active())
            || Membership::find_by_room_and_user(tx.conn(), current.room_id, user_id)?.is_none()
        {
            return Err(ThreadAccessError::ParentRoomRequired.into());
        }
        let inserted = tx.conn().execute("INSERT INTO thread_access_memberships(thread_id,user_id,added_by_id,created_at) VALUES (?,?,?,?) ON CONFLICT(thread_id,user_id) DO NOTHING", params![current.id,user_id,actor_id,tx.now()])?;
        if inserted != 0 {
            crate::ThreadMembership::join(tx, current.id, user_id)?;
            tx.broadcast_after_commit_once(&ThreadAccessChange {
                thread_id: current.id,
                room_id: current.room_id,
                user_id,
                added: true,
            });
        }
        query_one(tx.conn(), "SELECT thread_id,user_id,added_by_id,created_at FROM thread_access_memberships WHERE thread_id=? AND user_id=?", [current.id,user_id], |row| Ok(ThreadAccessMembership { thread_id: row.get(0)?, user_id: row.get(1)?, added_by_id: row.get(2)?, created_at: row.get(3)? }))?.ok_or(Error::RecordNotFound("ThreadAccessMembership"))
    }

    pub fn remove_access_member(&self, tx: &mut Tx<'_>, actor_id: i64, user_id: i64) -> Result<()> {
        let current = Self::find(tx.conn(), self.id)?;
        if current.visibility != ThreadVisibility::Private {
            return Err(ThreadAccessError::NotPrivate.into());
        }
        let actor = User::find(tx.conn(), actor_id)?;
        if !actor.is_active()
            || Room::find(tx.conn(), current.room_id)?.deleted_at.is_some()
            || Membership::find_by_room_and_user(tx.conn(), current.room_id, actor_id)?.is_none()
            || (actor_id != current.creator_id && !current.readable_by(tx.conn(), actor_id)?)
            || (actor_id != user_id && actor_id != current.creator_id && !actor.is_administrator())
        {
            return Err(ThreadAccessError::RemoveForbidden.into());
        }
        let removed = tx.conn().execute(
            "DELETE FROM thread_access_memberships WHERE thread_id=? AND user_id=?",
            [current.id, user_id],
        )?;
        if removed != 0 {
            tx.conn().execute(
                "UPDATE thread_memberships SET unread_at=NULL WHERE thread_id=? AND user_id=?",
                [current.id, user_id],
            )?;
            tx.broadcast_after_commit_once(&ThreadAccessChange {
                thread_id: current.id,
                room_id: current.room_id,
                user_id,
                added: false,
            });
            tx.emit_after_commit(crate::Event::DisconnectUser {
                user_id,
                reconnect: true,
            });
        }
        Ok(())
    }
}
