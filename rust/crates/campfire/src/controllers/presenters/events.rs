//! Preload event card facts once, with no request or viewer state in the provider.
use campfire_db::{CalendarEvent, Connection, Message, Result, Room, User};
use campfire_views::events::CardView;

pub fn for_message(conn: &Connection, message: &Message) -> Result<Vec<CardView>> {
    let mut statement=conn.prepare("SELECT e.* FROM events e JOIN event_references r ON r.event_id=e.id WHERE r.message_id=? AND e.room_id=? ORDER BY e.starts_at,e.id")?;
    let ids = statement
        .query_map(rusqlite::params![message.id, message.room_id], |row| {
            row.get::<_, i64>("id")
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.into_iter()
        .map(|id| {
            let e = CalendarEvent::find(conn, id)?;
            Ok(CardView {
                id: e.id,
                room_id: e.room_id,
                title: e.title.clone(),
                organizer_name: User::find(conn, e.organizer_id)?.name,
                starts_at: e.starts_at.jiff(),
                ends_at: e.ends_at.map(|t| t.jiff()),
                time_zone: e.time_zone.clone(),
                series: e.series(),
                cancelled: e.cancelled(),
                venue_name: e
                    .venue_room_id
                    .map(|id| Room::find(conn, id))
                    .transpose()?
                    .and_then(|r| r.name),
                meet_link: e.meet_link.as_deref().and_then(rails_compat::safe_https),
            })
        })
        .collect()
}

pub fn cards(conn: &Connection, message_id: i64) -> Result<String> {
    use askama::Template;
    let message = Message::find(conn, message_id)?;
    let events = for_message(conn, &message)?;
    let entries = campfire_views::events::card_entries(
        &events,
        &message.id.to_string(),
        &campfire_views::time::Zone::utc(),
    );
    campfire_views::events::Cards {
        message_key: &message.client_message_id,
        entries: &entries,
    }
    .render()
    .map_err(|e| campfire_db::Error::Other(e.to_string()))
}

use campfire_db::Timestamp;
use campfire_views::events::pages::{AttendeeView, IndexView, PageEvent, ShowView, VenueView};

pub fn room_name(conn: &Connection, room: &Room, user: &User) -> Result<String> {
    let names = if room.direct() {
        room.users(conn)?
            .into_iter()
            .filter(|u| u.id != user.id)
            .map(|u| u.name)
            .collect()
    } else {
        Vec::new()
    };
    Ok(campfire_views::rooms::room_display_name(
        room.name.as_deref(),
        room.direct(),
        &names,
        Some(&user.name),
    ))
}
pub fn page_event(
    conn: &Connection,
    e: &CalendarEvent,
    user: &User,
    remaining: Option<i64>,
) -> Result<PageEvent> {
    let venue=e.venue_room_id.map(|id| ->Result<_> {
        let room=Room::find(conn,id)?;
        let member=exists(conn,"SELECT 1 FROM memberships WHERE room_id=? AND user_id=?",rusqlite::params![id,user.id])?;
        let stage=room.room_type==campfire_db::RoomType::Stage;
        // WS13 reader seam: only the current stream, never historical rows.
        let live_user=if stage {query_one(conn,"SELECT u.name FROM streams s JOIN users u ON u.id=s.user_id WHERE s.room_id=? AND s.ended_at IS NULL LIMIT 1",[id],|r|r.get(0))?}else{None};
        Ok(VenueView{id,name:room.name.unwrap_or_default(),stage,member,live_user})
    }).transpose()?;
    let attendances = query_all(
        conn,
        "SELECT u.name,a.response FROM event_attendances a JOIN users u ON u.id=a.user_id WHERE a.event_id=? ORDER BY a.response,a.id",
        [e.id],
        |r| {
            Ok(AttendeeView {
                name: r.get(0)?,
                response: r.get(1)?,
            })
        },
    )?;
    let counts = e.attendance_counts(conn)?;
    let calendar_copy = exists(
        conn,
        "SELECT 1 FROM event_calendar_entries WHERE event_id=? AND user_id=?",
        rusqlite::params![e.id, user.id],
    )?;
    Ok(PageEvent {
        card: CardView {
            id: e.id,
            room_id: e.room_id,
            title: e.title.clone(),
            organizer_name: User::find(conn, e.organizer_id)?.name,
            starts_at: e.starts_at.jiff(),
            ends_at: e.ends_at.map(|t| t.jiff()),
            time_zone: e.time_zone.clone(),
            series: e.series(),
            cancelled: e.cancelled(),
            venue_name: venue.as_ref().map(|v| v.name.clone()),
            meet_link: e.meet_link.as_deref().and_then(rails_compat::safe_https),
        },
        venue,
        going: *counts.get("going").unwrap_or(&0),
        maybe: *counts.get("maybe").unwrap_or(&0),
        declined: *counts.get("declined").unwrap_or(&0),
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
        respondable: e.respondable_by(conn, Some(user))?,
        current_response: e.response_for(conn, Some(user.id))?,
        previous: e.previous_occurrence(conn)?.map(|e| e.id),
        next: e.next_occurrence(conn)?.map(|e| e.id),
        head: e.series_head(),
        description_html: e
            .description
            .as_deref()
            .filter(|s| !campfire_richtext::ruby::is_blank(s))
            .map(simple_format)
            .transpose()?,
        calendar_copy,
        attendances,
    })
}
fn simple_format(text: &str) -> Result<String> {
    let sanitized = campfire_richtext::sanitizer::sanitize(
        text,
        &campfire_richtext::sanitizer::SafeList::defaults(),
    )
    .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
    let text = sanitized.replace("\r\n", "\n").replace('\r', "\n");
    let split = regex::Regex::new("\\n\\n+").unwrap();
    let breaks = regex::Regex::new("([^\\n]\\n)([^\\n])").unwrap();
    Ok(split
        .split(&text)
        .map(|p| format!("<p>{}</p>", breaks.replace_all(p, "${1}<br />${2}")))
        .collect::<Vec<_>>()
        .join("\n\n"))
}
pub fn index(conn: &Connection, room: &Room, user: &User, now: Timestamp) -> Result<IndexView> {
    let events = query_all(
        conn,
        "SELECT id FROM events WHERE room_id=? ORDER BY starts_at,id",
        [room.id],
        |r| r.get::<_, i64>(0),
    )?
    .into_iter()
    .map(|id| CalendarEvent::find(conn, id))
    .collect::<Result<Vec<_>>>()?;
    let upcoming: Vec<_> = events
        .iter()
        .filter(|e| !e.cancelled() && e.ends_at.unwrap_or(e.starts_at) >= now)
        .collect();
    let mut counts = std::collections::BTreeMap::new();
    for e in &upcoming {
        if let Some(id) = e.series_id {
            *counts.entry(id).or_insert(0) += 1;
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    let upcoming = upcoming
        .into_iter()
        .filter(|e| e.series_id.is_none_or(|id| seen.insert(id)))
        .map(|e| page_event(conn, e, user, e.series_id.map(|id| counts[&id])))
        .collect::<Result<_>>()?;
    let past = events
        .iter()
        .rev()
        .filter(|e| !e.cancelled() && e.ends_at.unwrap_or(e.starts_at) < now)
        .map(|e| page_event(conn, e, user, None))
        .collect::<Result<_>>()?;
    let cancelled = events
        .iter()
        .rev()
        .filter(|e| e.cancelled())
        .map(|e| page_event(conn, e, user, None))
        .collect::<Result<_>>()?;
    Ok(IndexView {
        room_id: room.id,
        room_name: room_name(conn, room, user)?,
        upcoming,
        past,
        cancelled,
    })
}
pub fn show(conn: &Connection, room: &Room, user: &User, e: &CalendarEvent) -> Result<ShowView> {
    Ok(ShowView {
        room_name: room_name(conn, room, user)?,
        event: page_event(conn, e, user, None)?,
    })
}

fn query_all<T, P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    args: P,
    mut map: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    Ok(conn
        .prepare(sql)?
        .query_map(args, |r| map(r))?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}
fn query_one<T, P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    args: P,
    map: impl FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Option<T>> {
    use rusqlite::OptionalExtension;
    Ok(conn.query_row(sql, args, map).optional()?)
}
fn exists<P: rusqlite::Params>(conn: &Connection, sql: &str, args: P) -> Result<bool> {
    Ok(query_one(conn, sql, args, |_| Ok(()))?.is_some())
}

/// Preload the form's current-user venues, retaining a hidden stored venue.
pub fn form(
    conn: &Connection,
    room: &Room,
    user: &User,
    a: &campfire_db::NewCalendarEvent,
    persisted: Option<&CalendarEvent>,
    errors: &campfire_db::Errors,
    title_value: Option<String>,
) -> Result<campfire_views::events::forms::FormView> {
    use campfire_views::events::forms::{FormView, VenueOption};
    let mut venues = query_all(
        conn,
        "SELECT r.id,r.name,r.type FROM rooms r JOIN memberships m ON m.room_id=r.id WHERE m.user_id=? AND r.deleted_at IS NULL AND r.type IN ('Rooms::Voice','Rooms::Stage') ORDER BY LOWER(r.name)",
        [user.id],
        |r| {
            Ok(VenueOption {
                id: r.get(0)?,
                name: r.get(1)?,
                stage: r.get::<_, String>(2)? == "Rooms::Stage",
            })
        },
    )?;
    if let Some(id) = a
        .venue_room_id
        .filter(|id| !venues.iter().any(|v| v.id == *id))
        && let Some(r) = Room::find_by_id(conn, id)?
    {
        venues.push(VenueOption {
            id,
            name: r.name.unwrap_or_default(),
            stage: r.room_type == campfire_db::RoomType::Stage,
        });
    }
    if (a.starts_at.is_some() || a.ends_at.is_some())
        && campfire_views::time::Zone::lookup(&a.time_zone).is_none()
    {
        return Err(campfire_db::Error::Other(
            "Rails form cannot format an invalid event time zone".into(),
        ));
    }
    let zone = campfire_views::time::Zone::for_user(Some(&a.time_zone));
    Ok(FormView {
        room_id: room.id,
        room_name: room_name(conn, room, user)?,
        id: persisted.map(|e| e.id),
        title: a.title.clone(),
        title_value,
        description: a.description.clone(),
        starts_at: a.starts_at.map(|t| zone.format(t.jiff(), "%Y-%m-%dT%H:%M")),
        ends_at: a.ends_at.map(|t| zone.format(t.jiff(), "%Y-%m-%dT%H:%M")),
        time_zone: a.time_zone.clone(),
        venue_room_id: a.venue_room_id,
        recurrence_rule: a.recurrence_rule.clone(),
        recurrence_until: a.recurrence_until.map(|d| d.to_string()),
        meet_link_requested: a.meet_link_requested,
        meet_link: persisted
            .and_then(|e| e.meet_link.as_deref())
            .and_then(rails_compat::safe_https),
        series: persisted.is_some_and(CalendarEvent::series),
        head: persisted.is_some_and(CalendarEvent::series_head),
        errors: errors.full_messages(),
        error_fields: errors.0.iter().map(|(f, _)| f.to_string()).collect(),
        venues,
    })
}
