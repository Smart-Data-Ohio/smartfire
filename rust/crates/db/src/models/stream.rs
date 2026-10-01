//! `app/models/stream.rb`: persisted streams and their common commit callbacks.
use crate::sql::{query_all, query_one};
use crate::{CachedStatements, Connection, Errors, Event, Result, Timestamp, Tx};
use rusqlite::{Row, params};

pub const QUALITIES: [&str; 3] = ["720p15", "1080p15", "1080p30"];

#[derive(Debug, Clone)]
pub struct Stream {
    pub id: i64,
    pub room_id: i64,
    pub membership_id: i64,
    pub user_id: i64,
    pub quality: String,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
impl Stream {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            membership_id: row.get("membership_id")?,
            user_id: row.get("user_id")?,
            quality: row.get("quality")?,
            started_at: row.get("started_at")?,
            ended_at: row.get("ended_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM streams WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn live_for_room(conn: &Connection, room_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM streams WHERE room_id=? AND ended_at IS NULL ORDER BY id LIMIT 1",
            [room_id],
            Self::from_row,
        )
    }
    fn validate_quality(quality: &str) -> Result<()> {
        let mut errors = Errors::default();
        if !QUALITIES.contains(&quality) {
            errors.add("quality", "is not included in the list");
        }
        errors.into_result()
    }
    pub fn create(
        tx: &mut Tx<'_>,
        room_id: i64,
        membership_id: i64,
        user_id: i64,
        quality: &str,
        started_at: Option<Timestamp>,
    ) -> Result<Self> {
        Self::validate_quality(quality)?;
        let id = tx.conn().query_row_cached("INSERT INTO streams (room_id,membership_id,user_id,quality,started_at,created_at,updated_at) VALUES (?,?,?,?,?,?,?) RETURNING id", params![room_id,membership_id,user_id,quality,started_at.unwrap_or(tx.now()),tx.now(),tx.now()], |r|r.get(0))?;
        let stream = Self::find_by_id(tx.conn(), id)?.expect("inserted stream");
        stream.broadcast_changed(tx);
        Ok(stream)
    }
    pub fn live(&self) -> bool {
        self.ended_at.is_none()
    }
    pub fn end(&mut self, tx: &mut Tx<'_>, ended_by: Option<i64>) -> Result<bool> {
        if !self.live() {
            return Ok(false);
        }
        // This Rails app has no belongs_to presence validation on updates. Orphan
        // imported coordinates still end; only its declared quality validation runs.
        Self::validate_quality(&self.quality)?;
        tx.conn().execute_cached(
            "UPDATE streams SET ended_at=?,updated_at=? WHERE id=?",
            params![tx.now(), tx.now(), self.id],
        )?;
        self.ended_at = Some(tx.now());
        self.updated_at = tx.now();
        self.broadcast_changed(tx);
        if ended_by.is_some_and(|actor| actor != self.user_id) {
            tx.emit_after_commit(Event::broadcast(&super::huddle_effects::StreamStopped {
                room_id: self.room_id,
                user_id: self.user_id,
            }));
        }
        Ok(true)
    }
    fn broadcast_changed(&self, tx: &mut Tx<'_>) {
        tx.emit_after_commit(Event::broadcast(&super::huddle_effects::StreamChanged {
            room_id: self.room_id,
        }));
    }
    pub fn end_for_membership(tx: &mut Tx<'_>, room_id: i64, membership_id: i64) -> Result<()> {
        let streams = query_all(
            tx.conn(),
            "SELECT * FROM streams WHERE room_id=? AND membership_id=? AND ended_at IS NULL ORDER BY id",
            params![room_id, membership_id],
            Self::from_row,
        )?;
        for mut stream in streams {
            stream.end(tx, None)?;
        }
        Ok(())
    }
    pub fn end_for_user(tx: &mut Tx<'_>, user_id: i64) -> Result<()> {
        let streams = query_all(
            tx.conn(),
            "SELECT * FROM streams WHERE user_id=? AND ended_at IS NULL ORDER BY id",
            [user_id],
            Self::from_row,
        )?;
        for mut stream in streams {
            stream.end(tx, None)?;
        }
        Ok(())
    }
    pub fn end_for_room(tx: &mut Tx<'_>, room_id: i64) -> Result<()> {
        let streams = query_all(
            tx.conn(),
            "SELECT * FROM streams WHERE room_id=? AND ended_at IS NULL ORDER BY id",
            [room_id],
            Self::from_row,
        )?;
        for mut stream in streams {
            stream.end(tx, None)?;
        }
        Ok(())
    }
}
