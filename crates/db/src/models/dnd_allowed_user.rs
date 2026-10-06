//! app/models/dnd_allowed_user.rb: DND allowances are independent of user stars.
use crate::sql::query_one;
use crate::{Errors, Result, Timestamp, Tx, User};
use rusqlite::{Connection, Row, params};

#[derive(Clone, Debug, PartialEq)]
pub struct DndAllowedUser {
    pub id: i64,
    pub user_id: i64,
    pub allowed_user_id: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
impl DndAllowedUser {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            allowed_user_id: row.get("allowed_user_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, user_id: i64, allowed_user_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM dnd_allowed_users WHERE user_id=? AND allowed_user_id=?",
            params![user_id, allowed_user_id],
            Self::from_row,
        )
    }
    pub fn create(tx: &mut Tx<'_>, user_id: i64, allowed_user_id: i64) -> Result<Self> {
        let mut errors = Errors::default();
        if User::find_by_id(tx.conn(), user_id)?.is_none() {
            errors.add("user", "must exist");
        }
        let person = User::find_by_id(tx.conn(), allowed_user_id)?;
        if person.is_none() {
            errors.add("allowed_user", "must exist");
        }
        if Self::find(tx.conn(), user_id, allowed_user_id)?.is_some() {
            errors.add("allowed_user_id", "has already been taken");
        }
        if user_id == allowed_user_id {
            errors.add("allowed_user", "must be someone else");
        }
        if person.is_some_and(|u| !u.is_active() || u.is_bot()) {
            errors.add("allowed_user", "must be an active person");
        }
        errors.into_result()?;
        tx.conn().execute("INSERT INTO dnd_allowed_users (user_id,allowed_user_id,created_at,updated_at) VALUES (?,?,?,?)",params![user_id,allowed_user_id,tx.now(),tx.now()])?;
        Self::find(tx.conn(), user_id, allowed_user_id)?
            .ok_or(crate::Error::RecordNotFound("DndAllowedUser"))
    }
    pub fn find_or_create(tx: &mut Tx<'_>, user_id: i64, allowed_user_id: i64) -> Result<Self> {
        if let Some(allowance) = Self::find(tx.conn(), user_id, allowed_user_id)? {
            return Ok(allowance);
        }
        match Self::create(tx, user_id, allowed_user_id) {
            Err(error) if error.is_record_not_unique() => {
                Self::find(tx.conn(), user_id, allowed_user_id)?.ok_or(error)
            }
            result => result,
        }
    }
    /// delete_all intentionally does not touch user/allowance timestamps or callbacks.
    pub fn remove(tx: &Tx<'_>, user_id: i64, allowed_user_id: i64) -> Result<usize> {
        Ok(tx.conn().execute(
            "DELETE FROM dnd_allowed_users WHERE user_id=? AND allowed_user_id=?",
            params![user_id, allowed_user_id],
        )?)
    }
}
