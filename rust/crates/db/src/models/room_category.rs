//! `app/models/room_category.rb`: per-user sidebar categories, with no parent touches or
//! broadcasts. Deleting a category nullifies membership assignments without touching them.
use crate::error::OptionalExt;
use crate::sql::{CachedStatements, query_all, query_one};
use crate::{Errors, Result, Timestamp, Tx, User};
use rusqlite::{Connection, Row, params};

pub const NAME_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq)]
pub struct RoomCategory {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub collapsed: bool,
    pub position: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl RoomCategory {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            name: row.get("name")?,
            collapsed: row.get("collapsed")?,
            position: row.get("position")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        Self::find_by_id(conn, id)?.or_not_found("RoomCategory")
    }
    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM room_categories WHERE id = ?",
            [id],
            Self::from_row,
        )
    }
    pub fn ordered_for_user(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM room_categories WHERE user_id = ? ORDER BY position, id",
            [user_id],
            Self::from_row,
        )
    }
    pub fn next_position_for(conn: &Connection, user_id: i64) -> Result<i64> {
        Ok(conn.query_row_cached(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM room_categories WHERE user_id = ?",
            [user_id],
            |r| r.get(0),
        )?)
    }
    fn validate(conn: &Connection, user_id: i64, name: &str) -> Result<()> {
        let mut errors = Errors::default();
        if User::find_by_id(conn, user_id)?.is_none() {
            errors.add("user", "must exist");
        }
        if name.trim().is_empty() {
            errors.add("name", "can't be blank");
        }
        if name.chars().count() > NAME_LIMIT {
            errors.add("name", "is too long (maximum is 50 characters)");
        }
        errors.into_result()
    }
    /// Explicit model attributes. The controller supplies `next_position_for` when appending.
    pub fn create(
        tx: &mut Tx<'_>,
        user_id: i64,
        name: &str,
        position: i64,
        collapsed: bool,
    ) -> Result<Self> {
        Self::validate(tx.conn(), user_id, name)?;
        let id = tx.conn().query_row_cached("INSERT INTO room_categories (user_id, name, position, collapsed, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
            params![user_id, name, position, collapsed, tx.now(), tx.now()], |r| r.get(0))?;
        Self::find(tx.conn(), id)
    }
    pub fn update(
        &mut self,
        tx: &mut Tx<'_>,
        name: &str,
        position: i64,
        collapsed: bool,
    ) -> Result<()> {
        Self::validate(tx.conn(), self.user_id, name)?;
        if self.name != name || self.position != position || self.collapsed != collapsed {
            tx.conn().execute_cached("UPDATE room_categories SET name = ?, position = ?, collapsed = ?, updated_at = ? WHERE id = ?",
                params![name, position, collapsed, tx.now(), self.id])?;
            *self = Self::find(tx.conn(), self.id)?;
        }
        Ok(())
    }
    pub fn destroy(&self, tx: &Tx<'_>) -> Result<()> {
        tx.conn().execute_cached(
            "UPDATE memberships SET room_category_id = NULL WHERE room_category_id = ?",
            [self.id],
        )?;
        tx.conn()
            .execute_cached("DELETE FROM room_categories WHERE id = ?", [self.id])?;
        Ok(())
    }
}
