//! Calendar entry persistence and read-only event projection; event mutations remain with WS14e.
use crate::{Connection, Result, Timestamp, Tx, User};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
#[derive(Clone)]
pub struct Entry {
    pub id: i64,
    pub event_id: i64,
    pub user_id: i64,
    pub google_event_id: String,
    pub synced_at: Option<Timestamp>,
}
#[derive(Clone)]
pub struct CalendarEvent {
    pub id: i64,
    pub room_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub starts_at: Timestamp,
    pub ends_at: Option<Timestamp>,
    pub time_zone: String,
    pub cancelled_at: Option<Timestamp>,
    pub venue: Option<(i64, String)>,
}
pub fn event(conn: &Connection, id: i64) -> Result<Option<CalendarEvent>> {
    Ok(conn.query_row("SELECT e.id,e.room_id,e.title,e.description,e.starts_at,e.ends_at,e.time_zone,e.cancelled_at,r.id,r.name FROM events e LEFT JOIN rooms r ON r.id=e.venue_room_id WHERE e.id=?",[id],|r|{let venue_id:Option<i64>=r.get(8)?;Ok(CalendarEvent{id:r.get(0)?,room_id:r.get(1)?,title:r.get(2)?,description:r.get(3)?,starts_at:r.get(4)?,ends_at:r.get(5)?,time_zone:r.get(6)?,cancelled_at:r.get(7)?,venue:venue_id.map(|id|Ok::<_,rusqlite::Error>((id,r.get::<_,String>(9)?))).transpose()?})}).optional()?)
}
pub fn find(conn: &Connection, event_id: i64, user_id: i64) -> Result<Option<Entry>> {
    Ok(conn.query_row("SELECT id,event_id,user_id,google_event_id,synced_at FROM event_calendar_entries WHERE event_id=? AND user_id=?",[event_id,user_id],|r|Ok(Entry{id:r.get(0)?,event_id:r.get(1)?,user_id:r.get(2)?,google_event_id:r.get(3)?,synced_at:r.get(4)?})).optional()?)
}
/// InboundSync's find_each scope: active upcoming events, up to 50 entries in primary-key order.
pub fn inbound_entries(
    conn: &Connection,
    user_id: i64,
    now: Timestamp,
) -> Result<Vec<(Entry, i64)>> {
    Ok(conn.prepare("SELECT c.id,c.event_id,c.user_id,c.google_event_id,c.synced_at,e.room_id FROM event_calendar_entries c JOIN events e ON e.id=c.event_id WHERE c.user_id=? AND e.cancelled_at IS NULL AND COALESCE(e.ends_at,e.starts_at)>=? ORDER BY c.id LIMIT 50")?.query_map(params![user_id,now],|r|Ok((Entry{id:r.get(0)?,event_id:r.get(1)?,user_id:r.get(2)?,google_event_id:r.get(3)?,synced_at:r.get(4)?},r.get(5)?)))?.collect::<rusqlite::Result<_>>()?)
}
pub fn google_id(event_id: i64, user_id: i64) -> String {
    let bytes = [
        (event_id as u64).to_be_bytes(),
        (user_id as u64).to_be_bytes(),
    ]
    .concat();
    let mut id = "campfire".to_string();
    let alphabet = b"0123456789abcdefghijklmnopqrstuv";
    for start in (0..128).step_by(5) {
        let mut n = 0;
        for bit in start..start + 5 {
            n = n * 2
                + if bit < 128 {
                    (bytes[bit / 8] >> (7 - bit % 8)) & 1
                } else {
                    0
                };
        }
        id.push(alphabet[n as usize] as char);
    }
    id
}
fn validate(tx: &Tx<'_>, entry: &Entry) -> Result<()> {
    let mut errors = crate::Errors::default();
    if event(tx.conn(), entry.event_id)?.is_none() {
        errors.add("event", "must exist");
    }
    if User::find_by_id(tx.conn(), entry.user_id)?.is_none() {
        errors.add("user", "must exist");
    }
    if campfire_richtext::ruby::is_blank(&entry.google_event_id) {
        errors.add("google_event_id", "can't be blank");
    }
    errors.into_result()
}
pub fn reserve(tx: &mut Tx<'_>, event_id: i64, user_id: i64) -> Result<Entry> {
    if let Some(entry) = find(tx.conn(), event_id, user_id)? {
        return Ok(entry);
    }
    let entry = Entry {
        id: 0,
        event_id,
        user_id,
        google_event_id: google_id(event_id, user_id),
        synced_at: None,
    };
    validate(tx, &entry)?;
    tx.conn().execute("INSERT INTO event_calendar_entries(event_id,user_id,google_event_id,created_at,updated_at) VALUES(?,?,?,?,?)",params![event_id,user_id,entry.google_event_id,tx.now(),tx.now()])?;
    Ok(find(tx.conn(), event_id, user_id)?.expect("reserved entry"))
}
pub fn success(tx: &mut Tx<'_>, entry: &Entry) -> Result<()> {
    validate(tx, entry)?;
    tx.conn().execute("UPDATE event_calendar_entries SET synced_at=?,last_error=NULL,updated_at=? WHERE id=? AND (synced_at IS NOT ? OR last_error IS NOT NULL)",params![tx.now(),tx.now(),entry.id,tx.now()])?;
    Ok(())
}
pub fn failure(tx: &mut Tx<'_>, entry: &Entry, error: &str) -> Result<()> {
    validate(tx, entry)?;
    tx.conn().execute("UPDATE event_calendar_entries SET last_error=?,updated_at=? WHERE id=? AND last_error IS NOT ?",params![error,tx.now(),entry.id,error])?;
    Ok(())
}
pub fn delete(tx: &mut Tx<'_>, entry: &Entry) -> Result<()> {
    tx.conn()
        .execute("DELETE FROM event_calendar_entries WHERE id=?", [entry.id])?;
    Ok(())
}
/// EventCalendarEntry#destroy: capture the remote identity, then enqueue after commit.
/// Reconciliation uses `delete` instead, because the remote copy is already gone.
pub fn destroy(tx: &mut Tx<'_>, entry: &Entry) -> Result<()> {
    if tx.conn().execute("DELETE FROM event_calendar_entries WHERE id=?", [entry.id])? != 0 {
        tx.emit_after_commit(crate::Event::job(&crate::models::room_delete::RemoteDeleteJob((
            entry.user_id,
            entry.google_event_id.clone(),
        ))));
    }
    Ok(())
}
/// Association delete_all bypasses remote-delete callbacks for disconnect cleanup.
pub fn delete_all_for_user(tx: &mut Tx<'_>, user_id: i64) -> Result<usize> {
    Ok(tx.conn().execute("DELETE FROM event_calendar_entries WHERE user_id=?", [user_id])?)
}
pub fn response_is_notifying(
    conn: &Connection,
    event_id: i64,
    user_id: i64,
    room_id: i64,
) -> Result<bool> {
    Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM event_attendances WHERE event_id=? AND user_id=? AND response IN ('going','maybe')) AND EXISTS(SELECT 1 FROM memberships WHERE user_id=? AND room_id=?)",params![event_id,user_id,user_id,room_id],|r|r.get(0))?)
}
pub fn payload(event: &CalendarEvent, origin: Option<&str>) -> Value {
    let zone = crate::slash_commands::time_parser::zone(&event.time_zone);
    let format = |t: Timestamp| {
        let zoned = t.jiff().to_zoned(zone.clone());
        if matches!(zone.iana_name(), Some("UTC" | "Etc/UTC")) {
            zoned.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
        } else {
            zoned.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
        }
    };
    let url = |path: String| format!("{}{path}", origin.unwrap_or_default().trim_end_matches('/'));
    let mut lines = vec![];
    if let Some(description) = event
        .description
        .as_deref()
        .filter(|s| !campfire_richtext::ruby::is_blank(s))
    {
        lines.push(description.to_owned());
    }
    lines.push(format!(
        "From Smartfire: {}",
        url(format!("/rooms/{}/events/{}", event.room_id, event.id))
    ));
    if let Some((id, _)) = event.venue.as_ref() {
        lines.push(format!("Join: {}", url(format!("/rooms/{id}"))));
    }
    let mut payload = json!({"summary":event.title,"description":lines.join("\n\n"),"start":{"dateTime":format(event.starts_at),"timeZone":event.time_zone},"end":{"dateTime":format(event.ends_at.unwrap_or_else(||event.starts_at.since(jiff::SignedDuration::from_hours(1)))),"timeZone":event.time_zone},"reminders":{"useDefault":true}});
    if let Some((_, name)) = event.venue.as_ref() {
        payload["location"] = json!(name);
    }
    payload
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn google_entry_payload_and_ids_match_rails() {
        let v: Value =
            serde_json::from_str(include_str!("../../../../vectors/google_entry.json")).unwrap();
        for c in v["ids"].as_array().unwrap() {
            assert_eq!(
                google_id(
                    c["event_id"].as_i64().unwrap(),
                    c["user_id"].as_i64().unwrap()
                ),
                c["id"]
            );
        }
        for c in v["cases"].as_array().unwrap() {
            let e = &c["event"];
            let event = CalendarEvent {
                id: e["id"].as_i64().unwrap(),
                room_id: e["room_id"].as_i64().unwrap(),
                title: e["title"].as_str().unwrap().into(),
                description: e["description"].as_str().map(str::to_owned),
                starts_at: Timestamp::parse_db(e["starts_at"].as_str().unwrap()).unwrap(),
                ends_at: e["ends_at"].as_str().and_then(Timestamp::parse_db),
                time_zone: e["time_zone"].as_str().unwrap().into(),
                cancelled_at: None,
                venue: c["venue"]["id"]
                    .as_i64()
                    .map(|id| (id, c["venue"]["name"].as_str().unwrap().to_owned())),
            };
            assert_eq!(
                payload(&event, c["origin"].as_str()),
                c["payload"],
                "{}",
                c["name"]
            );
        }
    }
}
