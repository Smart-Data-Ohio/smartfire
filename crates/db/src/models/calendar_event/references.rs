//! app/models/event/reference_sync.rb. URLs match on any host and even a mismatched
//! room-id path, but the referenced event must belong to the message's actual room.
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
    sync_from_plain_text(tx, message, &message.plain_text_body(tx.conn(), tx.env().rich_text.as_ref())?)
}
pub(crate) fn sync_from_plain_text(tx: &mut Tx<'_>, message: &Message, plain: &str) -> Result<()> {
    let text = format!(
        "{}\n{}",
        message.markdown_source.as_deref().unwrap_or(""),
        plain
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
    tx.conn().execute(
        "DELETE FROM event_references WHERE message_id=? AND event_id NOT IN (SELECT value FROM json_each(?))",
        params![message.id, serde_json::json!(desired).to_string()],
    )?;
    for id in desired {
        tx.conn().execute("INSERT INTO event_references (message_id,event_id,created_at,updated_at) VALUES (?,?,?,?) ON CONFLICT(message_id,event_id) DO NOTHING",params![message.id,id,tx.now(),tx.now()])?;
    }
    Ok(())
}
