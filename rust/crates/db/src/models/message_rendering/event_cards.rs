//! Read-only Event card facts; WS14e owns event writes and attendance endpoints.
use crate::{Result, Timestamp};
use rusqlite::Connection;
use std::collections::HashMap;
pub struct EventCard {
    pub id: i64,
    pub room_id: i64,
    pub title: String,
    pub starts_at: Timestamp,
    pub ends_at: Option<Timestamp>,
    pub time_zone: String,
    pub cancelled: bool,
    pub series: bool,
    pub venue: Option<String>,
    pub meet_link: Option<String>,
    pub organizer: String,
}
pub(super) fn load(conn: &Connection, ids: &[i64]) -> Result<HashMap<i64, Vec<EventCard>>> {
    let mut result = HashMap::<i64, Vec<EventCard>>::new();
    for (message,card) in super::rows(conn,
        "SELECT r.message_id,e.*,u.name AS organizer_name,v.name AS venue_name FROM event_references r
         JOIN events e ON e.id=r.event_id JOIN users u ON u.id=e.organizer_id LEFT JOIN rooms v ON v.id=e.venue_room_id
         WHERE r.message_id IN ($ids) ORDER BY e.starts_at,e.id",ids,|r| {
            Ok((r.get::<_,i64>("message_id")?,EventCard { id:r.get("id")?,room_id:r.get("room_id")?,title:r.get("title")?,starts_at:r.get("starts_at")?,ends_at:r.get("ends_at")?,time_zone:r.get("time_zone")?,cancelled:r.get::<_,Option<Timestamp>>("cancelled_at")?.is_some(),series:r.get::<_,Option<i64>>("series_id")?.is_some(),venue:r.get("venue_name")?,meet_link:r.get("meet_link")?,organizer:r.get("organizer_name")? }))
         })? { result.entry(message).or_default().push(card); }
    Ok(result)
}
