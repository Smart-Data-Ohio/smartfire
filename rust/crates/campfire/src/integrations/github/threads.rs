//! `Github::PullRequestThread`: one PR per thread, one thread per PR and room.
use super::pull_requests::PullRequest;
use campfire_db::{ChannelThread, Connection, Errors, Result, Room, Tx};
use rusqlite::{OptionalExtension, params};
#[derive(Clone, Debug)]
pub struct PullRequestThread {
    pub id: i64,
    pub pull_request_id: i64,
    pub room_id: i64,
    pub channel_thread_id: i64,
}
impl PullRequestThread {
    fn from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            pull_request_id: r.get("github_pull_request_id")?,
            room_id: r.get("room_id")?,
            channel_thread_id: r.get("channel_thread_id")?,
        })
    }
    pub fn for_room_pr(
        conn: &Connection,
        room_id: i64,
        pull_request_id: i64,
    ) -> Result<Option<Self>> {
        Ok(conn.query_row("SELECT * FROM github_pull_request_threads WHERE room_id=? AND github_pull_request_id=?",params![room_id,pull_request_id],Self::from_row).optional()?)
    }
    pub fn validate(
        conn: &Connection,
        pull_request_id: i64,
        room_id: i64,
        channel_thread_id: i64,
    ) -> Result<Errors> {
        let mut errors = Errors::default();
        if matches!(
            PullRequest::find(conn, pull_request_id),
            Err(campfire_db::Error::RecordNotFound(_))
        ) {
            errors.add("pull_request", "must exist");
        }
        if Room::find_by_id(conn, room_id)?.is_none() {
            errors.add("room", "must exist");
        }
        if ChannelThread::find_by_id(conn, channel_thread_id)?.is_none() {
            errors.add("channel_thread", "must exist");
        }
        if Self::for_room_pr(conn, room_id, pull_request_id)?.is_some() {
            errors.add("github_pull_request_id", "has already been taken");
        }
        if conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM github_pull_request_threads WHERE channel_thread_id=?)",
            [channel_thread_id],
            |r| r.get::<_, bool>(0),
        )? {
            errors.add("channel_thread_id", "has already been taken");
        }
        Ok(errors)
    }
    pub fn create(
        tx: &Tx<'_>,
        pull_request_id: i64,
        room_id: i64,
        channel_thread_id: i64,
    ) -> Result<Self> {
        Self::validate(tx.conn(), pull_request_id, room_id, channel_thread_id)?.into_result()?;
        Ok(tx.conn().query_row("INSERT INTO github_pull_request_threads (github_pull_request_id,room_id,channel_thread_id,created_at,updated_at) VALUES (?,?,?,?,?) RETURNING *",params![pull_request_id,room_id,channel_thread_id,tx.now(),tx.now()],Self::from_row)?)
    }
    pub fn create_or_reuse(
        tx: &mut Tx<'_>,
        pull_request_id: i64,
        room_id: i64,
        channel_thread_id: i64,
    ) -> Result<Self> {
        let creation = Self::create(tx, pull_request_id, room_id, channel_thread_id);
        Self::recover_creation(tx, pull_request_id, room_id, channel_thread_id, creation)
    }

    pub(super) fn recover_creation(tx: &mut Tx<'_>, pull_request_id:i64, room_id:i64, channel_thread_id:i64, creation:Result<Self>) -> Result<Self> {
        match creation {
            Ok(mapping) => Ok(mapping),
            Err(error)
                if error.is_record_not_unique()
                    || matches!(&error,campfire_db::Error::RecordInvalid(e) if e.0==vec![("github_pull_request_id","has already been taken".into())]) =>
            {
                let winner = Self::for_room_pr(tx.conn(), room_id, pull_request_id)?.ok_or(
                    campfire_db::Error::RecordNotFound("Github::PullRequestThread"),
                )?;
                if winner.channel_thread_id != channel_thread_id {
                    ChannelThread::find(tx.conn(), channel_thread_id)?.destroy(tx)?;
                }
                Ok(winner)
            }
            Err(error) => Err(error),
        }
    }
}
