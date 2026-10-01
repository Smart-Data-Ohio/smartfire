//! Bounded, access-neutral file listing reads; callers authorize the room first.
use crate::{Connection, Result, Timestamp};
use rusqlite::{params, types::Value};

pub const TYPES: [&str; 5] = ["all", "images", "videos", "documents", "other"];
pub fn file_type(value: &str) -> &'static str {
    TYPES.into_iter().find(|t| *t == value).unwrap_or("all")
}
pub fn page(value: &str) -> i64 {
    {
        let raw =
            value.trim_start_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}']);
        let mut chars = raw.chars().peekable();
        let negative = chars.peek() == Some(&'-');
        if matches!(chars.peek(), Some('+' | '-')) {
            chars.next();
        }
        let digits = chars.take_while(char::is_ascii_digit).collect::<String>();
        let number = digits
            .parse::<i64>()
            .unwrap_or(if digits.is_empty() { 0 } else { i64::MAX });
        if negative { -number } else { number }
    }
    .clamp(1, 20)
}
// Every SQL pattern is fixed, just as in Rooms::FilesController; filename is always bound.
const DOCUMENT: &str = "b.content_type LIKE 'application/pdf' OR b.content_type LIKE 'text/%' OR b.content_type LIKE 'application/msword' OR b.content_type LIKE 'application/vnd.ms-%' OR b.content_type LIKE 'application/vnd.openxmlformats-officedocument.%' OR b.content_type LIKE 'application/vnd.oasis.opendocument.%'";
const OTHER: &str = "b.content_type NOT LIKE 'image/%' AND b.content_type NOT LIKE 'video/%' AND b.content_type NOT LIKE 'application/pdf' AND b.content_type NOT LIKE 'text/%' AND b.content_type NOT LIKE 'application/msword' AND b.content_type NOT LIKE 'application/vnd.ms-%' AND b.content_type NOT LIKE 'application/vnd.openxmlformats-officedocument.%' AND b.content_type NOT LIKE 'application/vnd.oasis.opendocument.%'";
#[derive(Debug)]
pub struct Upload {
    pub blob_id: i64,
    pub message_id: i64,
    pub thread_id: Option<i64>,
    pub creator_name: String,
    pub created_at: Timestamp,
}
#[derive(Debug)]
pub struct Drive {
    pub file_id: String,
    pub message_id: i64,
    pub thread_id: Option<i64>,
    pub creator_name: String,
    pub created_at: Timestamp,
}
pub fn uploads(
    conn: &Connection,
    room_id: i64,
    file_type: &str,
    filename: &str,
    size: i64,
) -> Result<Vec<Upload>> {
    let condition = match file_type {
        "images" => "b.content_type LIKE 'image/%'",
        "videos" => "b.content_type LIKE 'video/%'",
        "documents" => DOCUMENT,
        "other" => OTHER,
        _ => "1",
    };
    let mut values = vec![Value::Integer(room_id)];
    let mut sql = format!(
        "SELECT a.blob_id,m.id,m.thread_id,u.name,a.created_at FROM active_storage_attachments a JOIN active_storage_blobs b ON b.id=a.blob_id JOIN messages m ON m.id=a.record_id JOIN users u ON u.id=m.creator_id WHERE a.record_type='Message' AND a.name='attachment' AND m.room_id=? AND ({condition})"
    );
    if !filename.is_empty() {
        sql.push_str(" AND LOWER(b.filename) LIKE ? ESCAPE '\\'");
        let literal = filename
            .to_lowercase()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        values.push(Value::Text(format!("%{literal}%")));
    }
    sql.push_str(" ORDER BY a.created_at DESC,a.id DESC LIMIT ?");
    values.push(Value::Integer(size + 1));
    Ok(conn
        .prepare(&sql)?
        .query_map(rusqlite::params_from_iter(values), |r| {
            Ok(Upload {
                blob_id: r.get(0)?,
                message_id: r.get(1)?,
                thread_id: r.get(2)?,
                creator_name: r.get(3)?,
                created_at: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}
pub fn drives(conn: &Connection, room_id: i64, size: i64) -> Result<Vec<Drive>> {
    Ok(conn.prepare("SELECT d.file_id,m.id,m.thread_id,u.name,d.created_at FROM drive_attachments d JOIN messages m ON m.id=d.message_id JOIN users u ON u.id=m.creator_id WHERE m.room_id=? ORDER BY d.created_at DESC,d.id DESC LIMIT ?")?.query_map(params![room_id,size+1],|r|Ok(Drive{file_id:r.get(0)?,message_id:r.get(1)?,thread_id:r.get(2)?,creator_name:r.get(3)?,created_at:r.get(4)?}))?.collect::<rusqlite::Result<_>>()?)
}
