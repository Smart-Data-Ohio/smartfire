//! Event::ReminderPusher source facts. WS17 owns batch notification policy and transport.
//! Its notification_push::event_reminder_push consumes the identical event ID job.
use super::CalendarEvent;
use crate::sql::query_all;
use crate::{Result, Room, Timestamp, User};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderPayload {
    pub title: String,
    pub body: String,
    pub path: String,
    pub tag: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderPushSource {
    pub payload: ReminderPayload,
    /// Going/maybe, active human, current members. WS17 applies Reminder policy to
    /// this whole batch; there is no sender allowance and no inbox/involvement gate.
    pub recipient_ids: Vec<i64>,
}
impl CalendarEvent {
    pub fn reminder_push_source(
        conn: &Connection,
        id: i64,
        now: Timestamp,
    ) -> Result<Option<ReminderPushSource>> {
        let event = Self::find(conn, id)?;
        if event.reminder_stale(now) {
            return Ok(None);
        }
        let room = Room::find(conn, event.room_id)?;
        let title = if room.direct() {
            User::find(conn, event.organizer_id)?.display_name().to_owned()
        } else {
            room.name.unwrap_or_default()
        };
        let minutes = ((event.starts_at.as_microsecond() - now.as_microsecond()) as f64
            / 60_000_000.0)
            .round() as i64;
        let relative = if minutes <= 0 {
            "Starting now".to_string()
        } else {
            format!(
                "Starts in {minutes} {}",
                if minutes == 1 { "minute" } else { "minutes" }
            )
        };
        let mut body = format!("{relative}: {}", event.title);
        if let Some(venue) = event
            .venue_room_id
            .map(|id| Room::find_by_id(conn, id))
            .transpose()?
            .flatten()
        {
            body.push_str(&format!(" in {}", venue.name.unwrap_or_default()));
        }
        let recipient_ids = query_all(
            conn,
            "SELECT u.id FROM users u JOIN event_attendances a ON a.user_id=u.id JOIN memberships m ON m.user_id=u.id AND m.room_id=? WHERE a.event_id=? AND a.response IN ('going','maybe') AND u.status=0 AND u.role<>2 ORDER BY u.id",
            params![event.room_id, event.id],
            |r| r.get(0),
        )?;
        Ok(Some(ReminderPushSource {
            payload: ReminderPayload {
                title,
                body,
                path: format!("/rooms/{}/events/{}", event.room_id, event.id),
                tag: format!("event-{}", event.id),
            },
            recipient_ids,
        }))
    }
}
