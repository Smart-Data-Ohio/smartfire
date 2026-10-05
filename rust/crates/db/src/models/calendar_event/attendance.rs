//! app/models/event_attendance.rb and Event#respond!.
use super::{CalendarEvent, SyncEntryJob, member};
use crate::sql::{query_all, query_one};
use crate::{Errors, Result, Timestamp, Tx, User};
use rusqlite::{Connection, Row, params};

pub const RESPONSES: [&str; 3] = ["going", "maybe", "declined"];
#[derive(Debug, Clone, PartialEq)]
pub struct EventAttendance {
    pub id: i64,
    pub event_id: i64,
    pub user_id: i64,
    pub response: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
impl EventAttendance {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            event_id: row.get("event_id")?,
            user_id: row.get("user_id")?,
            response: row.get("response")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find_for(conn: &Connection, event_id: i64, user_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM event_attendances WHERE event_id=? AND user_id=?",
            params![event_id, user_id],
            Self::from_row,
        )
    }
    /// EventAttendance#create!: unlike Event#respond!, creation rejects an
    /// existing attendance instead of updating that member's response.
    pub fn create(tx: &mut Tx<'_>, event_id: i64, user_id: i64, response: &str) -> Result<Self> {
        tx.savepoint(|tx| {
            if Self::find_for(tx.conn(), event_id, user_id)?.is_some() {
                let mut errors = Errors::default();
                errors.add("user_id", "has already been taken");
                errors.into_result()?;
            }
            let event = CalendarEvent::find(tx.conn(), event_id)?;
            Self::save_response(tx, &event, user_id, response, true)
        })
    }
    /// EventAttendance#update!: unrelated timestamp saves run validation but
    /// enqueue calendar synchronization only when the response changes.
    pub fn update(
        tx: &mut Tx<'_>,
        id: i64,
        response: Option<&str>,
        updated_at: Option<Timestamp>,
    ) -> Result<Self> {
        tx.savepoint(|tx| {
            let previous = query_one(
                tx.conn(),
                "SELECT * FROM event_attendances WHERE id=?",
                [id],
                Self::from_row,
            )?
            .ok_or(crate::Error::RecordNotFound("EventAttendance"))?;
            let event = CalendarEvent::find(tx.conn(), previous.event_id)?;
            let saved = Self::save_response(
                tx,
                &event,
                previous.user_id,
                response.unwrap_or(&previous.response),
                true,
            )?;
            if let Some(stamp) = updated_at {
                tx.conn().execute(
                    "UPDATE event_attendances SET updated_at=? WHERE id=?",
                    params![stamp, saved.id],
                )?;
            }
            Ok(
                Self::find_for(tx.conn(), saved.event_id, saved.user_id)?
                    .expect("saved attendance"),
            )
        })
    }
    pub fn for_event(conn: &Connection, event_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM event_attendances WHERE event_id=? ORDER BY response,id",
            [event_id],
            Self::from_row,
        )
    }
    pub(super) fn respond_one(
        tx: &mut Tx<'_>,
        event: &CalendarEvent,
        user_id: i64,
        response: &str,
    ) -> Result<Self> {
        Self::save_response(tx, event, user_id, response, true)
    }
    pub(super) fn record_organizer(tx: &mut Tx<'_>, event: &CalendarEvent) -> Result<Self> {
        Self::save_response(tx, event, event.organizer_id, "going", false)
    }
    pub(super) fn save_response(
        tx: &mut Tx<'_>,
        event: &CalendarEvent,
        user_id: i64,
        response: &str,
        enqueue_sync: bool,
    ) -> Result<Self> {
        if !RESPONSES.contains(&response) {
            return Err(crate::Error::Other(format!(
                "'{response}' is not a valid response"
            )));
        }
        let mut errors = Errors::default();
        let user = User::find_by_id(tx.conn(), user_id)?;
        if user.is_none() {
            errors.add("user", "must exist");
        }
        if event.cancelled() {
            errors.add("event", "is cancelled");
        }
        if !user.as_ref().is_some_and(|u| u.is_active() && !u.is_bot()) {
            errors.add("user", "must be an active human");
        }
        if user.is_some() && !member(tx.conn(), event.room_id, user_id)? {
            errors.add("user", "must be a member of the event room");
        }
        errors.into_result()?;
        let previous = Self::find_for(tx.conn(), event.id, user_id)?;
        let now = tx.now();
        match &previous {
            Some(a) if a.response != response => {
                tx.conn().execute(
                    "UPDATE event_attendances SET response=?,updated_at=? WHERE id=?",
                    params![response, now, a.id],
                )?;
            }
            Some(_) => (),
            None => {
                tx.conn().execute("INSERT INTO event_attendances (event_id,user_id,response,created_at,updated_at) VALUES (?,?,?,?,?)",params![event.id,user_id,response,now,now])?;
            }
        }
        let saved = Self::find_for(tx.conn(), event.id, user_id)?.expect("saved attendance");
        if enqueue_sync && previous.is_none_or(|a| a.response != response) {
            tx.emit_record_job_once(
                "event_attendances",
                saved.id,
                &SyncEntryJob {
                    event_id: event.id,
                    user_id,
                },
            );
        }
        Ok(saved)
    }
}
impl CalendarEvent {
    pub fn response_for(&self, conn: &Connection, user_id: Option<i64>) -> Result<Option<String>> {
        let Some(user_id) = user_id else {
            return Ok(None);
        };
        Ok(EventAttendance::find_for(conn, self.id, user_id)?.map(|a| a.response))
    }
    pub fn attendance_counts(
        &self,
        conn: &Connection,
    ) -> Result<std::collections::BTreeMap<String, i64>> {
        Ok(query_all(
            conn,
            "SELECT response,COUNT(*) FROM event_attendances WHERE event_id=? GROUP BY response",
            [self.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?
        .into_iter()
        .collect())
    }
    /// Event#respond!: a head response applies to all active future occurrences;
    /// a follower stays local unless `apply_to_future`. Returns that occurrence's
    /// attendance. Changed attendances enqueue SyncEntry once per record/commit;
    /// queue rows commit atomically and publication waits for the writer commit.
    pub fn respond(
        tx: &mut Tx<'_>,
        event_id: i64,
        user_id: i64,
        response: &str,
        apply_to_future: bool,
    ) -> Result<EventAttendance> {
        tx.savepoint(|tx| {
            let event = Self::find(tx.conn(), event_id)?;
            let attendance = EventAttendance::respond_one(tx, &event, user_id, response)?;
            if event.series() && (event.series_head() || apply_to_future) {
                for follower in event.future_occurrences(tx.conn())? {
                    if !follower.cancelled() {
                        EventAttendance::respond_one(tx, &follower, user_id, response)?;
                    }
                }
            }
            Ok(attendance)
        })
    }
}
