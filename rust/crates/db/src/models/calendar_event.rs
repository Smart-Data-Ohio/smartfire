//! Event domain; named CalendarEvent to avoid the existing database side-effect Event.
pub mod recurrence;

use jiff::civil::Date;
use rusqlite::{Connection, Row, params};
use serde::{Deserialize, Serialize};

use crate::error::OptionalExt;
use crate::slash_commands::time_parser::known_zone;
use crate::sql::{exists, query_all, query_one};
use crate::{
    ActivityItem, Errors, Event as SideEffect, Job, Result, Room, RoomType, Timestamp, Tx, User,
};

pub mod attendance;
pub mod changes;
pub mod pusher;
pub mod references;
pub mod reminders;

#[derive(Debug, Clone, PartialEq)]
pub struct CalendarEvent {
    pub id: i64,
    pub room_id: i64,
    pub organizer_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub starts_at: Timestamp,
    pub ends_at: Option<Timestamp>,
    pub time_zone: String,
    pub venue_room_id: Option<i64>,
    pub series_id: Option<i64>,
    pub recurrence_rule: Option<String>,
    pub recurrence_until: Option<Date>,
    pub meet_link_requested: bool,
    pub meet_link: Option<String>,
    pub cancelled_at: Option<Timestamp>,
    pub reminded_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Nullable required fields permit the same validation errors as Rails' unsaved model.
#[derive(Debug, Clone, Default)]
pub struct NewCalendarEvent {
    pub room_id: i64,
    pub organizer_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub starts_at: Option<Timestamp>,
    pub ends_at: Option<Timestamp>,
    pub time_zone: String,
    pub venue_room_id: Option<i64>,
    pub recurrence_rule: Option<String>,
    pub recurrence_until: Option<Date>,
    pub meet_link_requested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEntryJob {
    pub event_id: i64,
    pub user_id: i64,
}
impl Job for SyncEntryJob {
    const CLASS: &'static str = "Calendar::SyncEntryJob";
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetLinkJob {
    pub event_id: i64,
}
impl Job for MeetLinkJob {
    const CLASS: &'static str = "Calendar::MeetLinkJob";
}
pub use crate::models::room_delete::RemoteDeleteJob;

impl CalendarEvent {
    pub(super) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let date: Option<String> = row.get("recurrence_until")?;
        let recurrence_until = date
            .map(|s| {
                s.parse::<Date>().map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })
            })
            .transpose()?;
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            organizer_id: row.get("organizer_id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            starts_at: row.get("starts_at")?,
            ends_at: row.get("ends_at")?,
            time_zone: row.get("time_zone")?,
            venue_room_id: row.get("venue_room_id")?,
            series_id: row.get("series_id")?,
            recurrence_rule: row.get("recurrence_rule")?,
            recurrence_until,
            meet_link_requested: row.get("meet_link_requested")?,
            meet_link: row.get("meet_link")?,
            cancelled_at: row.get("cancelled_at")?,
            reminded_at: row.get("reminded_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(
            conn,
            "SELECT * FROM events WHERE id=?",
            [id],
            Self::from_row,
        )?
        .or_not_found("Event")
    }
    /// Message-card associations, ordered like Event.soonest_first and restricted
    /// to events in the referring message's room.
    pub fn for_message_ids(
        conn: &Connection,
        ids: &[i64],
    ) -> Result<std::collections::HashMap<i64, Vec<Self>>> {
        let mut result = std::collections::HashMap::<i64, Vec<Self>>::new();
        if ids.is_empty() {
            return Ok(result);
        }
        let sql = format!(
            "SELECT r.message_id,e.* FROM events e JOIN event_references r ON r.event_id=e.id JOIN messages m ON m.id=r.message_id AND m.room_id=e.room_id WHERE r.message_id IN ({}) ORDER BY e.starts_at,e.id",
            crate::sql::placeholders(ids.len())
        );
        for row in conn
            .prepare(&sql)?
            .query_map(rusqlite::params_from_iter(ids), |row| {
                Ok((row.get::<_, i64>("message_id")?, Self::from_row(row)?))
            })?
        {
            let (message, event) = row?;
            result.entry(message).or_default().push(event);
        }
        Ok(result)
    }
    /// The controller's RoomScoped lookup. Always require membership, even for administrators
    /// and open rooms: direct event URLs do not join a room.
    pub fn find_visible(conn: &Connection, room_id: i64, id: i64, user_id: i64) -> Result<Self> {
        query_one(conn, "SELECT e.* FROM events e JOIN rooms r ON r.id=e.room_id \
            JOIN memberships m ON m.room_id=r.id WHERE e.id=? AND e.room_id=? AND m.user_id=? AND r.deleted_at IS NULL",
            params![id, room_id, user_id], Self::from_row)?.or_not_found("Event")
    }
    pub fn cancelled(&self) -> bool {
        self.cancelled_at.is_some()
    }
    pub fn series(&self) -> bool {
        self.series_id.is_some()
    }
    pub fn series_head(&self) -> bool {
        self.series_id == Some(self.id)
    }
    pub fn needs_meet_link(&self) -> bool {
        self.meet_link_requested
            && self
                .meet_link
                .as_deref()
                .is_none_or(campfire_richtext::ruby::is_blank)
            && !self.cancelled()
    }
    pub fn cancellable_by(&self, user: Option<&User>) -> bool {
        user.is_some_and(|u| {
            u.is_active() && !u.is_bot() && (u.id == self.organizer_id || u.is_administrator())
        })
    }
    pub fn manageable_by(&self, user: Option<&User>) -> bool {
        !self.cancelled() && self.cancellable_by(user)
    }
    pub fn respondable_by(&self, conn: &Connection, user: Option<&User>) -> Result<bool> {
        let Some(user) = user.filter(|u| u.is_active() && !u.is_bot()) else {
            return Ok(false);
        };
        Ok(!self.cancelled() && member(conn, self.room_id, user.id)?)
    }
    pub fn series_events(&self, conn: &Connection) -> Result<Vec<Self>> {
        match self.series_id {
            Some(series_id) => query_all(
                conn,
                "SELECT * FROM events WHERE series_id=? ORDER BY starts_at, cancelled_at ASC NULLS FIRST, id",
                [series_id],
                Self::from_row,
            ),
            None => Ok(vec![Self::find(conn, self.id)?]),
        }
    }
    pub fn future_occurrences(&self, conn: &Connection) -> Result<Vec<Self>> {
        if !self.series() {
            return Ok(Vec::new());
        }
        Ok(self
            .series_events(conn)?
            .into_iter()
            .filter(|e| (e.starts_at, e.id) > (self.starts_at, self.id))
            .collect())
    }
    pub fn previous_occurrence(&self, conn: &Connection) -> Result<Option<Self>> {
        if !self.series() {
            return Ok(None);
        }
        Ok(self
            .series_events(conn)?
            .into_iter()
            .rfind(|e| (e.starts_at, e.id) < (self.starts_at, self.id)))
    }
    pub fn next_occurrence(&self, conn: &Connection) -> Result<Option<Self>> {
        Ok(self.future_occurrences(conn)?.into_iter().next())
    }

    /// All create validations from app/models/event.rb. Series ids cannot be supplied by
    /// callers; only materialization can construct a follower.
    pub fn validate(conn: &Connection, a: &NewCalendarEvent) -> Result<Errors> {
        Self::validate_fields(conn, a, true, true)
    }
    fn validate_fields(
        conn: &Connection,
        a: &NewCalendarEvent,
        venue_changed: bool,
        range: bool,
    ) -> Result<Errors> {
        let mut errors = Errors::default();
        if Room::find_by_id(conn, a.room_id)?.is_none() {
            errors.add("room", "must exist");
        }
        if User::find_by_id(conn, a.organizer_id)?.is_none() {
            errors.add("organizer", "must exist");
        }
        if campfire_richtext::ruby::is_blank(&a.title) {
            errors.add("title", "can't be blank");
        }
        if a.starts_at.is_none() {
            errors.add("starts_at", "can't be blank");
        }
        if campfire_richtext::ruby::is_blank(&a.time_zone) {
            errors.add("time_zone", "can't be blank");
        }
        let rule = a
            .recurrence_rule
            .as_deref()
            .filter(|s| !campfire_richtext::ruby::is_blank(s));
        if rule.is_some_and(|s| !recurrence::RULES.contains(&s)) {
            errors.add("recurrence_rule", "is not included in the list");
        }
        let zone = known_zone(&a.time_zone);
        if !campfire_richtext::ruby::is_blank(&a.time_zone) && zone.is_none() {
            errors.add("time_zone", "is invalid");
        }
        if let (Some(start), Some(end)) = (a.starts_at, a.ends_at)
            && end <= start
        {
            errors.add("ends_at", "must be after the start time");
        }
        if !User::find_by_id(conn, a.organizer_id)?.is_some_and(|u| u.is_active() && !u.is_bot())
            || !member(conn, a.room_id, a.organizer_id)?
        {
            errors.add("organizer", "must be an active human member of the room");
        }
        if let Some(id) = a.venue_room_id.filter(|_| venue_changed) {
            let valid = Room::find_by_id(conn, id)?.is_some_and(|r| {
                r.deleted_at.is_none() && matches!(r.room_type, RoomType::Voice | RoomType::Stage)
            });
            if !valid || !member(conn, id, a.organizer_id)? {
                errors.add("venue", "must be a voice or Stage channel you belong to");
            }
        }
        if let Some(rule) = rule.filter(|_| range) {
            if a.recurrence_until.is_none() {
                errors.add("recurrence_until", "can't be blank");
            }
            if let (Some(start), Some(until), Some(zone)) = (a.starts_at, a.recurrence_until, zone)
            {
                let date = start.jiff().to_zoned(zone).date();
                if until <= date {
                    errors.add("recurrence_until", "must be after the start date");
                } else if date
                    .checked_add(jiff::Span::new().years(1))
                    .is_ok_and(|year| until > year)
                {
                    errors.add(
                        "recurrence_until",
                        "must be at most one year after the start date",
                    );
                }
                let count =
                    recurrence::slots(a.starts_at, None, &a.time_zone, rule, Some(until)).len();
                if count > recurrence::MAX_OCCURRENCES {
                    errors.add("recurrence_until",format!("would create {count} occurrences (maximum 52); pick an earlier end date"));
                }
            }
        }
        Ok(errors)
    }

    pub(super) fn validate_existing(&self, conn: &Connection) -> Result<Errors> {
        let a = NewCalendarEvent {
            room_id: self.room_id,
            organizer_id: self.organizer_id,
            title: self.title.clone(),
            description: self.description.clone(),
            starts_at: Some(self.starts_at),
            ends_at: self.ends_at,
            time_zone: self.time_zone.clone(),
            venue_room_id: self.venue_room_id,
            recurrence_rule: self.recurrence_rule.clone(),
            recurrence_until: self.recurrence_until,
            meet_link_requested: self.meet_link_requested,
        };
        let mut errors =
            Self::validate_fields(conn, &a, false, !self.series() || self.series_head())?;
        if self.series_head()
            && self
                .recurrence_rule
                .as_deref()
                .is_none_or(campfire_richtext::ruby::is_blank)
        {
            errors.add("recurrence_rule", "can't be removed from a repeating event");
        }
        Ok(errors)
    }

    pub fn create(tx: &mut Tx<'_>, mut attributes: NewCalendarEvent) -> Result<Self> {
        tx.savepoint(|tx| {
            attributes.recurrence_rule = attributes
                .recurrence_rule
                .filter(|s| !campfire_richtext::ruby::is_blank(s));
            Self::validate(tx.conn(), &attributes)?.into_result()?;
            let mut event = Self::insert(tx, &attributes, None)?;
            if let Some(rule) = &attributes.recurrence_rule {
                tx.conn()
                    .execute("UPDATE events SET series_id=id WHERE id=?", [event.id])?;
                event.series_id = Some(event.id);
                for (start, end) in recurrence::slots(
                    attributes.starts_at,
                    attributes.ends_at,
                    &attributes.time_zone,
                    rule,
                    attributes.recurrence_until,
                )
                .into_iter()
                .skip(1)
                {
                    let follower = NewCalendarEvent {
                        starts_at: Some(start),
                        ends_at: end,
                        ..attributes.clone()
                    };
                    Self::insert(tx, &follower, Some(event.id))?;
                }
            }
            let created = event.clone();
            tx.after_commit(move |after| {
                // Rails after_create_commit: earlier successful side effects survive
                // a rejected announcement. Each write keeps its own jobs atomic.
                created.invite_after_commit(after)?;
                crate::run_write(after.conn(), after.env(), |tx| {
                    created.announce_in_channel(tx)
                })
            });
            // Rails registers each Event before its organizer attendance with the
            // transaction. Its after-create-commit Meet job precedes attendance sync.
            for occurrence in event.series_events(tx.conn())? {
                if occurrence.needs_meet_link() {
                    tx.emit_after_commit(SideEffect::job(&MeetLinkJob {
                        event_id: occurrence.id,
                    }));
                }
                tx.emit_after_commit(SideEffect::job(&SyncEntryJob {
                    event_id: occurrence.id,
                    user_id: occurrence.organizer_id,
                }));
            }
            Ok(event)
        })
    }
    fn insert(tx: &mut Tx<'_>, a: &NewCalendarEvent, series_id: Option<i64>) -> Result<Self> {
        let now = tx.now();
        let id = tx.conn().query_row("INSERT INTO events (room_id,organizer_id,title,description,starts_at,ends_at,time_zone,venue_room_id,series_id,recurrence_rule,recurrence_until,meet_link_requested,created_at,updated_at) \
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING id",params![a.room_id,a.organizer_id,a.title,a.description,a.starts_at,a.ends_at,a.time_zone,a.venue_room_id,series_id,a.recurrence_rule,a.recurrence_until.map(|d|d.to_string()),a.meet_link_requested,now,now],|r|r.get(0))?;
        let event = Self::find(tx.conn(), id)?;
        attendance::EventAttendance::record_organizer(tx, &event)?;
        Ok(event)
    }
    fn invite_after_commit(&self, after: &mut Tx<'_>) -> Result<()> {
        let ids = query_all(
            after.conn(),
            "SELECT u.id FROM users u JOIN memberships m ON m.user_id=u.id \
            WHERE m.room_id=? AND u.id<>? AND u.status=0 AND u.role<>2 AND m.involvement IN ('mentions','everything') ORDER BY u.id",
            params![self.room_id, self.organizer_id],
            |r| r.get::<_, i64>(0),
        )?;
        for user_id in ids {
            // Rails' create_or_find_by! commits each recipient independently.
            // A later failure stops fan-out and announcement, retaining earlier
            // rows and their already-published after-commit activity frames.
            crate::run_write(after.conn(), after.env(), |tx| {
                ActivityItem::refresh_unread(tx, user_id, "Event", self.id, "event_invitation")
                    .map(|_| ())
            })?;
        }
        Ok(())
    }
    fn announce_in_channel(&self, tx: &mut Tx<'_>) -> Result<()> {
        use crate::broadcasts::{Broadcast, Partial, room_dom_id, room_messages};
        let url = format!(
            "{}/rooms/{}/events/{}",
            tx.env().default_url_origin,
            self.room_id,
            self.id
        );
        let message = crate::Message::create(
            tx,
            crate::NewMessage {
                room_id: self.room_id,
                creator_id: self.organizer_id,
                markdown_source: Some(format!("Scheduled an event: {}\n{url}", self.title)),
                ..Default::default()
            },
        )?;
        let room = Room::find(tx.conn(), self.room_id)?;
        tx.emit_after_commit(SideEffect::broadcast(&Broadcast::append(
            room_messages(&room),
            room_dom_id(&room, Some("messages")),
            Partial::Message {
                message_id: message.id,
            },
        )));
        Ok(())
    }
    /// Current notified going/maybe active human members, re-read in the triggering write.
    pub fn notification_recipient_ids(&self, conn: &Connection) -> Result<Vec<i64>> {
        query_all(
            conn,
            "SELECT u.id FROM users u JOIN event_attendances a ON a.user_id=u.id \
            JOIN memberships m ON m.user_id=u.id AND m.room_id=? WHERE a.event_id=? AND a.response IN ('going','maybe') \
            AND u.status=0 AND u.role<>2 AND m.involvement IN ('mentions','everything') ORDER BY u.id",
            params![self.room_id, self.id],
            |r| r.get(0),
        )
    }
    pub fn sync_calendar_entries(&self, tx: &mut Tx<'_>) -> Result<()> {
        let ids = query_all(
            tx.conn(),
            "SELECT a.user_id FROM event_attendances a JOIN google_accounts g ON g.user_id=a.user_id \
            WHERE a.event_id=? AND a.response IN ('going','maybe') AND g.disconnected_reason IS NULL",
            [self.id],
            |r| r.get::<_, i64>(0),
        )?;
        for user_id in ids {
            tx.emit_after_commit(SideEffect::job(&SyncEntryJob {
                event_id: self.id,
                user_id,
            }));
        }
        Ok(())
    }
    /// Register the record callback at its first save, preserving Rails delivery order.
    pub(super) fn broadcast_cards(&self, tx: &mut Tx<'_>) -> Result<()> {
        use crate::broadcasts::{Broadcast, Partial, conversation_messages, message_dom_id};
        let ids = query_all(
            tx.conn(),
            "SELECT message_id FROM event_references WHERE event_id=? ORDER BY message_id",
            [self.id],
            |r| r.get::<_, i64>(0),
        )?;
        for id in ids {
            let message = crate::Message::find(tx.conn(), id)?;
            tx.emit_broadcast_once(
                "events",
                self.id,
                &Broadcast::replace_keeping_scroll(
                    conversation_messages(tx.conn(), &message)?,
                    message_dom_id(&message, Some("event_cards")),
                    Partial::EventCards { message_id: id },
                ),
            );
        }
        Ok(())
    }
    pub(super) fn update_callbacks(&self, tx: &mut Tx<'_>) -> Result<()> {
        self.broadcast_cards(tx)?;
        if self.needs_meet_link() {
            tx.emit_record_job_once_if(
                "events",
                self.id,
                &MeetLinkJob { event_id: self.id },
                Some(Self::meet_link_callback_pending),
            );
        }
        Ok(())
    }
    fn meet_link_callback_pending(conn: &Connection, id: i64) -> Result<bool> {
        Ok(Self::find(conn, id)?.needs_meet_link())
    }
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.savepoint(|tx| {
            let entries = query_all(tx.conn(),"SELECT user_id,google_event_id FROM event_calendar_entries WHERE event_id=? ORDER BY id",[self.id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?)))?;
            for (user_id,google_event_id) in entries { tx.emit_after_commit(SideEffect::job(&RemoteDeleteJob((user_id,google_event_id)))); }
            tx.conn().execute("DELETE FROM event_calendar_entries WHERE event_id=?",[self.id])?;
            tx.conn().execute("DELETE FROM event_attendances WHERE event_id=?",[self.id])?;
            ActivityItem::destroy_for_source(tx,"Event",self.id)?;
            tx.conn().execute("DELETE FROM event_references WHERE event_id=?",[self.id])?;
            tx.conn().execute("DELETE FROM events WHERE id=?",[self.id])?;
            Ok(())
        })
    }
}

pub(super) fn member(conn: &Connection, room_id: i64, user_id: i64) -> Result<bool> {
    exists(
        conn,
        "SELECT 1 FROM memberships WHERE room_id=? AND user_id=?",
        params![room_id, user_id],
    )
}
