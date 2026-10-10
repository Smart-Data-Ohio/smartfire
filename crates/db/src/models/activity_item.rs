//! app/models/activity_item.rb: inbox sources, live accessibility, state and after-commit signals.

mod access;
pub mod message_recorder;
mod recorder;
mod recording_source;
pub use recording_source::{ActivityEventType, ActivityRecordingFacts, ActivityRecordingSource, AgentBudgetNoticeActivityReader, SourceAuthorization};
pub use recorder::ActivitySource;
pub use access::{ActivityQuery, ActivityUnread};

use rusqlite::{Connection, Row, params};

use crate::broadcasts::Broadcast;
use crate::database::Tx;
use crate::error::{Errors, OptionalExt, Result};
use crate::models::User;
use crate::sql::{CachedStatements, query_one};
use crate::time::Timestamp;

/// Removed inbox items, as `(id, user_id)`. The classic inbox drops them only on
/// reload, so it has no frame; the cable sink publishes the single-page app's `activity.removed`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActivityItemsRemoved {
    pub items: Vec<(i64, i64)>,
    /// A retained message's room, so removing a mention also refreshes its sidebar counts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<i64>,
}

impl crate::events::Broadcast for ActivityItemsRemoved {
    const KIND: &'static str = "ActivityItem#sync_removed";
}

/// An unread grouped thread item re-pointed at a newer reply (`record_authorized`). The classic
/// inbox isn't told (`app/models/activity_item.rb` broadcasts state changes only); the cable sink
/// publishes the single-page app's `activity.item` so its row shows the newer reply.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActivityItemTouched {
    pub id: i64,
    pub user_id: i64,
}

impl crate::events::Broadcast for ActivityItemTouched {
    const KIND: &'static str = "ActivityItem#sync_touched";
}

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
        query_one(
            conn,
            r#"SELECT * FROM "activity_items" WHERE "id" = ? LIMIT 1"#,
            [id],
            Self::from_row,
        )?
        .or_not_found("ActivityItem")
    }

    /// `ActivityItem.find_by(user:, source:)`
    pub fn find_by_user_and_source(
        conn: &Connection,
        user_id: i64,
        source_type: &str,
        source_id: i64,
    ) -> Result<Option<Self>> {
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

    pub fn read(&self) -> bool {
        self.read_at.is_some() && self.handled_at.is_none()
    }
    pub fn handled(&self) -> bool {
        self.handled_at.is_some()
    }
    pub fn state(&self) -> &'static str {
        if self.handled() {
            "handled"
        } else if self.read_at.is_some() {
            "read"
        } else {
            "unread"
        }
    }

    /// Return the freshly saved row for the inbox payload (also WS13's established API).
    pub fn mark_read(&self, tx: &mut Tx<'_>) -> Result<Self> {
        if self.read_at.is_none() {
            self.save_state(tx, Some(tx.now()), self.handled_at)
        } else {
            Self::find(tx.conn(), self.id)
        }
    }
    pub fn mark_unread(&self, tx: &mut Tx<'_>) -> Result<Self> {
        self.save_state(tx, None, None)
    }
    pub fn mark_unhandled(&self, tx: &mut Tx<'_>) -> Result<Self> {
        self.save_state(tx, self.read_at, None)
    }

    fn save_state(
        &self,
        tx: &mut Tx<'_>,
        read_at: Option<Timestamp>,
        handled_at: Option<Timestamp>,
    ) -> Result<Self> {
        let read_changed = self.read_at != read_at;
        let handled_changed = self.handled_at != handled_at;
        // Rails writes only dirty columns from the loaded instance, preserving concurrent changes
        // to other columns. No dirty state also leaves updated_at and the callback untouched.
        if !read_changed && !handled_changed {
            return Self::find(tx.conn(), self.id);
        }
        self.validate_event_type()?;
        let now = tx.now();
        match (read_changed, handled_changed) {
            (true, true) => tx.conn().execute_cached(
                "UPDATE activity_items SET read_at=?,handled_at=?,updated_at=? WHERE id=?",
                params![read_at, handled_at, now, self.id],
            )?,
            (true, false) => tx.conn().execute_cached(
                "UPDATE activity_items SET read_at=?,updated_at=? WHERE id=?",
                params![read_at, now, self.id],
            )?,
            (false, true) => tx.conn().execute_cached(
                "UPDATE activity_items SET handled_at=?,updated_at=? WHERE id=?",
                params![handled_at, now, self.id],
            )?,
            (false, false) => unreachable!(),
        };
        // Rails callbacks see the saved instance, including untouched snapshot columns.
        let saved = Self {
            read_at,
            handled_at,
            updated_at: now,
            ..self.clone()
        };
        Self::broadcast_item(tx, self.user_id, &saved)?;
        Self::find(tx.conn(), self.id)
    }

    /// `find_or_initialize_by(user:, source:)`, then `event_type =`, unread again (`read_at` and
    /// `handled_at` nil), `save!`. A new row broadcasts (`after_create_commit`); an existing one
    /// broadcasts only when its state or type changed (`broadcast_updated`).
    pub fn refresh_unread(
        tx: &mut Tx<'_>,
        user_id: i64,
        source_type: &str,
        source_id: i64,
        event_type: &str,
    ) -> Result<Self> {
        Self::refresh_unread_inner(tx, user_id, source_type, source_id, event_type, true)
    }

    /// [`Self::refresh_unread`] without the broadcast, for a Slack import's undo.
    pub(crate) fn refresh_unread_quietly(
        tx: &mut Tx<'_>,
        user_id: i64,
        source_type: &str,
        source_id: i64,
        event_type: &str,
    ) -> Result<Self> {
        Self::refresh_unread_inner(tx, user_id, source_type, source_id, event_type, false)
    }

    fn refresh_unread_inner(
        tx: &mut Tx<'_>,
        user_id: i64,
        source_type: &str,
        source_id: i64,
        event_type: &str,
        announce: bool,
    ) -> Result<Self> {
        let previous = Self::find_by_user_and_source(tx.conn(), user_id, source_type, source_id)?;
        let mut errors = Errors::default();
        // Rails 8 validates required associations only on new/changed foreign keys.
        // A state/type refresh of an existing row must still work after its source disappears.
        if previous.is_none() {
            if User::find_by_id(tx.conn(), user_id)?.is_none() {
                errors.add("user", "must exist");
            }
            // The eleven inbox source writers have known model tables. Arbitrary polymorphic
            // source construction belongs to the still-pending generic recorder port (WS12).
            let table = match source_type {
                "Message" => Some("messages"),
                "SavedItem" => Some("saved_items"),
                "WorkThreadEvent" => Some("work_thread_events"),
                "BoardSlaNudge" => Some("board_sla_nudges"),
                "HuddleGrant" => Some("huddle_grants"),
                "Event" => Some("events"),
                "AgentApproval" => Some("agent_approvals"),
                "AgentBudgetNotice" => Some("agent_budget_notices"),
                "ScheduledMessage" => Some("scheduled_messages"),
                "TwoFactorCredential" => Some("two_factor_credentials"),
                "Session" => Some("sessions"),
                _ => None,
            };
            if let Some(table) = table {
                let exists: bool = tx.conn().query_row_cached(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?)"),
                    [source_id],
                    |row| row.get(0),
                )?;
                if !exists {
                    errors.add("source", "must exist");
                }
            }
        }
        if !EVENT_TYPES.contains(&event_type) {
            errors.add("event_type", "is not included in the list");
        }
        errors.into_result()?;
        let now = tx.now();
        let item = match previous {
            Some(item) => {
                let changed = item.event_type != event_type
                    || item.read_at.is_some()
                    || item.handled_at.is_some();
                if changed {
                    tx.conn().execute_cached(
                        r#"UPDATE "activity_items" SET "event_type" = ?, "read_at" = NULL, "handled_at" = NULL, "updated_at" = ? WHERE "id" = ?"#,
                        params![event_type, now, item.id],
                    )?;
                    if announce {
                        Self::broadcast_change(tx, user_id, item.id)?;
                    }
                }
                Self::find(tx.conn(), item.id)?
            }
            None => {
                let id: i64 = tx.conn().query_row_cached(
                    r#"INSERT INTO "activity_items" ("created_at", "event_type", "source_id", "source_type", "updated_at", "user_id") VALUES (?, ?, ?, ?, ?, ?) RETURNING "id""#,
                    params![now, event_type, source_id, source_type, now, user_id],
                    |r| r.get(0),
                )?;
                if announce {
                    Self::broadcast_change(tx, user_id, id)?;
                }
                Self::find(tx.conn(), id)?
            }
        };
        Ok(item)
    }

    /// `broadcast_activity_change`: to active humans only, on `ActivityChannel`'s stream, after
    /// commit. (Huddle items' invitation payload is WS13's.)
    pub(crate) fn broadcast_change(tx: &mut Tx<'_>, user_id: i64, id: i64) -> Result<()> {
        // These writers load and change the item within the same write transaction.
        let item = Self::find(tx.conn(), id)?;
        Self::broadcast_item(tx, user_id, &item)
    }

    fn broadcast_item(tx: &mut Tx<'_>, user_id: i64, item: &Self) -> Result<()> {
        if let Some(user) = User::find_by_id(tx.conn(), user_id)? {
            Self::broadcast_item_for_user(tx, &user, item)?;
        }
        Ok(())
    }

    fn broadcast_item_for_user(tx: &mut Tx<'_>, user: &User, item: &Self) -> Result<()> {
        if user.is_active() && !user.is_bot() {
            if crate::models::huddle_invitations::enqueue_item_ring(tx, item)? {
                return Ok(());
            }
            tx.emit_broadcast_once(
                "activity_items",
                item.id,
                &Broadcast::Cable {
                    stream: format!("user_{}_activity", user.id),
                    payload: serde_json::json!({ "activityItemId": item.id }),
                },
            );
        }
        Ok(())
    }

    /// `mark_handled!`: preserve an existing read timestamp when accepting a late invite.
    pub fn mark_handled(&self, tx: &mut Tx<'_>) -> Result<Self> {
        self.validate_event_type()?;
        let now = tx.now();
        self.save_state(tx, self.read_at.or(Some(now)), Some(now))
    }

    fn validate_event_type(&self) -> Result<()> {
        let mut errors = Errors::default();
        if !EVENT_TYPES.contains(&self.event_type.as_str()) {
            errors.add("event_type", "is not included in the list");
        }
        errors.into_result()
    }

    /// Approval settlement preserves an earlier read timestamp and broadcasts once
    /// for each newly handled item. The event sink runs only after commit.
    pub(crate) fn handle_for_source(
        tx: &mut Tx<'_>,
        source_type: &str,
        source_id: i64,
    ) -> Result<()> {
        let items = crate::sql::query_all(
            tx.conn(),
            "SELECT * FROM activity_items WHERE source_type=? AND source_id=? AND handled_at IS NULL ORDER BY id",
            params![source_type, source_id],
            Self::from_row,
        )?;
        for item in items {
            item.mark_handled(tx)?;
        }
        Ok(())
    }

    /// `has_many :activity_items, as: :source, dependent: :destroy`
    pub(crate) fn destroy_for_source(tx: &mut Tx<'_>, source_type: &str, source_id: i64) -> Result<()> {
        let removed = Self::delete_for_source(tx, source_type, source_id)?;
        Self::emit_removed(tx, removed);
        Ok(())
    }

    /// [`Self::destroy_for_source`] without telling anyone, for a Slack import's undo, which
    /// broadcasts nothing. Answers the `(id, user_id)` of the rows it deleted.
    pub(crate) fn delete_for_source(
        tx: &Tx<'_>,
        source_type: &str,
        source_id: i64,
    ) -> Result<Vec<(i64, i64)>> {
        crate::sql::query_all(
            tx.conn(),
            r#"DELETE FROM "activity_items" WHERE "source_type" = ? AND "source_id" = ? RETURNING "id", "user_id""#,
            params![source_type, source_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    }

    /// Tells the owners' other tabs that their items sourced in room `room_id` (on its messages
    /// and their saved items, work events, SLA nudges, huddle grants and events) are about to
    /// leave their reach: `user_id`'s on leaving it, everyone's when it's deleted. The rows stay,
    /// as in Rails (`ActivityItem.accessible_to` hides them, and shows them again if access
    /// returns). Call it before the memberships go.
    pub(crate) fn emit_hidden_in_room(tx: &mut Tx<'_>, room_id: i64, user_id: Option<i64>) -> Result<()> {
        let hidden = crate::sql::query_all(
            tx.conn(),
            "SELECT ai.id, ai.user_id FROM activity_items ai WHERE (?2 IS NULL OR ai.user_id = ?2) AND ( \
             (ai.source_type = 'Message' AND ai.source_id IN (SELECT id FROM messages WHERE room_id = ?1)) \
             OR (ai.source_type = 'SavedItem' AND ai.source_id IN (SELECT s.id FROM saved_items s JOIN messages m ON m.id = s.message_id WHERE m.room_id = ?1)) \
             OR (ai.source_type = 'WorkThreadEvent' AND ai.source_id IN (SELECT e.id FROM work_thread_events e JOIN channel_threads t ON t.id = e.channel_thread_id WHERE t.room_id = ?1)) \
             OR (ai.source_type = 'BoardSlaNudge' AND ai.source_id IN (SELECT id FROM board_sla_nudges WHERE room_id = ?1)) \
             OR (ai.source_type = 'HuddleGrant' AND ai.source_id IN (SELECT id FROM huddle_grants WHERE room_id = ?1)) \
             OR (ai.source_type = 'Event' AND ai.source_id IN (SELECT id FROM events WHERE room_id = ?1)))",
            params![room_id, user_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Self::emit_removed(tx, hidden);
        Ok(())
    }

    /// Tells the owners' other tabs that the items sourced on these rows are about to leave
    /// their reach because the rows are deleted without their items (a session, a two-step
    /// credential, an agent's budget notices): the items stay, as in Rails, but
    /// `ActivityItem.accessible_to` no longer finds them.
    pub(crate) fn emit_hidden_for_sources(tx: &mut Tx<'_>, source_type: &str, source_ids: &[i64]) -> Result<()> {
        if source_ids.is_empty() {
            return Ok(());
        }
        let hidden = crate::sql::query_all(
            tx.conn(),
            "SELECT id, user_id FROM activity_items WHERE source_type = ? AND source_id IN (SELECT value FROM json_each(?))",
            params![source_type, serde_json::json!(source_ids).to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Self::emit_removed(tx, hidden);
        Ok(())
    }

    /// Tells the owners' other tabs that these `(id, user_id)` items went with their source.
    pub(crate) fn emit_removed(tx: &mut Tx<'_>, items: Vec<(i64, i64)>) {
        if !items.is_empty() {
            tx.emit_after_commit(crate::Event::broadcast(&ActivityItemsRemoved { items, room_id: None }));
        }
    }
}
