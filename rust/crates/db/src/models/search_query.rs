//! `app/models/search_query.rb` and the bounded window in `SearchesController#set_messages`.
//! Board/work/event side sections remain explicit WS12/WS14 extensions.
use crate::sql::query_all;
use crate::{Connection, Message, Result, Timestamp};
use jiff::{
    civil::{Date, Time},
    tz::TimeZone,
};
use regex::Regex;
use rusqlite::types::Value;
use serde::Serialize;
use std::sync::LazyLock;
static OPERATORS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[ \t\n\x0b\x0c\r])((from|in|has|before|after|on|is):([^ \t\n\x0b\x0c\r]+))")
        .unwrap()
});
static WORDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}\p{M}\p{N}_]+").unwrap());
static SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{White_Space}+").unwrap());
static DATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$").unwrap());
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Chip {
    pub token: String,
    pub label: String,
    pub remove_query: String,
}
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub raw: String,
    pub text: String,
    pub from_names: Vec<String>,
    pub in_rooms: Vec<String>,
    pub has_values: Vec<String>,
    pub before_date: Option<Date>,
    pub after_date: Option<Date>,
    pub on_date: Option<Date>,
    pub thread_only: bool,
    pub chips: Vec<Chip>,
}
#[derive(Debug)]
pub struct SearchPage {
    pub messages: Vec<Message>,
    pub has_more: bool,
}
fn squish(s: &str) -> String {
    SPACE.replace_all(s, " ").trim().into()
}
fn like(s: &str) -> String {
    format!(
        "%{}%",
        s.to_lowercase()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}
fn midnight(date: Date, zone: &TimeZone) -> Result<Timestamp> {
    date.to_datetime(Time::MIN)
        .to_zoned(zone.clone())
        .map(|d| Timestamp::from_jiff(d.timestamp()))
        .map_err(|e| crate::Error::Other(e.to_string()))
}
impl SearchQuery {
    pub fn parse(raw: &str) -> Self {
        let mut q = Self {
            raw: raw.into(),
            ..Default::default()
        };
        let mut remaining = raw.to_string();
        for captures in OPERATORS.captures_iter(raw) {
            let token = &captures[1];
            let name = &captures[2];
            let value = &captures[3];
            let cleaned = match name {
                "from" | "in" => {
                    let value = value
                        .strip_prefix(if name == "from" { '@' } else { '#' })
                        .unwrap_or(value)
                        .trim_end_matches([',', '.', '!', '?', ';', ':', ')']);
                    if campfire_richtext::ruby::is_blank(value) {
                        None
                    } else {
                        Some(value.to_string())
                    }
                }
                "has" => {
                    let value = value.to_lowercase();
                    ["link", "file", "image", "pin"]
                        .contains(&value.as_str())
                        .then_some(value)
                }
                "is" => value.eq_ignore_ascii_case("thread").then(|| "true".into()),
                _ => {
                    if DATE.is_match(value) && value.parse::<Date>().is_ok() {
                        Some(value.into())
                    } else {
                        None
                    }
                }
            };
            let Some(cleaned) = cleaned else { continue };
            match name {
                "from" => q.from_names.push(cleaned.clone()),
                "in" => q.in_rooms.push(cleaned.clone()),
                "has" => {
                    if !q.has_values.contains(&cleaned) {
                        q.has_values.push(cleaned.clone())
                    }
                }
                "before" => q.before_date = cleaned.parse().ok(),
                "after" => q.after_date = cleaned.parse().ok(),
                "on" => q.on_date = cleaned.parse().ok(),
                "is" => q.thread_only = true,
                _ => unreachable!(),
            }
            q.chips.push(Chip {
                token: token.into(),
                label: format!("{name}: {cleaned}"),
                remove_query: squish(&raw.replacen(token, "", 1)),
            });
            remaining = remaining.replacen(token, "", 1);
        }
        q.text = squish(&remaining);
        q
    }
    pub fn text_tokens(&self) -> Vec<String> {
        WORDS
            .find_iter(&self.text)
            .map(|m| m.as_str().into())
            .collect()
    }
    pub fn filters(&self) -> bool {
        !self.from_names.is_empty()
            || !self.in_rooms.is_empty()
            || !self.has_values.is_empty()
            || self.before_date.is_some()
            || self.after_date.is_some()
            || self.on_date.is_some()
            || self.thread_only
    }
    pub fn blank_query(&self) -> bool {
        self.text_tokens().is_empty() && !self.filters()
    }
    pub fn match_expression(&self) -> Option<String> {
        let tokens = self.text_tokens();
        (!tokens.is_empty()).then(|| {
            tokens
                .iter()
                .map(|w| format!("\"{w}\""))
                .collect::<Vec<_>>()
                .join(" ")
        })
    }
    fn sql(&self, zone: &TimeZone) -> Result<(String, Vec<Value>)> {
        let mut sql = String::from(
            "SELECT messages.* FROM messages INNER JOIN rooms ON rooms.id=messages.room_id",
        );
        let mut values = vec![];
        let expression = self.match_expression();
        if expression.is_some() {
            sql.push_str(" INNER JOIN message_search_index idx ON idx.rowid=messages.id");
        }
        sql.push_str(" WHERE rooms.deleted_at IS NULL AND messages.system_note=0");
        for (names, table, column) in [
            (&self.from_names, "users", "creator_id"),
            (&self.in_rooms, "rooms", "room_id"),
        ] {
            if !names.is_empty() {
                sql.push_str(&format!(
                    " AND messages.{column} IN (SELECT id FROM {table} WHERE "
                ));
                sql.push_str(
                    &vec![format!("LOWER({table}.name) LIKE ? ESCAPE '\\'"); names.len()]
                        .join(" OR "),
                );
                sql.push(')');
                values.extend(names.iter().map(|s| Value::from(like(s))));
            }
        }
        for has in &self.has_values {
            sql.push_str(match has.as_str(){
   "link"=>" AND EXISTS (SELECT 1 FROM action_text_rich_texts rt WHERE rt.record_type='Message' AND rt.record_id=messages.id AND rt.name='body' AND (rt.body LIKE '%href=%' OR rt.body LIKE '%http://%' OR rt.body LIKE '%https://%'))",
   "file"=>" AND (EXISTS (SELECT 1 FROM active_storage_attachments a WHERE a.record_type='Message' AND a.record_id=messages.id AND a.name='attachment') OR EXISTS (SELECT 1 FROM drive_attachments d WHERE d.message_id=messages.id))",
   "image"=>" AND EXISTS (SELECT 1 FROM active_storage_attachments a JOIN active_storage_blobs b ON b.id=a.blob_id WHERE a.record_type='Message' AND a.record_id=messages.id AND a.name='attachment' AND b.content_type LIKE 'image/%')",
   "pin"=>" AND EXISTS (SELECT 1 FROM message_pins p WHERE p.message_id=messages.id)",_=>unreachable!(),
  });
        }
        if let Some(date) = self.before_date {
            sql.push_str(" AND messages.created_at < ?");
            values.push(midnight(date, zone)?.to_db().into());
        }
        if let Some(date) = self.after_date {
            sql.push_str(" AND messages.created_at >= ?");
            let next = date
                .checked_add(jiff::Span::new().days(1))
                .map_err(|e| crate::Error::Other(e.to_string()))?;
            values.push(midnight(next, zone)?.to_db().into());
        }
        if let Some(date) = self.on_date {
            sql.push_str(" AND messages.created_at >= ? AND messages.created_at < ?");
            values.push(midnight(date, zone)?.to_db().into());
            let next = date
                .checked_add(jiff::Span::new().days(1))
                .map_err(|e| crate::Error::Other(e.to_string()))?;
            values.push(midnight(next, zone)?.to_db().into());
        }
        if self.thread_only {
            sql.push_str(" AND messages.thread_id IS NOT NULL");
        }
        if let Some(expression) = expression {
            sql.push_str(" AND idx.body MATCH ?");
            values.push(expression.into());
        }
        Ok((sql, values))
    }
    pub fn messages_for_user(
        &self,
        conn: &Connection,
        user: i64,
        zone: TimeZone,
        before: Option<i64>,
    ) -> Result<SearchPage> {
        if self.blank_query() {
            return Ok(SearchPage {
                messages: vec![],
                has_more: false,
            });
        }
        let (mut sql, mut values) = self.sql(&zone)?;
        sql.push_str(" AND EXISTS (SELECT 1 FROM memberships mem WHERE mem.room_id=rooms.id AND mem.user_id=?)");
        values.push(user.into());
        if let Some(before) = before {
            let cursor = Message::find_reachable(conn, user, before)?;
            sql.push_str(" AND (messages.created_at,messages.id)<(?,?)");
            values.extend([cursor.created_at.to_db().into(), cursor.id.into()]);
        }
        sql.push_str(" ORDER BY messages.created_at DESC,messages.id DESC LIMIT ?");
        values.push((super::message::PAGE_SIZE + 1).into());
        let mut messages = query_all(
            conn,
            &sql,
            rusqlite::params_from_iter(values),
            Message::from_row,
        )?;
        let has_more = messages.len() > super::message::PAGE_SIZE as usize;
        messages.truncate(super::message::PAGE_SIZE as usize);
        messages.reverse();
        Ok(SearchPage { messages, has_more })
    }
    pub fn messages_in_room(&self, conn: &Connection, room: i64) -> Result<Vec<Message>> {
        if self.blank_query() {
            return Ok(vec![]);
        }
        let (mut sql, mut values) = self.sql(&TimeZone::UTC)?;
        sql.push_str(" AND messages.room_id=? ORDER BY messages.created_at ASC,messages.id ASC");
        values.push(room.into());
        query_all(
            conn,
            &sql,
            rusqlite::params_from_iter(values),
            Message::from_row,
        )
    }
}
