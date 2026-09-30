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
