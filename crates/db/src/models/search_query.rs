//! `app/models/search_query.rb` and the bounded window in `SearchesController#set_messages`.
//! Owns read-only side-section queries; event/thread mutations stay with their owners.
use crate::sql::query_all;
use crate::{Connection, Message, Result, Timestamp};
use jiff::{
    civil::{Date, Time},
    tz::TimeZone,
};
use rails_compat::unicode;
use regex::Regex;
use rusqlite::types::Value;
use serde::Serialize;
use std::sync::LazyLock;
static OPERATORS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[ \t\n\x0b\x0c\r])((from|in|has|before|after|on|is):([^ \t\n\x0b\x0c\r]+))")
        .unwrap()
});
mod word_ranges;
use word_ranges::WORD_RANGES;
fn word_character(character: char) -> bool {
    let point = character as u32;
    WORD_RANGES
        .binary_search_by(|&(start, end)| {
            if end < point {
                std::cmp::Ordering::Less
            } else if start > point {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}
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
        unicode::downcase(s)
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}
/// The start of `date` in `zone`, or `None` past the last representable instant (a date
/// operator accepts any four-digit year, and `9999-12-31` can't be held in every zone). Such a
/// bound lies after every message, as Ruby's unbounded dates do.
fn midnight(date: Date, zone: &TimeZone) -> Option<Timestamp> {
    crate::slash_commands::time_parser::local_datetime(date.to_datetime(Time::MIN), zone)
}

/// The start of the day after `date` in `zone`, or `None` past the last representable instant.
fn next_midnight(date: Date, zone: &TimeZone) -> Option<Timestamp> {
    midnight(date.checked_add(jiff::Span::new().days(1)).ok()?, zone)
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
        self.text
            .split(|character| !word_character(character))
            .filter(|token| !token.is_empty())
            .map(str::to_owned)
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
   "file"=>" AND (EXISTS (SELECT 1 FROM active_storage_attachments a WHERE a.record_type='Message' AND a.record_id=messages.id AND a.name IN ('attachment','attachments')) OR EXISTS (SELECT 1 FROM drive_attachments d WHERE d.message_id=messages.id))",
   "image"=>" AND EXISTS (SELECT 1 FROM active_storage_attachments a JOIN active_storage_blobs b ON b.id=a.blob_id WHERE a.record_type='Message' AND a.record_id=messages.id AND a.name IN ('attachment','attachments') AND b.content_type LIKE 'image/%')",
   "pin"=>" AND EXISTS (SELECT 1 FROM message_pins p WHERE p.message_id=messages.id)",_=>unreachable!(),
  });
        }
        // A bound past the last representable instant comes after every message: `before:`
        // then holds for all of them, and `after:`/`on:` for none.
        if let Some(date) = self.before_date
            && let Some(end) = midnight(date, zone)
        {
            sql.push_str(" AND messages.created_at < ?");
            values.push(end.to_db().into());
        }
        if let Some(date) = self.after_date {
            match next_midnight(date, zone) {
                Some(start) => {
                    sql.push_str(" AND messages.created_at >= ?");
                    values.push(start.to_db().into());
                }
                None => sql.push_str(" AND 0"),
            }
        }
        if let Some(date) = self.on_date {
            match midnight(date, zone) {
                Some(start) => {
                    sql.push_str(" AND messages.created_at >= ?");
                    values.push(start.to_db().into());
                    let resolved_date = start.jiff().to_zoned(zone.clone()).date();
                    if let Some(end) = next_midnight(resolved_date, zone) {
                        sql.push_str(" AND messages.created_at < ?");
                        values.push(end.to_db().into());
                    }
                }
                None => sql.push_str(" AND 0"),
            }
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
        let key = match before {
            Some(before) => {
                let cursor = Message::find_reachable(conn, user, before)?;
                Some((cursor.created_at, cursor.id))
            }
            None => None,
        };
        self.messages_for_user_after(conn, user, zone, key)
    }
    /// The single-page app's page: [`Self::messages_for_user`] starting strictly after the
    /// `(created_at, id)` key `after` in `created_at DESC, id DESC` order, whether or not that
    /// message still exists or is reachable.
    pub fn messages_for_user_after(
        &self,
        conn: &Connection,
        user: i64,
        zone: TimeZone,
        after: Option<(Timestamp, i64)>,
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
        if let Some((created_at, id)) = after {
            sql.push_str(" AND (messages.created_at,messages.id)<(?,?)");
            values.extend([created_at.to_db().into(), id.into()]);
        }
        sql.push_str(" ORDER BY messages.created_at DESC,messages.id DESC LIMIT ?");
        values.push((super::message::PAGE_SIZE + 1).into());
        let mut ids: Vec<i64> = query_all(
            conn,
            &sql.replacen("SELECT messages.*", "SELECT messages.id", 1),
            rusqlite::params_from_iter(values),
            |row| row.get(0),
        )?;
        let has_more = ids.len() > super::message::PAGE_SIZE as usize;
        ids.truncate(super::message::PAGE_SIZE as usize);
        let messages = if ids.is_empty() {
            vec![]
        } else {
            query_all(
                conn,
                &format!(
                    "SELECT * FROM messages WHERE id IN ({}) ORDER BY created_at ASC,id ASC",
                    crate::sql::placeholders(ids.len())
                ),
                rusqlite::params_from_iter(ids),
                Message::from_row,
            )?
        };
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

/// Preloaded scalar side-section rows; rendering never reads a room association.
#[derive(Debug)]
pub struct SearchSection {
    pub kind: &'static str,
    pub records: Vec<SearchSectionRecord>,
}
#[derive(Debug)]
pub struct SearchSectionRecord {
    pub id: i64,
    pub room_id: i64,
    pub room_type: String,
    pub room_name: Option<String>,
    pub title: String,
    pub time: Timestamp,
    pub status: Option<String>,
    pub cancelled: bool,
}
impl SearchQuery {
    pub fn sections_for_user(&self, conn: &Connection, user: i64) -> Result<Vec<SearchSection>> {
        if self.text_tokens().is_empty() {
            return Ok(vec![]);
        }
        let mut sections = vec![];
        for kind in ["board-posts", "work-threads", "events"] {
            let events = kind == "events";
            let table = if events { "events" } else { "channel_threads" };
            let title = if events { "title" } else { "name" };
            let time = if events {
                "starts_at"
            } else {
                "last_activity_at"
            };
            let status = if events { "NULL" } else { "rec.work_status" };
            let cancelled = if events {
                "rec.cancelled_at IS NOT NULL"
            } else {
                "0"
            };
            let mut sql = format!(
                "SELECT rec.id,rec.room_id,rooms.type,rooms.name,rec.{title},rec.{time},{status},{cancelled} FROM {table} rec JOIN rooms ON rooms.id=rec.room_id WHERE rooms.deleted_at IS NULL AND EXISTS (SELECT 1 FROM memberships mem WHERE mem.room_id=rooms.id AND mem.user_id=?)"
            );
            let mut values: Vec<Value> = vec![user.into()];
            if !events {
                sql.push_str(if kind == "board-posts" {
                    " AND rooms.type='Rooms::Board'"
                } else {
                    " AND rooms.type!='Rooms::Board' AND rec.work_status IS NOT NULL"
                });
            }
            if !self.in_rooms.is_empty() {
                sql.push_str(" AND (");
                sql.push_str(
                    &vec!["LOWER(rooms.name) LIKE ? ESCAPE '\\'"; self.in_rooms.len()].join(" OR "),
                );
                sql.push(')');
                values.extend(self.in_rooms.iter().map(|name| Value::from(like(name))));
            }
            for token in self.text_tokens() {
                if events {
                    sql.push_str(" AND (LOWER(rec.title) LIKE ? ESCAPE '\\' OR LOWER(rec.description) LIKE ? ESCAPE '\\')");
                    values.push(like(&token).into());
                    values.push(like(&token).into());
                } else {
                    sql.push_str(" AND LOWER(rec.name) LIKE ? ESCAPE '\\'");
                    values.push(like(&token).into());
                }
            }
            sql.push_str(&format!(" ORDER BY rec.{time} DESC,rec.id DESC LIMIT 10"));
            let records = query_all(conn, &sql, rusqlite::params_from_iter(values), |r| {
                Ok(SearchSectionRecord {
                    id: r.get(0)?,
                    room_id: r.get(1)?,
                    room_type: r.get(2)?,
                    room_name: r.get(3)?,
                    title: r.get(4)?,
                    time: r.get(5)?,
                    status: r.get(6)?,
                    cancelled: r.get(7)?,
                })
            })?;
            if !records.is_empty() {
                sections.push(SearchSection { kind, records });
            }
        }
        Ok(sections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn word_ranges_match_the_pinned_ruby_runtime() {
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("../../../../vectors/messaging/search.json"))
                .unwrap();
        assert_eq!(
            serde_json::to_value(WORD_RANGES).unwrap(),
            oracle["word_ranges"]
        );
    }

    /// Ruby's dates run past year 9999, so a bound there just lies after every message.
    #[test]
    fn dates_past_the_last_instant_bound_after_every_message() {
        let kiritimati = TimeZone::get("Pacific/Kiritimati").unwrap();
        for zone in [TimeZone::UTC, kiritimati] {
            let sql = |raw: &str| SearchQuery::parse(raw).sql(&zone).unwrap();
            let (after, values) = sql("after:9999-12-31");
            assert!(after.ends_with(" AND 0"), "{after}");
            assert!(values.is_empty());
            let (on, _) = sql("on:9999-12-31");
            assert!(on.contains(" AND 0") || !on.contains("< ?"), "{on}");
            let (before, _) = sql("before:9999-12-31 hello");
            assert!(before.contains("MATCH ?"), "{before}");
            let (ordinary, values) = sql("after:2026-01-01 before:2026-02-01");
            assert!(ordinary.contains("created_at >= ?") && ordinary.contains("created_at < ?"));
            assert_eq!(values.len(), 2);
        }
    }
}
