use crate::sql::query_all;
use crate::{Connection, Errors, Result, Timestamp, Tx, User};
use rusqlite::params;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserBlock {
    pub blocker_id: i64,
    pub blocked_id: i64,
    pub created_at: Timestamp,
}

impl UserBlock {
    pub fn for_viewer(conn: &Connection, viewer: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT blocker_id,blocked_id,created_at FROM user_blocks WHERE blocker_id=? ORDER BY created_at,blocked_id",
            [viewer],
            |row| {
                Ok(Self {
                    blocker_id: row.get(0)?,
                    blocked_id: row.get(1)?,
                    created_at: row.get(2)?,
                })
            },
        )
    }

    pub fn set(tx: &Tx<'_>, viewer: i64, person: i64, ignored: bool) -> Result<i64> {
        let mut errors = Errors::default();
        if viewer == person {
            errors.add("person", "must be someone else");
        }
        errors.into_result()?;
        User::find(tx.conn(), viewer)?;
        User::find(tx.conn(), person)?;
        let changed = if ignored {
            tx.conn().execute("INSERT INTO user_blocks(blocker_id,blocked_id,created_at) VALUES (?,?,?) ON CONFLICT(blocker_id,blocked_id) DO NOTHING", params![viewer,person,tx.now()])?
        } else {
            tx.conn().execute(
                "DELETE FROM user_blocks WHERE blocker_id=? AND blocked_id=?",
                params![viewer, person],
            )?
        };
        if changed != 0 {
            tx.conn().execute(
                "UPDATE users SET activity_revision=activity_revision+1 WHERE id=?",
                [viewer],
            )?;
        }
        Ok(tx.conn().query_row(
            "SELECT activity_revision FROM users WHERE id=?",
            [viewer],
            |row| row.get(0),
        )?)
    }
}
