//! Persisted liveness from `app/models/stream.rb`, `end_stale_live!`.
//! This completes the reconciler's state sweep. The common Stream start/end render
//! callbacks still belong to the remaining Stream/Stage lifecycle and view slice.
use crate::sql::query_all;
use crate::{CachedStatements, Connection, Errors, Result, Tx};
use jiff::SignedDuration;
use rusqlite::{OptionalExtension, params};

pub fn live_ids(conn: &Connection) -> Result<Vec<i64>> {
    query_all(
        conn,
        "SELECT id FROM streams WHERE ended_at IS NULL ORDER BY id",
        [],
        |r| r.get(0),
    )
}

/// Recheck liveness in the immediate write transaction so a fresh gateway sighting wins
/// over the sweep's earlier read. Only this presenter membership in this room counts.
pub fn end_stale(tx: &mut Tx<'_>, id: i64) -> Result<()> {
    let row=tx.conn().query_row_cached("SELECT room_id,membership_id,quality FROM streams WHERE id=? AND ended_at IS NULL",[id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?))).optional()?;
    let Some((room, member, quality)) = row else {
        return Ok(());
    };
    let recent=tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM huddle_grants WHERE room_id=? AND membership_id=? AND revoked_at IS NULL AND last_seen_at>?)",params![room,member,tx.now().ago(SignedDuration::from_secs(30))],|r|r.get::<_,bool>(0))?;
    if recent {
        return Ok(());
    }
    let mut errors = Errors::default();
    // This Rails app does not require belongs_to associations on Stream updates;
    // imported orphan coordinates still end. Only its declared quality validation runs.
    if !["720p15", "1080p15", "1080p30"].contains(&quality.as_str()) {
        errors.add("quality", "is not included in the list");
    }
    errors.into_result()?;
    tx.conn().execute_cached(
        "UPDATE streams SET ended_at=?,updated_at=? WHERE id=? AND ended_at IS NULL",
        params![tx.now(), tx.now(), id],
    )?;
    Ok(())
}
