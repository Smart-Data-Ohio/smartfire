//! `reference/app/models/thread_tag.rb`: one row per tag name on a board post. The tag set is
//! written through [`ChannelThread`](crate::ChannelThread)'s `tag_names`; each create/destroy
//! replaces both board rows after commit (`after_commit :broadcast_board_row_replace`).

use rusqlite::{Connection, Row, params};

use crate::database::Tx;
use crate::error::{Errors, OptionalExt, Result};
use crate::{ChannelThread, Room};
use crate::sql::{self, CachedStatements, query_all, query_one};
use crate::time::Timestamp;

/// `ThreadTag::NAME_LIMIT`
pub const TAG_NAME_LIMIT: usize = 30;

#[derive(Debug, Clone, PartialEq)]
pub struct ThreadTag {
    pub id: i64,
    pub channel_thread_id: i64,
    pub name: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl ThreadTag {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            channel_thread_id: row.get("channel_thread_id")?,
            name: row.get("name")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(conn, r#"SELECT * FROM "thread_tags" WHERE "thread_tags"."id" = ? LIMIT 1"#, [id], Self::from_row)?
            .or_not_found("ThreadTag")
    }

    /// `thread.tags`: `-> { order(:name) }`
    pub fn for_thread(conn: &Connection, thread_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT "thread_tags".* FROM "thread_tags" WHERE "thread_tags"."channel_thread_id" = ? ORDER BY "thread_tags"."name" ASC"#,
            [thread_id],
            Self::from_row,
        )
    }

    pub fn for_threads(conn: &Connection, thread_ids: &[i64]) -> Result<Vec<Self>> {
        if thread_ids.is_empty() { return Ok(Vec::new()); }
        query_all(conn, "SELECT * FROM thread_tags WHERE channel_thread_id IN (SELECT value FROM json_each(?)) ORDER BY name", [serde_json::json!(thread_ids).to_string()], Self::from_row)
    }

    /// `thread.tags.create!(name:)`
    pub fn create(tx: &mut Tx<'_>, thread_id: i64, name: &str) -> Result<Self> {
        Self::validate(tx.conn(), thread_id, name)?.into_result()?;
        Self::insert(tx, thread_id, name)
    }

    // The parent was just persisted or reloaded under this writer lock. This removes
    // only the association existence read; uniqueness and name validation still run.
    pub(crate) fn create_for_thread(tx: &mut Tx<'_>, thread: &ChannelThread, name: &str) -> Result<Self> {
        Self::validate_name(tx.conn(), thread.id, name, Errors::default())?.into_result()?;
        Self::insert(tx, thread.id, name)
    }

    fn insert(tx: &mut Tx<'_>, thread_id: i64, name: &str) -> Result<Self> {
        let now = tx.now();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "thread_tags" ("channel_thread_id", "created_at", "name", "updated_at") VALUES (?, ?, ?, ?) RETURNING "id""#,
            params![thread_id, now, name, now],
            |r| r.get(0),
        )?;
        Self::register_row_callback(tx, id, thread_id)?;
        Ok(Self { id, channel_thread_id: thread_id, name: name.into(), created_at: now, updated_at: now })
    }

    /// `validates :name, presence: true, length: { maximum: 30 }, format: NAME_FORMAT,
    /// uniqueness: { scope: :channel_thread_id }` for a new tag.
    pub fn validate(conn: &Connection, thread_id: i64, name: &str) -> Result<Errors> {
        let mut errors = Errors::default();
        if ChannelThread::find_by_id(conn, thread_id)?.is_none() {
            errors.add("channel_thread", "must exist");
        }
        Self::validate_name(conn, thread_id, name, errors)
    }

    fn validate_name(conn: &Connection, thread_id: i64, name: &str, mut errors: Errors) -> Result<Errors> {
        if name.trim().is_empty() {
            errors.add("name", "can't be blank");
        }
        if name.chars().count() > TAG_NAME_LIMIT {
            errors.add("name", format!("is too long (maximum is {TAG_NAME_LIMIT} characters)"));
        }
        if !valid_tag_name(name) && !Self::catalog_name(conn, thread_id, name)? {
            errors.add("name", "is invalid");
        }
        let taken = sql::exists(
            conn,
            r#"SELECT 1 AS one FROM "thread_tags" WHERE "thread_tags"."name" = ? AND "thread_tags"."channel_thread_id" = ? LIMIT 1"#,
            params![name, thread_id],
        )?;
        if taken {
            errors.add("name", "has already been taken");
        }
        Ok(errors)
    }

    fn catalog_name(conn: &Connection, thread_id: i64, name: &str) -> Result<bool> {
        sql::exists(conn,
            "SELECT 1 FROM board_tags JOIN channel_threads ON channel_threads.room_id=board_tags.room_id WHERE channel_threads.id=? AND board_tags.name=?",
            params![thread_id, name])
    }

    /// `destroy`
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn().execute_cached(r#"DELETE FROM "thread_tags" WHERE "thread_tags"."id" = ?"#, [self.id])?;
        Self::register_row_callback(tx, self.id, self.channel_thread_id)?;
        Ok(())
    }

    fn register_row_callback(tx: &mut Tx<'_>, id: i64, thread_id: i64) -> Result<()> {
        // A work-field save may already have queued this thread's final sync snapshot.
        if !tx.has_commit_record("board_post_creation", thread_id)
            && !crate::models::channel_thread::ThreadWorkChange::pending(tx, thread_id)
            && let Some(thread) = ChannelThread::find_by_id(tx.conn(), thread_id)?
            && thread.board_post(tx.conn())?
        {
            crate::models::channel_thread::ThreadWorkChange::emit(tx, thread_id);
        }
        tx.after_commit_record_latest("thread_tag_board_row", id, move |tx| {
            if let Some(thread) = ChannelThread::find_by_id(tx.conn(), thread_id)? && thread.board_post(tx.conn())? {
                thread.broadcast_board_row_replace(tx)?;
            }
            Ok(())
        });
        Ok(())
    }
}

/// `ThreadTag::NAME_FORMAT`: `/\A[a-z0-9][a-z0-9-]*\z/`
pub fn valid_tag_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z' | b'0'..=b'9'))
        && bytes.all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-'))
}

pub const BOARD_TAG_LIMIT: usize = 20;
pub const BOARD_TAG_NAME_LIMIT: usize = 20;

#[derive(Debug, Clone, PartialEq)]
pub struct BoardTag {
    pub id: i64,
    pub room_id: i64,
    pub name: String,
    pub emoji: Option<String>,
    pub position: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl BoardTag {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?, room_id: row.get("room_id")?, name: row.get("name")?,
            emoji: row.get("emoji")?, position: row.get("position")?,
            created_at: row.get("created_at")?, updated_at: row.get("updated_at")?,
        })
    }

    pub fn for_room(conn: &Connection, room_id: i64) -> Result<Vec<Self>> {
        query_all(conn, "SELECT * FROM board_tags WHERE room_id=? ORDER BY position,id", [room_id], Self::from_row)
    }

    pub fn find(conn: &Connection, room_id: i64, id: i64) -> Result<Self> {
        query_one(conn, "SELECT * FROM board_tags WHERE room_id=? AND id=?", params![room_id,id], Self::from_row)?
            .or_not_found("BoardTag")
    }

    fn validate(conn: &Connection, room_id: i64, name: &str, id: Option<i64>) -> Result<()> {
        BoardTagPolicy::for_room(conn, room_id)?;
        let mut errors = Errors::default();
        if name.is_empty() { errors.add("name", "can't be blank"); }
        if name.chars().count() > BOARD_TAG_NAME_LIMIT {
            errors.add("name", "is too long (maximum is 20 characters)");
        }
        let tags = Self::for_room(conn, room_id)?;
        if id.is_none() && tags.len() >= BOARD_TAG_LIMIT {
            errors.add("tags", "are limited to 20 per board");
        }
        if tags.iter().any(|tag| Some(tag.id) != id && rails_compat::unicode::downcase(&tag.name) == rails_compat::unicode::downcase(name)) {
            errors.add("name", "has already been taken");
        }
        errors.into_result()
    }

    pub fn create(tx: &mut Tx<'_>, room_id: i64, name: &str, emoji: Option<&str>) -> Result<Self> {
        let name = name.trim();
        Self::validate(tx.conn(), room_id, name, None)?;
        let position: i64 = tx.conn().query_row("SELECT COALESCE(MAX(position)+1,0) FROM board_tags WHERE room_id=?", [room_id], |r| r.get(0))?;
        let id = tx.conn().query_row("INSERT INTO board_tags(room_id,name,emoji,position,created_at,updated_at) VALUES(?,?,?,?,?,?) RETURNING id",
            params![room_id,name,emoji,position,tx.now(),tx.now()], |r| r.get(0))?;
        Self::find(tx.conn(), room_id, id)
    }

    fn uses(&self, conn: &Connection) -> Result<Vec<ThreadTag>> {
        query_all(conn, "SELECT thread_tags.* FROM thread_tags JOIN channel_threads ON channel_threads.id=thread_tags.channel_thread_id WHERE channel_threads.room_id=? AND lower(thread_tags.name)=lower(?)",
            params![self.room_id,self.name], ThreadTag::from_row)
    }

    pub fn update(tx: &mut Tx<'_>, room_id: i64, id: i64, name: &str, emoji: Option<&str>) -> Result<Self> {
        let prior = Self::find(tx.conn(), room_id, id)?;
        let name = name.trim();
        Self::validate(tx.conn(), room_id, name, Some(id))?;
        tx.conn().execute("UPDATE board_tags SET name=?,emoji=?,updated_at=? WHERE id=?", params![name,emoji,tx.now(),id])?;
        if name != prior.name {
            for tag in prior.uses(tx.conn())? {
                // A renamed catalog label can already be present as a free-text tag.
                for duplicate in ThreadTag::for_thread(tx.conn(), tag.channel_thread_id)?.into_iter()
                    .filter(|other| other.id != tag.id && rails_compat::unicode::downcase(&other.name) == rails_compat::unicode::downcase(name))
                {
                    duplicate.destroy(tx)?;
                }
                tx.conn().execute("UPDATE thread_tags SET name=?,updated_at=? WHERE id=?", params![name,tx.now(),tag.id])?;
                ThreadTag::register_row_callback(tx, tag.id, tag.channel_thread_id)?;
            }
        }
        Self::find(tx.conn(), room_id, id)
    }

    pub fn destroy(tx: &mut Tx<'_>, room_id: i64, id: i64) -> Result<()> {
        let tag = Self::find(tx.conn(), room_id, id)?;
        for used in tag.uses(tx.conn())? { used.destroy(tx)?; }
        tx.conn().execute("DELETE FROM board_tags WHERE id=?", [id])?;
        Ok(())
    }

    pub fn reorder(tx: &mut Tx<'_>, room_id: i64, ids: &[i64]) -> Result<()> {
        BoardTagPolicy::for_room(tx.conn(), room_id)?;
        let tags = Self::for_room(tx.conn(), room_id)?;
        let unique = ids.iter().copied().collect::<std::collections::HashSet<_>>();
        if ids.len() != tags.len() || unique.len() != tags.len() || tags.iter().any(|tag| !unique.contains(&tag.id)) {
            let mut errors = Errors::default();
            errors.add("tag_ids", "must contain every board tag exactly once");
            return errors.into_result();
        }
        for (position, id) in ids.iter().enumerate() {
            tx.conn().execute("UPDATE board_tags SET position=?,updated_at=? WHERE id=?", params![position as i64,tx.now(),id])?;
        }
        Ok(())
    }

    pub fn canonical_name(conn: &Connection, room_id: i64, name: &str) -> Result<Option<String>> {
        let name = rails_compat::unicode::downcase(name.trim());
        Ok(Self::for_room(conn, room_id)?.into_iter()
            .find(|tag| rails_compat::unicode::downcase(&tag.name) == name).map(|tag| tag.name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardTagPolicy {
    pub tags_required: bool,
    pub default_board_tag_id: Option<i64>,
}

impl BoardTagPolicy {
    pub fn for_room(conn: &Connection, room_id: i64) -> Result<Self> {
        let room = Room::find(conn, room_id)?;
        if !room.board() || room.deleted_at.is_some() { return Err(crate::Error::RecordNotFound("Room")); }
        Ok(conn.query_row("SELECT tags_required,default_board_tag_id FROM rooms WHERE id=?", [room_id], |r| {
            Ok(Self { tags_required: r.get(0)?, default_board_tag_id: r.get(1)? })
        })?)
    }

    pub fn update(tx: &mut Tx<'_>, room_id: i64, required: bool, default_id: Option<i64>) -> Result<Self> {
        Self::for_room(tx.conn(), room_id)?;
        if let Some(id) = default_id
            && !sql::exists(tx.conn(), "SELECT 1 FROM board_tags WHERE room_id=? AND id=?", params![room_id,id])? {
            let mut errors = Errors::default();
            errors.add("default_board_tag_id", "must belong to this board");
            errors.into_result()?;
        }
        tx.conn().execute("UPDATE rooms SET tags_required=?,default_board_tag_id=?,updated_at=? WHERE id=?", params![required,default_id,tx.now(),room_id])?;
        Ok(Self { tags_required: required, default_board_tag_id: default_id })
    }

    pub(crate) fn post_tags(conn: &Connection, room_id: i64, names: &[String]) -> Result<Vec<String>> {
        let catalog = BoardTag::for_room(conn, room_id)?;
        let mut tags = Vec::new();
        for name in crate::models::channel_thread::normalize_tag_names(names) {
            let name = catalog.iter().find(|tag| rails_compat::unicode::downcase(&tag.name) == name)
                .map_or(name.clone(), |tag| tag.name.clone());
            if !tags.contains(&name) { tags.push(name); }
        }
        let policy = Self::for_room(conn, room_id)?;
        if policy.tags_required && !catalog.iter().any(|tag| tags.contains(&tag.name)) {
            if let Some(default) = catalog.iter().find(|tag| Some(tag.id) == policy.default_board_tag_id) {
                tags.push(default.name.clone());
            } else {
                let mut errors = Errors::default();
                errors.add("tags", "must include a catalog tag");
                errors.into_result()?;
            }
        }
        Ok(tags)
    }
}
