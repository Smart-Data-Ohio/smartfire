//! `app/models/scheduled_message.rb` and `scheduled_message/dispatcher.rb`.
//! Dispatch one row per write transaction. The claim, post and history commit together; stale
//! claims written by Rails are reclaimable after five minutes. The scheduler belongs to WS3.

use jiff::SignedDuration;
use rusqlite::{Connection, Row, params};

use crate::broadcasts::{Broadcast, Partial, conversation_messages, dom_id, room_dom_id};
use crate::error::OptionalExt;
use crate::models::message::SOURCE_LIMIT;
use crate::sql::{CachedStatements, query_all, query_one};
use crate::{
    ActivityItem, ChannelThread, Database, Errors, Event, Membership, Message, NewMessage, Result,
    Room, Timestamp, Tx, User,
};

pub const STALE_CLAIM_AFTER: SignedDuration = SignedDuration::from_mins(5);

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledMessage {
    pub id: i64,
    pub user_id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
    pub reply_to_message_id: Option<i64>,
    pub sent_message_id: Option<i64>,
    pub markdown_source: String,
    pub send_at: Timestamp,
    pub claimed_at: Option<Timestamp>,
    pub sent_at: Option<Timestamp>,
    pub dropped_at: Option<Timestamp>,
    pub drop_reason: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone)]
pub struct NewScheduledMessage {
    pub user_id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
    pub reply_to_message_id: Option<i64>,
    pub markdown_source: String,
    pub send_at: Timestamp,
}

impl ScheduledMessage {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            room_id: row.get("room_id")?,
            thread_id: row.get("thread_id")?,
            reply_to_message_id: row.get("reply_to_message_id")?,
            sent_message_id: row.get("sent_message_id")?,
            markdown_source: row.get("markdown_source")?,
            send_at: row.get("send_at")?,
            claimed_at: row.get("claimed_at")?,
            sent_at: row.get("sent_at")?,
            dropped_at: row.get("dropped_at")?,
            drop_reason: row.get("drop_reason")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        Self::find_by_id(conn, id)?.or_not_found("ScheduledMessage")
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM scheduled_messages WHERE id = ?",
            [id],
            Self::from_row,
        )
    }

    /// `owned_by(user).pending.ordered` or `.past`: ordering and history are independent of
    /// access, which callers check through `sendable` when presenting rows.
    pub fn owned_by(conn: &Connection, user_id: i64, past: bool) -> Result<Vec<Self>> {
        let (condition, order) = if past {
            (
                "(sent_at IS NOT NULL OR dropped_at IS NOT NULL)",
                "send_at DESC, id DESC",
            )
        } else {
            (
                "sent_at IS NULL AND dropped_at IS NULL",
                "send_at ASC, id ASC",
            )
        };
        query_all(
            conn,
            &format!(
                "SELECT * FROM scheduled_messages WHERE user_id = ? AND {condition} ORDER BY {order}"
            ),
            [user_id],
            Self::from_row,
        )
    }

    pub fn pending(&self) -> bool {
        self.sent_at.is_none() && self.dropped_at.is_none()
    }
    pub fn sent(&self) -> bool {
        self.sent_at.is_some()
    }
    pub fn dropped(&self) -> bool {
        self.dropped_at.is_some()
    }
    pub fn claimed(&self, now: Timestamp) -> bool {
        self.pending()
            && self
                .claimed_at
                .is_some_and(|at| at >= now.ago(STALE_CLAIM_AFTER))
    }

    /// Required user/room; optional conversation associations; body presence/character limit;
    /// future send time only when changed. Scheduled replies require the *same* stream, unlike
    /// Message's exception for replying to a thread's parent root message.
    pub fn validate(
        conn: &Connection,
        attrs: &NewScheduledMessage,
        now: Timestamp,
        send_at_changed: bool,
    ) -> Result<Errors> {
        let mut errors = Errors::default();
        if User::find_by_id(conn, attrs.user_id)?.is_none() {
            errors.add("user", "must exist");
        }
        if Room::find_by_id(conn, attrs.room_id)?.is_none() {
            errors.add("room", "must exist");
        }
        if attrs.markdown_source.trim().is_empty() {
            errors.add("markdown_source", "can't be blank");
        }
        if attrs.markdown_source.chars().count() > SOURCE_LIMIT {
            errors.add(
                "markdown_source",
                format!("is too long (maximum is {SOURCE_LIMIT} characters)"),
            );
        }
        if send_at_changed && attrs.send_at <= now {
            errors.add("send_at", "must be in the future");
        }
        if let Some(thread) = attrs
            .thread_id
            .map(|id| ChannelThread::find_by_id(conn, id))
            .transpose()?
            .flatten()
            && thread.room_id != attrs.room_id
        {
            errors.add("thread", "must belong to the scheduled room");
        }
        if let Some(source) = attrs
            .reply_to_message_id
            .map(|id| Message::find_by_id(conn, id))
            .transpose()?
            .flatten()
            && (source.room_id != attrs.room_id || source.thread_id != attrs.thread_id)
        {
            errors.add("reply_to_message", "must be in the same conversation");
        }
        Ok(errors)
    }

    pub fn create(tx: &mut Tx<'_>, attrs: NewScheduledMessage) -> Result<Self> {
        Self::validate(tx.conn(), &attrs, tx.now(), true)?.into_result()?;
        let id = tx.conn().query_row_cached(
            "INSERT INTO scheduled_messages (user_id, room_id, thread_id, reply_to_message_id, markdown_source, send_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
            params![attrs.user_id, attrs.room_id, attrs.thread_id, attrs.reply_to_message_id, attrs.markdown_source, attrs.send_at, tx.now(), tx.now()], |r| r.get(0))?;
        Self::find(tx.conn(), id)
    }

    /// The draft's editable fields. Ownership and the controller's refusal to edit/cancel a
    /// live claim are WS8b's; `claimed` is provided for that guard.
    pub fn update(&mut self, tx: &mut Tx<'_>, source: &str, send_at: Timestamp) -> Result<()> {
        let attrs = NewScheduledMessage {
            user_id: self.user_id,
            room_id: self.room_id,
            thread_id: self.thread_id,
            reply_to_message_id: self.reply_to_message_id,
            markdown_source: source.to_string(),
            send_at,
        };
        Self::validate(tx.conn(), &attrs, tx.now(), send_at != self.send_at)?.into_result()?;
        if source != self.markdown_source || send_at != self.send_at {
            tx.conn().execute_cached("UPDATE scheduled_messages SET markdown_source = ?, send_at = ?, updated_at = ? WHERE id = ?",
                params![source, send_at, tx.now(), self.id])?;
            *self = Self::find(tx.conn(), self.id)?;
        }
        Ok(())
    }

    pub fn sendable(&self, conn: &Connection) -> Result<bool> {
        let Some(user) = User::find_by_id(conn, self.user_id)? else {
            return Ok(false);
        };
        let Some(room) = Room::find_by_id(conn, self.room_id)? else {
            return Ok(false);
        };
        if !user.is_active()
            || user.is_bot()
            || room.deleted()
            || Membership::find_by_room_and_user(conn, self.room_id, self.user_id)?.is_none()
        {
            return Ok(false);
        }
        if let Some(thread) = self
            .thread_id
            .map(|id| ChannelThread::find_by_id(conn, id))
            .transpose()?
            .flatten()
            && thread.room_id != self.room_id
        {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn drop(&mut self, tx: &mut Tx<'_>, reason: Option<&str>, now: Timestamp) -> Result<()> {
        tx.conn().execute_cached("UPDATE scheduled_messages SET dropped_at = ?, drop_reason = ?, updated_at = ? WHERE id = ?",
            params![now, reason, tx.now(), self.id])?;
        ActivityItem::refresh_unread(
            tx,
            self.user_id,
            "ScheduledMessage",
            self.id,
            "scheduled_message_dropped",
        )?;
        *self = Self::find(tx.conn(), self.id)?;
        Ok(())
    }

    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        ActivityItem::destroy_for_source(tx, "ScheduledMessage", self.id)?;
        tx.conn()
            .execute_cached("DELETE FROM scheduled_messages WHERE id = ?", [self.id])?;
        Ok(())
    }

    /// Prepend to thread destruction, before the FK clears the stream identity. Sent history
    /// stays sent; pending rows become dropped history and retain their private inbox source.
    pub(crate) fn drop_for_thread(tx: &mut Tx<'_>, thread_id: i64) -> Result<()> {
        let pending = query_all(
            tx.conn(),
            "SELECT * FROM scheduled_messages WHERE thread_id = ? AND sent_at IS NULL AND dropped_at IS NULL ORDER BY id",
            [thread_id],
            Self::from_row,
        )?;
        for mut row in pending {
            row.drop(tx, Some("its thread was deleted"), tx.now())?;
        }
        Ok(())
    }

    /// `due_candidates`: includes soft-deleted rooms so stranded drafts drop, but excludes
    /// inactive/bot authors. A live claim at the exact five-minute boundary is still held.
    pub fn due_candidate_ids(conn: &Connection, now: Timestamp) -> Result<Vec<i64>> {
        query_all(
            conn,
            "SELECT s.id FROM scheduled_messages s JOIN users u ON u.id = s.user_id JOIN rooms r ON r.id = s.room_id WHERE s.sent_at IS NULL AND s.dropped_at IS NULL AND s.send_at <= ? AND (s.claimed_at IS NULL OR s.claimed_at < ?) AND u.status = 0 AND u.role != 2 ORDER BY s.id",
            params![now, now.ago(STALE_CLAIM_AFTER)],
            |r| r.get(0),
        )
    }

    /// `dispatch_item!` / `dispatch_now!`. One real SQLite transaction is the critical section:
    /// a failure rolls back the claim, post and history, leaving the next tick able to retry.
    pub fn dispatch(tx: &mut Tx<'_>, id: i64, now: Timestamp, immediate: bool) -> Result<bool> {
        let count = tx.conn().execute_cached(
            "UPDATE scheduled_messages SET claimed_at = ?, updated_at = ? WHERE id = ? AND sent_at IS NULL AND dropped_at IS NULL AND (claimed_at IS NULL OR claimed_at < ?) AND (? OR send_at <= ?)",
            params![now, now, id, now.ago(STALE_CLAIM_AFTER), immediate, now])?;
        if count != 1 {
            return Ok(false);
        }
        let mut scheduled = Self::find(tx.conn(), id)?;
        if !scheduled.sendable(tx.conn())? {
            let reason = Room::find_by_id(tx.conn(), scheduled.room_id)?
                .filter(Room::deleted)
                .map(|_| "its room was deleted");
            scheduled.drop(tx, reason, now)?;
            return Ok(false);
        }
        let thread = scheduled
            .thread_id
            .map(|id| ChannelThread::find(tx.conn(), id))
            .transpose()?;
        if thread.as_ref().is_some_and(|t| t.locked_at.is_some()) {
            tx.conn().execute_cached(
                "UPDATE scheduled_messages SET claimed_at = NULL WHERE id = ?",
                [id],
            )?;
            return Ok(false);
        }
        let reply_id = scheduled
            .reply_to_message_id
            .map(|id| Message::find_by_id(tx.conn(), id))
            .transpose()?
            .flatten()
            .map(|message| message.id);
        let attrs = NewMessage {
            room_id: scheduled.room_id,
            creator_id: scheduled.user_id,
            thread_id: scheduled.thread_id,
            reply_to_message_id: reply_id,
            markdown_source: Some(scheduled.markdown_source.clone()),
            ..Default::default()
        };
        // Validate before thread joins/reopening mutate state. SQL/rendering failures propagate
        // and roll the transaction back rather than converting transient errors into drops.
        let errors = Message::validate(tx.conn(), &attrs)?;
        if !errors.is_empty() {
            scheduled.drop(tx, Some(&errors.full_messages().join(", ")), now)?;
            return Ok(false);
        }
        let message = match thread {
            Some(mut thread) => thread.post_message(tx, scheduled.user_id, attrs)?,
            None => Message::create(tx, attrs)?,
        };
        tx.conn().execute_cached("UPDATE scheduled_messages SET sent_at = ?, sent_message_id = ?, updated_at = ? WHERE id = ?",
            params![now, message.id, tx.now(), id])?;
        let room = Room::find(tx.conn(), message.room_id)?;
        let target = match message.thread_id {
            Some(thread) => dom_id("channel_thread", thread, Some("messages")),
            None => room_dom_id(&room, Some("messages")),
        };
        tx.emit_after_commit(Event::broadcast(&Broadcast::append(
            conversation_messages(tx.conn(), &message)?,
            target,
            Partial::Message {
                message_id: message.id,
            },
        )));
        if message.thread_id.is_none() {
            let mentioned_ids: Vec<_> = message
                .mentionees(tx.conn(), tx.rich_text())?
                .iter()
                .map(|user| user.id)
                .collect();
            for member in Membership::for_room(tx.conn(), message.room_id)? {
                if member.involvement != Some(crate::Involvement::Muted)
                    || mentioned_ids.contains(&member.user_id)
                {
                    tx.emit_after_commit(Event::broadcast(&Broadcast::Cable {
                        stream: crate::broadcasts::unread_rooms_stream_name(member.user_id),
                        payload: serde_json::json!({ "roomId": message.room_id }),
                    }));
                }
            }
        }
        // Scheduled messages have no attachment column. process_attachment therefore has no
        // work here. Human root posts fan out to legacy bots; thread posts never do.
        crate::models::bot_webhook_fanout::deliver(tx, &message)?;
        Ok(true)
    }

    /// Per-row error isolation, ready for WS3's periodic task to call in a blocking worker.
    pub fn dispatch_due(db: &Database, now: Timestamp) -> Result<Vec<i64>> {
        let ids = db.read_blocking(|conn| Self::due_candidate_ids(conn, now))?;
        let mut sent = Vec::new();
        for id in ids {
            match db.write_blocking(move |tx| Self::dispatch(tx, id, now, false)) {
                Ok(true) => sent.push(id),
                Ok(false) => {}
                Err(error) => tracing::error!(id, %error, "Scheduled message failed"),
            }
        }
        Ok(sent)
    }
}
