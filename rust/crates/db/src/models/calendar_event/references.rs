//! app/models/event/reference_sync.rb. URLs match on any host and even a mismatched
//! room-id path, but the referenced event must belong to the message's actual room.
use crate::sql::query_all;
use crate::{Message, Result, Tx};
use regex::Regex;
use rusqlite::params;
use std::sync::LazyLock;
static PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/rooms/[0-9]+/events/([0-9]+)\b").unwrap());
pub fn extract_ids(text: &str) -> Vec<i64> {
    let mut ids = Vec::new();
    for c in PATTERN.captures_iter(text) {
        let digits = c[1].trim_start_matches('0');
        if let Ok(id) = (if digits.is_empty() { "0" } else { digits }).parse::<i64>()
            && !ids.contains(&id)
        {
            ids.push(id);
        }
    }
    ids
}
pub fn sync(tx: &mut Tx<'_>, message: &Message) -> Result<()> {
    let text = format!(
        "{}\n{}",
        message.markdown_source.as_deref().unwrap_or(""),
        message.plain_text_body(tx.conn(), tx.env().rich_text.as_ref())?
    );
    let mut desired = Vec::new();
    for id in extract_ids(&text) {
        if crate::sql::exists(
            tx.conn(),
            "SELECT 1 FROM events WHERE id=? AND room_id=?",
            params![id, message.room_id],
        )? {
            desired.push(id);
        }
    }
    let existing = query_all(
        tx.conn(),
        "SELECT id,event_id FROM event_references WHERE message_id=?",
        [message.id],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
    )?;
    for (id, event_id) in &existing {
        if !desired.contains(event_id) {
            tx.conn()
                .execute("DELETE FROM event_references WHERE id=?", [id])?;
        }
    }
    for id in desired {
        if !existing.iter().any(|(_, e)| *e == id) {
            tx.conn().execute("INSERT INTO event_references (message_id,event_id,created_at,updated_at) VALUES (?,?,?,?)",params![message.id,id,tx.now(),tx.now()])?;
        }
    }
    Ok(())
}
