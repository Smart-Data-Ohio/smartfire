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
    /// Discussion mappings for the page's referenced PRs, scoped to each message's room.
    pub fn for_messages(
        conn: &Connection,
        ids: &[i64],
    ) -> Result<std::collections::HashMap<(i64, i64), i64>> {
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let sql = "SELECT DISTINCT t.room_id,t.github_pull_request_id,t.channel_thread_id FROM github_pull_request_threads t JOIN messages m ON m.room_id=t.room_id JOIN github_pull_request_references r ON r.message_id=m.id AND r.github_pull_request_id=t.github_pull_request_id WHERE m.id IN (SELECT value FROM json_each(?))";
        Ok(conn
            .prepare(sql)?
            .query_map([serde_json::json!(ids).to_string()], |row| {
                Ok(((row.get(0)?, row.get(1)?), row.get(2)?))
            })?
            .collect::<rusqlite::Result<_>>()?)
    }
    pub fn validate(
        conn: &Connection,
        pull_request_id: i64,
        room_id: i64,
        channel_thread_id: i64,
    ) -> Result<Errors> {
        let mut errors = Errors::default();
        match PullRequest::find(conn, pull_request_id) {
            Ok(_) => {}
            Err(campfire_db::Error::RecordNotFound(_)) => errors.add("pull_request", "must exist"),
            Err(error) => return Err(error),
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
        #[cfg(test)]
        let injected = test_creation_race::take();
        #[cfg(test)]
        let creation = match injected {
            Some(create) => create(tx),
            None => Self::create(tx, pull_request_id, room_id, channel_thread_id),
        };
        #[cfg(not(test))]
        let creation = Self::create(tx, pull_request_id, room_id, channel_thread_id);
        Self::recover_creation(tx, pull_request_id, room_id, channel_thread_id, creation)
    }

    pub(super) fn recover_creation(
        tx: &mut Tx<'_>,
        pull_request_id: i64,
        room_id: i64,
        channel_thread_id: i64,
        creation: Result<Self>,
    ) -> Result<Self> {
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

/// Discuss creates and joins through WS8's real thread hooks. Queue persistence shares the write.
pub fn discuss(
    tx: &mut Tx<'_>,
    room_id: i64,
    user_id: i64,
    pull_request_id: i64,
    parent_id: i64,
) -> Result<PullRequestThread> {
    PullRequest::find(tx.conn(), pull_request_id)?;
    let parent = campfire_db::Message::find(tx.conn(), parent_id)?;
    if parent.room_id!=room_id||parent.thread_id.is_some()||!tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM github_pull_request_references WHERE github_pull_request_id=? AND message_id=?)",params![pull_request_id,parent_id],|r|r.get::<_,bool>(0))? {return Err(campfire_db::Error::RecordNotFound("Message"));}
    if let Some(existing) = PullRequestThread::for_room_pr(tx.conn(), room_id, pull_request_id)? {
        return Ok(existing);
    }
    let thread = ChannelThread::create(
        tx,
        campfire_db::NewChannelThread {
            room_id,
            creator_id: user_id,
            parent_message_id: Some(parent_id),
            ..Default::default()
        },
    )?;
    campfire_db::ThreadMembership::join(tx, thread.id, user_id)?;
    let mapping = PullRequestThread::create_or_reuse(tx, pull_request_id, room_id, thread.id)?;
    if mapping.channel_thread_id == thread.id {
        tx.emit_after_commit(campfire_db::Event::job(&super::jobs::FetchPullRequestJob {
            pull_request_id,
        }));
    }
    Ok(mapping)
}

// The pinned Rails race test replaces create! once, inserts a real winner through
// the original model and then raises either uniqueness exception. Keep precisely
// that producer boundary on the database writer thread; all HTTP/recovery code runs.
#[cfg(test)]
pub(crate) mod test_creation_race {
    use super::*;
    type Create = Box<dyn for<'a> FnOnce(&mut Tx<'a>) -> Result<PullRequestThread>>;
    std::thread_local! {static CREATE: std::cell::RefCell<Option<Create>> = const { std::cell::RefCell::new(None) };}
    pub(crate) fn set(create: Create) {
        CREATE.with(|cell| *cell.borrow_mut() = Some(create));
    }
    pub(super) fn take() -> Option<Create> {
        CREATE.with(|cell| cell.borrow_mut().take())
    }
}
