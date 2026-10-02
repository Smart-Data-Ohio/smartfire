//! app/models/board_stale_digest.rb: a per-board/day claim with no uniqueness validation.
use crate::{Connection, Errors, Message, Result, Room, Timestamp, Tx};
use crate::sql::{query_all, query_one};
use jiff::civil::Date;
use rusqlite::{Row, params};

#[derive(Debug, Clone, PartialEq)]
pub struct BoardStaleDigest {
    pub id: i64,
    pub room_id: i64,
    pub digest_on: Date,
    pub message_id: Option<i64>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
impl BoardStaleDigest {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let date: String = row.get("digest_on")?;
        let date = date.parse().map_err(|e| rusqlite::Error::FromSqlConversionFailure(0,rusqlite::types::Type::Text,Box::new(e)))?;
        Ok(Self { id:row.get("id")?,room_id:row.get("room_id")?,digest_on:date,message_id:row.get("message_id")?,created_at:row.get("created_at")?,updated_at:row.get("updated_at")? })
    }
    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(conn,"SELECT * FROM board_stale_digests WHERE id=?",[id],Self::from_row)
    }
    pub fn for_room(conn: &Connection, room_id: i64) -> Result<Vec<Self>> {
        query_all(conn,"SELECT * FROM board_stale_digests WHERE room_id=? ORDER BY id",[room_id],Self::from_row)
    }
    pub fn validate(conn: &Connection, room_id: i64, on: Option<Date>, message_id: Option<i64>) -> Result<Errors> {
        let mut errors = Errors::default();
        if Room::find_by_id(conn,room_id)?.is_none() { errors.add("room","must exist"); }
        if on.is_none() { errors.add("digest_on","can't be blank"); }
        // belongs_to :message is optional; Rails does not require an optional target to exist.
        let _ = message_id;
        Ok(errors)
    }
    /// The claim commits before posting, so a failing note retains the claim like Rails.
    /// `None` is an already claimed day, including when its message is still nil.
    pub fn claim(tx: &mut Tx<'_>, room_id: i64, on: Date) -> Result<Option<Self>> {
        Self::validate(tx.conn(),room_id,Some(on),None)?.into_result()?;
        query_one(tx.conn(),"INSERT INTO board_stale_digests(room_id,digest_on,created_at,updated_at) VALUES(?,?,?,?) ON CONFLICT(room_id,digest_on) DO NOTHING RETURNING *",params![room_id,on.to_string(),tx.now(),tx.now()],Self::from_row)
    }
    pub fn attach_message(&mut self, tx: &mut Tx<'_>, message: &Message) -> Result<()> {
        if self.message_id != Some(message.id) {
            tx.conn().execute("UPDATE board_stale_digests SET message_id=?,updated_at=? WHERE id=?",params![message.id,tx.now(),self.id])?;
            self.message_id=Some(message.id);
            self.updated_at=tx.now();
        }
        Ok(())
    }
}
