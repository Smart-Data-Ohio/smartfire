//! app/services/activity_items/recorder.rb. Known sources authorize recipients;
//! the board opener caller may bypass the source check after authorizing its roster.
use crate::{ActivityItem, Error, Result, Tx, User, WorkThreadEvent};
use rusqlite::params;

#[derive(Debug, Clone, Copy)]
pub enum ActivitySource {
    Message(i64),
    WorkThreadEvent(i64),
    BoardSlaNudge(i64),
}

impl ActivityItem {
    /// Board creation has already authorized its roster and persisted this opener in
    /// the same transaction. Reuse those snapshots instead of reloading them per user.
    pub(crate) fn record_authorized_board_opener(
        tx: &mut Tx<'_>,
        user: &User,
        message: &crate::Message,
    ) -> Result<Option<Self>> {
        if !user.is_active() || user.is_bot() || message.creator_id == user.id {
            return Ok(None);
        }
        Self::record_authorized_with_recipient(
            tx,
            user.id,
            "Message",
            message.id,
            message.thread_id,
            "thread_activity",
            Some(user),
        )
    }

    pub fn record(
        tx: &mut Tx<'_>,
        user_id: i64,
        source: ActivitySource,
        event_type: &str,
        skip_source_check: bool,
    ) -> Result<Option<Self>> {
        if !super::EVENT_TYPES.contains(&event_type) {
            return Err(Error::Other(format!(
                "Unknown activity event type: {event_type}"
            )));
        }
        let Some(user) = User::find_by_id(tx.conn(), user_id)? else {
            return Ok(None);
        };
        if !user.is_active() || user.is_bot() {
            return Ok(None);
        }
        let (source_type, source_id, thread_id, allowed) = match source {
            ActivitySource::Message(id) => {
                let Some(message) = crate::Message::find_by_id(tx.conn(), id)? else {
                    return Ok(None);
                };
                if message.creator_id == user_id {
                    return Ok(None);
                }
                let allowed = skip_source_check
                    || super::message_recorder::candidates(
                        tx.conn(),
                        tx.rich_text(),
                        &message,
                        tx.now(),
                    )?
                    .recipients
                    .iter()
                    .any(|candidate| candidate.user_id == user_id);
                ("Message", id, message.thread_id, allowed)
            }
            ActivitySource::WorkThreadEvent(id) => {
                let Some(event) = WorkThreadEvent::find_by_id(tx.conn(), id)? else {
                    return Ok(None);
                };
                let allowed =
                    skip_source_check || event.recipient_user_ids(tx.conn())?.contains(&user_id);
                (
                    "WorkThreadEvent",
                    id,
                    Some(event.channel_thread_id),
                    allowed,
                )
            }
            ActivitySource::BoardSlaNudge(id) => {
                let Some(nudge) = crate::BoardSlaNudge::find_by_id(tx.conn(), id)? else {
                    return Ok(None);
                };
                let allowed = skip_source_check || nudge.activity_recipient_ids().contains(&user_id);
                ("BoardSlaNudge", id, None, allowed)
            }
        };
        if !allowed {
            return Ok(None);
        }
        Self::record_authorized(tx, user_id, source_type, source_id, thread_id, event_type)
    }

    /// WorkThreadEvent's fanout has already authorized and preloaded this recipient.
    pub(crate) fn record_authorized_work_event(
        tx: &mut Tx<'_>,
        user: &User,
        event: &WorkThreadEvent,
        event_type: &str,
    ) -> Result<Option<Self>> {
        if !super::EVENT_TYPES.contains(&event_type) {
            return Err(Error::Other(format!(
                "Unknown activity event type: {event_type}"
            )));
        }
        if !user.is_active() || user.is_bot() {
            return Ok(None);
        }
        Self::record_authorized(
            tx,
            user.id,
            "WorkThreadEvent",
            event.id,
            Some(event.channel_thread_id),
            event_type,
        )
    }

    fn record_authorized(
        tx: &mut Tx<'_>,
        user_id: i64,
        source_type: &str,
        source_id: i64,
        thread_id: Option<i64>,
        event_type: &str,
    ) -> Result<Option<Self>> {
        Self::record_authorized_with_recipient(
            tx,
            user_id,
            source_type,
            source_id,
            thread_id,
            event_type,
            None,
        )
    }

    fn record_authorized_with_recipient(
        tx: &mut Tx<'_>,
        user_id: i64,
        source_type: &str,
        source_id: i64,
        thread_id: Option<i64>,
        event_type: &str,
        recipient: Option<&User>,
    ) -> Result<Option<Self>> {
        if let Some(thread_id) =
            thread_id.filter(|_| matches!(event_type, "thread_activity" | "work_update"))
        {
            let grouped = crate::sql::query_one(
                tx.conn(),
                "SELECT * FROM activity_items WHERE user_id=? AND handled_at IS NULL AND event_type=? AND ((source_type='Message' AND source_id IN (SELECT id FROM messages WHERE thread_id=?)) OR (source_type='WorkThreadEvent' AND source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id=?))) ORDER BY updated_at DESC,id DESC LIMIT 1",
                params![user_id, event_type, thread_id, thread_id],
                Self::from_row,
            )?;
            if let Some(before) = grouped {
                if before.source_type != source_type
                    || before.source_id != source_id
                    || before.read_at.is_some()
                    || before.updated_at != tx.now()
                {
                    tx.conn().execute("UPDATE activity_items SET source_type=?,source_id=?,read_at=NULL,updated_at=? WHERE id=?",params![source_type,source_id,tx.now(),before.id])?;
                }
                let item = Self::find(tx.conn(), before.id)?;
                // app/models/activity_item.rb: source/updated_at changes alone do not broadcast.
                if before.read_at.is_some() {
                    Self::broadcast_recorded(tx, user_id, &item, recipient)?;
                }
                return Ok(Some(item));
            }
        }
        let inserted = tx.conn().execute("INSERT INTO activity_items(user_id,source_type,source_id,event_type,created_at,updated_at) VALUES (?,?,?,?,?,?) ON CONFLICT(user_id,source_type,source_id) DO NOTHING",params![user_id,source_type,source_id,event_type,tx.now(),tx.now()])?;
        let item = Self::find_by_user_and_source(tx.conn(), user_id, source_type, source_id)?
            .ok_or(Error::RecordNotFound("ActivityItem"))?;
        if inserted == 1 {
            Self::broadcast_recorded(tx, user_id, &item, recipient)?;
        }
        Ok(Some(item))
    }

    fn broadcast_recorded(
        tx: &mut Tx<'_>,
        user_id: i64,
        item: &Self,
        recipient: Option<&User>,
    ) -> Result<()> {
        match recipient {
            Some(user) => Self::broadcast_item_for_user(tx, user, item),
            None => Self::broadcast_item(tx, user_id, item),
        }
    }
}
