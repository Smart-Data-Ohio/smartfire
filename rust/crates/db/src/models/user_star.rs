//! app/models/user_star.rb and user/starring.rb: a private, per-viewer preference.
use crate::sql::{placeholders, query_all, query_one};
use crate::{Errors, Result, Timestamp, Tx, User};
use rusqlite::{Connection, Row, params};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub struct UserStar {
    pub id: i64,
    pub user_id: i64,
    pub starred_user_id: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl UserStar {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            starred_user_id: row.get("starred_user_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, user_id: i64, starred_user_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM user_stars WHERE user_id=? AND starred_user_id=?",
            params![user_id, starred_user_id],
            Self::from_row,
        )
    }

    /// Model writes permit bot starrers and inactive targets; the human-only rule is HTTP policy.
    pub fn create(tx: &mut Tx<'_>, user_id: i64, starred_user_id: i64) -> Result<Self> {
        let mut errors = Errors::default();
        if User::find_by_id(tx.conn(), user_id)?.is_none() {
            errors.add("user", "must exist");
        }
        if User::find_by_id(tx.conn(), starred_user_id)?.is_none() {
            errors.add("starred_user", "must exist");
        }
        if Self::find(tx.conn(), user_id, starred_user_id)?.is_some() {
            errors.add("starred_user_id", "has already been taken");
        }
        if user_id == starred_user_id {
            errors.add("starred_user", "must be someone else");
        }
        errors.into_result()?;
        tx.conn().execute("INSERT INTO user_stars(user_id,starred_user_id,created_at,updated_at) VALUES (?,?,?,?)",
            params![user_id, starred_user_id, tx.now(), tx.now()])?;
        Self::find(tx.conn(), user_id, starred_user_id)?
            .ok_or(crate::Error::RecordNotFound("UserStar"))
    }

    pub fn find_or_create(tx: &mut Tx<'_>, user_id: i64, starred_user_id: i64) -> Result<Self> {
        if let Some(star) = Self::find(tx.conn(), user_id, starred_user_id)? {
            return Ok(star);
        }
        Self::create(tx, user_id, starred_user_id)
    }

    /// delete_all: no touches, broadcasts or notifications to the starred person.
    pub fn remove(tx: &Tx<'_>, user_id: i64, starred_user_id: i64) -> Result<usize> {
        Ok(tx.conn().execute(
            "DELETE FROM user_stars WHERE user_id=? AND starred_user_id=?",
            params![user_id, starred_user_id],
        )?)
    }
}

impl User {
    pub fn starred(&self, conn: &Connection, other_id: i64) -> Result<bool> {
        Ok(UserStar::find(conn, self.id, other_id)?.is_some())
    }

    pub fn starred_ids_among(&self, conn: &Connection, user_ids: &[i64]) -> Result<HashSet<i64>> {
        if user_ids.is_empty() {
            return Ok(HashSet::new());
        }
        let sql = format!(
            "SELECT starred_user_id FROM user_stars WHERE user_id=? AND starred_user_id IN ({})",
            placeholders(user_ids.len())
        );
        Ok(query_all(
            conn,
            &sql,
            rusqlite::params_from_iter(std::iter::once(self.id).chain(user_ids.iter().copied())),
            |row| row.get(0),
        )?
        .into_iter()
        .collect())
    }
}
