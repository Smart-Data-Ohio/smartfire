//! `reference/app/models/activity_item.rb`, the part WS8's reminders need: an inbox row per
//! recipient and source, refreshed unread in place, with its `user_<id>_activity` broadcast. The
//! inbox itself (`accessible_to`, filters, grouping, mark read and handled, every other event
//! type's writer) is WS12's.

use rusqlite::{Connection, Row, params};

use crate::broadcasts::Broadcast;
use crate::database::Tx;
use crate::error::{Errors, OptionalExt, Result};
use crate::events::Event;
use crate::models::User;
use crate::sql::{CachedStatements, query_one};
use crate::time::Timestamp;

/// `ActivityItem::EVENT_TYPES`
pub const EVENT_TYPES: [&str; 20] = [
    "mention",
    "reply",
    "thread_activity",
    "keyword_alert",
    "work_update",
    "work_assignment",
    "work_sla",
    "huddle_started",
    "huddle_missed",
    "event_invitation",
    "event_update",
    "event_cancelled",
    "event_reminder",
    "pr_review_request",
    "agent_approval_request",
    "agent_budget_exceeded",
    "message_reminder",
    "scheduled_message_dropped",
    "two_factor_lockout",
    "new_sign_in",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ActivityItem {
    pub id: i64,
    pub user_id: i64,
    pub source_type: String,
    pub source_id: i64,
    pub event_type: String,
    pub read_at: Option<Timestamp>,
    pub handled_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl ActivityItem {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            source_type: row.get("source_type")?,
            source_id: row.get("source_id")?,
            event_type: row.get("event_type")?,
            read_at: row.get("read_at")?,
            handled_at: row.get("handled_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(conn, r#"SELECT * FROM "activity_items" WHERE "id" = ? LIMIT 1"#, [id], Self::from_row)?.or_not_found("ActivityItem")
    }

    /// `ActivityItem.find_by(user:, source:)`
    pub fn find_by_user_and_source(conn: &Connection, user_id: i64, source_type: &str, source_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "activity_items" WHERE "user_id" = ? AND "source_type" = ? AND "source_id" = ? LIMIT 1"#,
            params![user_id, source_type, source_id],
            Self::from_row,
        )
    }

    /// `unread?`
    pub fn unread(&self) -> bool {
        self.read_at.is_none() && self.handled_at.is_none()
    }

    /// `find_or_initialize_by(user:, source:)`, then `event_type =`, unread again (`read_at` and
    /// `handled_at` nil), `save!`. A new row broadcasts (`after_create_commit`); an existing one
    /// broadcasts only when its state or type changed (`broadcast_updated`).
    pub fn refresh_unread(tx: &mut Tx<'_>, user_id: i64, source_type: &str, source_id: i64, event_type: &str) -> Result<Self> {
        let mut errors = Errors::default();
        if !EVENT_TYPES.contains(&event_type) {
            errors.add("event_type", "is not included in the list");
        }
        if User::find_by_id(tx.conn(), user_id)?.is_none() {
            errors.add("user", "must exist");
        }
        errors.into_result()?;
        let now = tx.now();
        let item = match Self::find_by_user_and_source(tx.conn(), user_id, source_type, source_id)? {
            Some(item) => {
                let changed = item.event_type != event_type || item.read_at.is_some() || item.handled_at.is_some();
                if changed {
                    tx.conn().execute_cached(
                        r#"UPDATE "activity_items" SET "event_type" = ?, "read_at" = NULL, "handled_at" = NULL, "updated_at" = ? WHERE "id" = ?"#,
                        params![event_type, now, item.id],
                    )?;
                    Self::broadcast_change(tx, user_id, item.id)?;
                }
                Self::find(tx.conn(), item.id)?
            }
            None => {
                let id: i64 = tx.conn().query_row_cached(
                    r#"INSERT INTO "activity_items" ("created_at", "event_type", "source_id", "source_type", "updated_at", "user_id") VALUES (?, ?, ?, ?, ?, ?) RETURNING "id""#,
                    params![now, event_type, source_id, source_type, now, user_id],
                    |r| r.get(0),
                )?;
                Self::broadcast_change(tx, user_id, id)?;
                Self::find(tx.conn(), id)?
            }
        };
        Ok(item)
    }

    /// `broadcast_activity_change`: to active humans only, on `ActivityChannel`'s stream, after
    /// commit. (Huddle items' invitation payload is WS13's.)
    pub(crate) fn broadcast_change(tx: &mut Tx<'_>, user_id: i64, id: i64) -> Result<()> {
        let human = User::find_by_id(tx.conn(), user_id)?.is_some_and(|user| user.is_active() && !user.is_bot());
        if human {
            if crate::models::huddle_invitations::enqueue_item_ring(tx, id)? {
                return Ok(());
            }
            tx.emit_after_commit(Event::broadcast(&Broadcast::Cable {
                stream: format!("user_{user_id}_activity"),
                payload: serde_json::json!({ "activityItemId": id }),
            }));
        }
        Ok(())
    }

    /// `mark_handled!`: preserve an existing read timestamp when accepting a late invite.
    pub fn mark_handled(&self, tx: &mut Tx<'_>) -> Result<Self> {
        tx.conn().execute_cached("UPDATE activity_items SET read_at=COALESCE(read_at,?),handled_at=?,updated_at=? WHERE id=?", params![tx.now(),tx.now(),tx.now(),self.id])?;
        Self::broadcast_change(tx,self.user_id,self.id)?;
        Self::find(tx.conn(),self.id)
    }

    /// `has_many :activity_items, as: :source, dependent: :destroy`
    pub(crate) fn destroy_for_source(tx: &Tx<'_>, source_type: &str, source_id: i64) -> Result<()> {
        tx.conn().execute_cached(
            r#"DELETE FROM "activity_items" WHERE "source_type" = ? AND "source_id" = ?"#,
            params![source_type, source_id],
        )?;
        Ok(())
    }
}
