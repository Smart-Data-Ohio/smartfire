//! `reference/app/models/thread_tag.rb`: one row per tag name on a board post. The tag set is
//! written through [`ChannelThread`](crate::ChannelThread)'s `tag_names`; its board row
//! broadcast (`after_commit :broadcast_board_row_replace`) belongs to the board port (WS12).

use rusqlite::{Connection, Row, params};

use crate::database::Tx;
use crate::error::{Errors, OptionalExt, Result};
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

    /// `thread.tags.create!(name:)`
    pub fn create(tx: &mut Tx<'_>, thread_id: i64, name: &str) -> Result<Self> {
        Self::validate(tx.conn(), thread_id, name)?.into_result()?;
        let now = tx.now();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "thread_tags" ("channel_thread_id", "created_at", "name", "updated_at") VALUES (?, ?, ?, ?) RETURNING "id""#,
            params![thread_id, now, name, now],
            |r| r.get(0),
        )?;
        Self::find(tx.conn(), id)
    }

    /// `validates :name, presence: true, length: { maximum: 30 }, format: NAME_FORMAT,
    /// uniqueness: { scope: :channel_thread_id }` for a new tag.
    pub fn validate(conn: &Connection, thread_id: i64, name: &str) -> Result<Errors> {
        let mut errors = Errors::default();
        if name.trim().is_empty() {
            errors.add("name", "can't be blank");
        }
        if name.chars().count() > TAG_NAME_LIMIT {
            errors.add("name", format!("is too long (maximum is {TAG_NAME_LIMIT} characters)"));
        }
        if !valid_tag_name(name) {
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

    /// `destroy`
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn().execute_cached(r#"DELETE FROM "thread_tags" WHERE "thread_tags"."id" = ?"#, [self.id])?;
        Ok(())
    }
}

/// `ThreadTag::NAME_FORMAT`: `/\A[a-z0-9][a-z0-9-]*\z/`
pub fn valid_tag_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z' | b'0'..=b'9'))
        && bytes.all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-'))
}
