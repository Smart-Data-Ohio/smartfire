//! Persisted liveness from `app/models/stream.rb`, `end_stale_live!`.
//! Stale ends use the same Stream lifecycle and commit callbacks as explicit ends.
use crate::sql::query_all;
use crate::{CachedStatements, Connection, Result, Tx};
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
    let row=tx.conn().query_row_cached("SELECT room_id,membership_id FROM streams WHERE id=? AND ended_at IS NULL",[id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?))).optional()?;
    let Some((room, member)) = row else {
        return Ok(());
    };
    let recent=tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM huddle_grants WHERE room_id=? AND membership_id=? AND revoked_at IS NULL AND last_seen_at>?)",params![room,member,tx.now().ago(SignedDuration::from_secs(30))],|r|r.get::<_,bool>(0))?;
    if recent {
        return Ok(());
    }
    if let Some(mut stream) = super::stream::Stream::find_by_id(tx.conn(), id)? {
        stream.end(tx, None)?;
    }
    Ok(())
}
