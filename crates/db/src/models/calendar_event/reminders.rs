//! app/models/event/reminder_dispatcher.rb. Each event is dispatched in a writer transaction;
//! SQLite BEGIN IMMEDIATE supplies the row-lock equivalent across database handles.
use super::CalendarEvent;
use crate::sql::query_all;
use crate::{ActivityItem, Event, Job, Result, Timestamp, Tx};
use jiff::SignedDuration;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderPushJob {
    pub event_id: i64,
}
impl Job for ReminderPushJob {
    const CLASS: &'static str = "Event::ReminderPushJob";
}
impl CalendarEvent {
    pub fn reminder_stale(&self, now: Timestamp) -> bool {
        self.ends_at.is_some_and(|end| end <= now)
            || self.starts_at < now.ago(SignedDuration::from_mins(5))
    }
    pub fn due_reminder_ids(conn: &Connection, now: Timestamp) -> Result<Vec<i64>> {
        query_all(
            conn,
            "SELECT e.id FROM events e JOIN rooms r ON r.id=e.room_id WHERE e.cancelled_at IS NULL \
            AND e.reminded_at IS NULL AND r.deleted_at IS NULL AND e.starts_at>=? AND e.starts_at<=? ORDER BY e.id",
            params![
                now.ago(SignedDuration::from_mins(60)),
                now.since(SignedDuration::from_mins(15))
            ],
            |r| r.get(0),
        )
    }
    pub fn dispatch_reminder(tx: &mut Tx<'_>, id: i64, now: Timestamp) -> Result<bool> {
        tx.savepoint(|tx| {
            let event = Self::find(tx.conn(), id)?;
            if event.cancelled()
                || event.reminded_at.is_some()
                || crate::Room::find(tx.conn(), event.room_id)?
                    .deleted_at
                    .is_some()
            {
                return Ok(false);
            }
            // Rails event.update! runs unchanged-field validations on the claim.
            event.validate_existing(tx.conn())?.into_result()?;
            let stale = event.reminder_stale(now);
            if !stale {
                for user_id in event.notification_recipient_ids(tx.conn())? {
                    let raw: Option<String> = tx.conn().query_row(
                        "SELECT inbox_preferences FROM users WHERE id=?",
                        [user_id],
                        |r| r.get(0),
                    )?;
                    let preferences: serde_json::Value = raw
                        .as_deref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default();
                    if reminders_enabled(&preferences) {
                        ActivityItem::refresh_unread(tx, user_id, "Event", id, "event_reminder")?;
                    }
                }
            }
            tx.conn().execute(
                "UPDATE events SET reminded_at=?,updated_at=? WHERE id=?",
                params![now, tx.now(), id],
            )?;
            event.update_callbacks(tx)?;
            if !stale {
                tx.emit_after_commit(Event::job(&ReminderPushJob { event_id: id }));
            }
            Ok(!stale)
        })
    }
}
fn reminders_enabled(raw: &serde_json::Value) -> bool {
    // User::InboxPreferences casts only four false values; unknown values default true.
    match raw.get("event_reminders") {
        Some(serde_json::Value::Bool(false)) => false,
        Some(serde_json::Value::Number(n)) if n.as_f64() == Some(0.0) => false,
        Some(serde_json::Value::String(s)) if s == "0" || s == "false" => false,
        _ => true,
    }
}
