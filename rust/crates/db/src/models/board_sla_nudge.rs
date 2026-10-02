//! app/models/board_sla_nudge.rb: one immutable SLA claim per status crossing/stage.
//! The dispatcher authorizes recipients; a stored claim exposes only its recorded recipient.
use crate::sql::{query_all, query_one};
use crate::{
    ActivityItem, ChannelThread, Connection, Errors, Event, Result, Room, Timestamp, Tx, User,
};
use rusqlite::{Row, params};

pub const STAGES: [&str; 2] = ["nudge", "escalation"];

#[derive(Debug, Clone, PartialEq)]
pub struct BoardSlaNudge {
    pub id: i64,
    pub room_id: i64,
    pub channel_thread_id: i64,
    pub recipient_id: i64,
    pub work_status: String,
    pub stage: String,
    pub status_entered_at: Timestamp,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone)]
pub struct NewBoardSlaNudge {
    pub room_id: i64,
    pub channel_thread_id: i64,
    pub recipient_id: i64,
    pub work_status: Option<String>,
    pub stage: Option<String>,
    pub status_entered_at: Option<Timestamp>,
}

impl BoardSlaNudge {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            channel_thread_id: row.get("channel_thread_id")?,
            recipient_id: row.get("recipient_id")?,
            work_status: row.get("work_status")?,
            stage: row.get("stage")?,
            status_entered_at: row.get("status_entered_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM board_sla_nudges WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        Self::find_by_id(conn, id)?.ok_or(crate::Error::RecordNotFound("BoardSlaNudge"))
    }
    /// Preload exactly the authorized inbox page's source IDs, in one read.
    pub fn for_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<Self>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        query_all(
            conn,
            "SELECT * FROM board_sla_nudges WHERE id IN (SELECT value FROM json_each(?))",
            [serde_json::json!(ids).to_string()],
            Self::from_row,
        )
    }
    pub fn waited_minutes(&self, now: Timestamp) -> i64 {
        now.as_microsecond()
            .saturating_sub(self.status_entered_at.as_microsecond())
            .div_euclid(60_000_000)
            .max(0)
    }
    pub fn activity_recipient_ids(&self) -> [i64; 1] {
        [self.recipient_id]
    }
    pub fn recipient_user_ids(&self) -> [i64; 1] {
        self.activity_recipient_ids()
    }

    pub fn validate(
        conn: &Connection,
        input: &NewBoardSlaNudge,
        existing_id: Option<i64>,
    ) -> Result<Errors> {
        let mut errors = Errors::default();
        if Room::find_by_id(conn, input.room_id)?.is_none() {
            errors.add("room", "must exist");
        }
        if ChannelThread::find_by_id(conn, input.channel_thread_id)?.is_none() {
            errors.add("channel_thread", "must exist");
        }
        if User::find_by_id(conn, input.recipient_id)?.is_none() {
            errors.add("recipient", "must exist");
        }
        let status = input.work_status.as_deref();
        if status.is_none_or(campfire_richtext::ruby::is_blank) {
            errors.add("work_status", "can't be blank");
        }
        if status.is_none_or(|value| !super::channel_thread::WORK_STATUSES.contains(&value)) {
            errors.add("work_status", "is not included in the list");
        }
        let stage = input.stage.as_deref();
        if stage.is_none_or(campfire_richtext::ruby::is_blank) {
            errors.add("stage", "can't be blank");
        }
        if stage.is_none_or(|value| !STAGES.contains(&value)) {
            errors.add("stage", "is not included in the list");
        }
        if input.status_entered_at.is_none() {
            errors.add("status_entered_at", "can't be blank");
        }
        let duplicate: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM board_sla_nudges WHERE channel_thread_id=? AND work_status IS ? AND stage IS ? AND status_entered_at IS ? AND id != ?)",
            params![input.channel_thread_id, status, stage, input.status_entered_at, existing_id.unwrap_or(0)], |row| row.get(0))?;
        if duplicate {
            errors.add(
                "status_entered_at",
                "already fired for this status crossing",
            );
        }
        Ok(errors)
    }
    /// Saving the claim alone has no inbox or push callback, as in Rails.
    pub fn create(tx: &mut Tx<'_>, input: NewBoardSlaNudge) -> Result<Self> {
        Self::validate(tx.conn(), &input, None)?.into_result()?;
        let now = tx.now();
        Ok(tx.conn().query_row(
            "INSERT INTO board_sla_nudges(room_id,channel_thread_id,recipient_id,work_status,stage,status_entered_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?) RETURNING *",
            params![input.room_id,input.channel_thread_id,input.recipient_id,input.work_status,input.stage,input.status_entered_at,now,now], Self::from_row)?)
    }
    /// Dispatcher building block. The caller owns stage eligibility and per-sweep push dedupe.
    /// Claim, inbox record and an optional push intent share the caller's source transaction.
    pub fn claim_and_notify(tx: &mut Tx<'_>, input: NewBoardSlaNudge, push: bool) -> Result<Self> {
        tx.savepoint(move |tx| {
            let nudge = Self::create(tx, input)?;
            ActivityItem::record(
                tx,
                nudge.recipient_id,
                super::activity_item::ActivitySource::BoardSlaNudge(nudge.id),
                "work_sla",
                false,
            )?;
            if push {
                tx.emit_after_commit(Event::job(&super::notification_push::BoardNudgeJob {
                    nudge_id: nudge.id,
                }));
            }
            Ok(nudge)
        })
    }
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        ActivityItem::destroy_for_source(tx, "BoardSlaNudge", self.id)?;
        tx.conn()
            .execute("DELETE FROM board_sla_nudges WHERE id=?", [self.id])?;
        Ok(())
    }
}
