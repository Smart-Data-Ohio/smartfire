//! rooms/events_controller.rb#index preloads associations across the whole list.
//! Keep full PageEvent facts accurate while avoiding per-event and per-venue SQL.
use super::*;
use std::collections::{HashMap, HashSet};

struct Facts {
    organizers: HashMap<i64, User>,
    venues: HashMap<i64, VenueView>,
    attendances: HashMap<i64, Vec<AttendeeView>>,
    counts: HashMap<(i64, String), i64>,
    responses: HashMap<i64, String>,
    copies: HashSet<i64>,
    series: HashMap<i64, Vec<CalendarEvent>>,
    member: bool,
}
impl Facts {
    fn load(conn: &Connection, room: &Room, user: &User, events: &[CalendarEvent]) -> Result<Self> {
        let event_ids: Vec<_> = events.iter().map(|e| e.id).collect();
        let encoded = serde_json::json!(event_ids).to_string();
        let organizer_ids: Vec<_> = events
            .iter()
            .map(|e| e.organizer_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let organizers = User::where_ids(conn, &organizer_ids)?
            .into_iter()
            .map(|u| (u.id, u))
            .collect();
        let venue_ids: Vec<_> = events
            .iter()
            .filter_map(|e| e.venue_room_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let encoded_venues = serde_json::json!(venue_ids).to_string();
        let venue_members:HashSet<i64>=query_all(conn,"SELECT room_id FROM memberships WHERE user_id=? AND room_id IN (SELECT value FROM json_each(?))",rusqlite::params![user.id,encoded_venues],|r|r.get(0))?.into_iter().collect();
        let mut live_names = HashMap::new();
        for (rid, name) in query_all(
            conn,
            "SELECT s.room_id,u.name FROM streams s JOIN users u ON u.id=s.user_id WHERE s.ended_at IS NULL AND s.room_id IN (SELECT value FROM json_each(?)) ORDER BY s.id",
            [encoded_venues],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )? {
            live_names.entry(rid).or_insert(name);
        }
        let venues = Room::for_ids(conn, &venue_ids)?
            .into_iter()
            .map(|v| {
                let stage = v.room_type == campfire_db::RoomType::Stage;
                (
                    v.id,
                    VenueView {
                        id: v.id,
                        name: v.name.unwrap_or_default(),
                        stage,
                        member: venue_members.contains(&v.id),
                        live_user: if stage {
                            live_names.remove(&v.id)
                        } else {
                            None
                        },
                    },
                )
            })
            .collect();
        let mut attendances: HashMap<i64, Vec<AttendeeView>> = HashMap::new();
        let mut counts = HashMap::new();
        let mut responses = HashMap::new();
        // LEFT JOIN keeps orphan attendance counts identical to attendance_counts;
        // only the displayed names use the same inner-join semantics as show.
        for (eid, uid, response, name) in query_all(
            conn,
            "SELECT a.event_id,a.user_id,a.response,u.name FROM event_attendances a LEFT JOIN users u ON u.id=a.user_id WHERE a.event_id IN (SELECT value FROM json_each(?)) ORDER BY a.response,a.id",
            [&encoded],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            },
        )? {
            *counts.entry((eid, response.clone())).or_insert(0) += 1;
            if uid == user.id {
                responses.insert(eid, response.clone());
            }
            if let Some(name) = name {
                attendances
                    .entry(eid)
                    .or_default()
                    .push(AttendeeView { name, response });
            }
        }
        let copies=query_all(conn,"SELECT event_id FROM event_calendar_entries WHERE user_id=? AND event_id IN (SELECT value FROM json_each(?))",rusqlite::params![user.id,encoded],|r|r.get(0))?.into_iter().collect();
        let series_ids: Vec<_> = events
            .iter()
            .filter_map(|e| e.series_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let mut series: HashMap<i64, Vec<CalendarEvent>> = HashMap::new();
        for e in CalendarEvent::for_series_ids(conn, &series_ids)? {
            series.entry(e.series_id.unwrap()).or_default().push(e);
        }
        let member = exists(
            conn,
            "SELECT 1 FROM memberships WHERE room_id=? AND user_id=?",
            rusqlite::params![room.id, user.id],
        )?;
        Ok(Self {
            organizers,
            venues,
            attendances,
            counts,
            responses,
            copies,
            series,
            member,
        })
    }
    fn page(&self, e: &CalendarEvent, user: &User, remaining: Option<i64>) -> Result<PageEvent> {
        let venue = e
            .venue_room_id
            .map(|id| {
                self.venues
                    .get(&id)
                    .cloned()
                    .ok_or(campfire_db::Error::RecordNotFound("Room"))
            })
            .transpose()?;
        let siblings = e.series_id.and_then(|id| self.series.get(&id));
        let previous = siblings
            .and_then(|rows| {
                rows.iter()
                    .rfind(|s| (s.starts_at, s.id) < (e.starts_at, e.id))
            })
            .map(|s| s.id);
        let next = siblings
            .and_then(|rows| {
                rows.iter()
                    .find(|s| (s.starts_at, s.id) > (e.starts_at, e.id))
            })
            .map(|s| s.id);
        Ok(PageEvent {
            card: CardView {
                id: e.id,
                room_id: e.room_id,
                title: e.title.clone(),
                organizer_name: self
                    .organizers
                    .get(&e.organizer_id)
                    .ok_or(campfire_db::Error::RecordNotFound("User"))?
                    .name
                    .clone(),
                starts_at: super::calendar_time(e.starts_at),
                ends_at: e.ends_at.map(super::calendar_time),
                time_zone: e.time_zone.clone(),
                series: e.series(),
                cancelled: e.cancelled(),
                venue_name: venue.as_ref().map(|v| v.name.clone()),
                meet_link: e.meet_link.as_deref().and_then(rails_compat::safe_https),
            },
            venue,
            going: *self.counts.get(&(e.id, "going".into())).unwrap_or(&0),
            maybe: *self.counts.get(&(e.id, "maybe".into())).unwrap_or(&0),
            declined: *self.counts.get(&(e.id, "declined".into())).unwrap_or(&0),
            recurrence_label: e.recurrence_rule.as_deref().map(|r| {
                format!(
                    "Repeats {}",
                    campfire_db::models::calendar_event::recurrence::phrase(r)
                )
            }),
            recurrence_phrase: e
                .recurrence_rule
                .as_deref()
                .map(|r| campfire_db::models::calendar_event::recurrence::phrase(r).to_owned()),
            recurrence_until: e
                .recurrence_until
                .map(|d| d.strftime("%B %-d, %Y").to_string()),
            remaining,
            manageable: e.manageable_by(Some(user)),
            respondable: !e.cancelled() && user.is_active() && !user.is_bot() && self.member,
            current_response: self.responses.get(&e.id).cloned(),
            previous,
            next,
            head: e.series_head(),
            description_html: e
                .description
                .as_deref()
                .filter(|s| !campfire_richtext::ruby::is_blank(s))
                .map(simple_format)
                .transpose()?,
            calendar_copy: self.copies.contains(&e.id),
            attendances: self.attendances.get(&e.id).cloned().unwrap_or_default(),
        })
    }
}
pub(super) fn load(
    conn: &Connection,
    room: &Room,
    user: &User,
    now: Timestamp,
) -> Result<IndexView> {
    let events = CalendarEvent::for_room(conn, room.id)?;
    let facts = Facts::load(conn, room, user, &events)?;
    let upcoming: Vec<_> = events
        .iter()
        .filter(|e| !e.cancelled() && e.ends_at.unwrap_or(e.starts_at) >= now)
        .collect();
    let mut counts = HashMap::new();
    for e in &upcoming {
        if let Some(id) = e.series_id {
            *counts.entry(id).or_insert(0) += 1;
        }
    }
    let mut seen = HashSet::new();
    let upcoming = upcoming
        .into_iter()
        .filter(|e| e.series_id.is_none_or(|id| seen.insert(id)))
        .map(|e| facts.page(e, user, e.series_id.map(|id| counts[&id])))
        .collect::<Result<_>>()?;
    let past = events
        .iter()
        .rev()
        .filter(|e| !e.cancelled() && e.ends_at.unwrap_or(e.starts_at) < now)
        .map(|e| facts.page(e, user, None))
        .collect::<Result<_>>()?;
    let cancelled = events
        .iter()
        .rev()
        .filter(|e| e.cancelled())
        .map(|e| facts.page(e, user, None))
        .collect::<Result<_>>()?;
    Ok(IndexView {
        room_id: room.id,
        room_name: room_name(conn, room, user)?,
        upcoming,
        past,
        cancelled,
    })
}
