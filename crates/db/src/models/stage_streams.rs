//! Stage stream authorization/order from Rooms::Stage::StreamsController.
use super::stream::{QUALITIES, Stream};
use crate::sql;
use crate::{CachedStatements, Membership, Result, Room, StageRole, Tx, User};
use jiff::SignedDuration;
use rusqlite::params;

#[derive(Debug)]
pub enum Denial {
    NotFound,
    Forbidden(&'static str),
    UnknownQuality,
    AlreadyLive(String),
}
fn scope(tx: &Tx<'_>, room_id: i64, user_id: i64) -> Result<Option<(Room, Membership)>> {
    let Some(member) = Membership::find_by_room_and_user(tx.conn(), room_id, user_id)? else {
        return Ok(None);
    };
    let room = Room::find(tx.conn(), room_id)?;
    Ok((room.stage() && room.deleted_at.is_none()).then_some((room, member)))
}
fn conflict(tx: &Tx<'_>, room_id: i64) -> Result<Denial> {
    let name = Stream::live_for_room(tx.conn(), room_id)?
        .map(|stream| User::find(tx.conn(), stream.user_id).map(|user| user.name))
        .transpose()?
        .unwrap_or_else(|| "Someone".into());
    Ok(Denial::AlreadyLive(name))
}
pub fn start(
    tx: &mut Tx<'_>,
    room_id: i64,
    user_id: i64,
    quality: &str,
) -> Result<std::result::Result<Stream, Denial>> {
    let Some((_, member)) = scope(tx, room_id, user_id)? else {
        return Ok(Err(Denial::NotFound));
    };
    if !matches!(
        member.stage_role,
        Some(StageRole::Host | StageRole::Speaker)
    ) {
        return Ok(Err(Denial::Forbidden(
            "Only hosts and speakers can go live",
        )));
    }
    if !sql::exists(
        tx.conn(),
        "SELECT 1 FROM memberships WHERE room_id=? AND stage_role='host' LIMIT 1",
        [room_id],
    )? {
        return Ok(Err(Denial::Forbidden("The stage needs a host to go live")));
    }
    if member.server_muted_at.is_some() {
        return Ok(Err(Denial::Forbidden("Muted members cannot go live")));
    }
    let seen=tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM huddle_grants WHERE room_id=? AND membership_id=? AND revoked_at IS NULL AND last_seen_at>?)",params![room_id,member.id,tx.now().ago(SignedDuration::from_secs(20))],|r|r.get::<_,bool>(0))?;
    if !seen {
        return Ok(Err(Denial::Forbidden("Join the stage before going live")));
    }
    if !QUALITIES.contains(&quality) {
        return Ok(Err(Denial::UnknownQuality));
    }
    if Stream::live_for_room(tx.conn(), room_id)?.is_some() {
        return Ok(Err(conflict(tx, room_id)?));
    }
    match Stream::create(tx, room_id, member.id, user_id, quality, None) {
        Ok(stream) => Ok(Ok(stream)),
        Err(error) if error.is_record_not_unique() => Ok(Err(conflict(tx, room_id)?)),
        Err(error) => Err(error),
    }
}
pub fn stop(
    tx: &mut Tx<'_>,
    room_id: i64,
    user_id: i64,
    requested_id: Option<&str>,
) -> Result<std::result::Result<(), Denial>> {
    let Some((_, member)) = scope(tx, room_id, user_id)? else {
        return Ok(Err(Denial::NotFound));
    };
    let mut stream = Stream::live_for_room(tx.conn(), room_id)?;
    if member.stage_role != Some(StageRole::Host)
        && !User::find(tx.conn(), user_id)?.is_administrator()
        && !stream
            .as_ref()
            .is_some_and(|s| s.membership_id == member.id)
    {
        return Ok(Err(Denial::Forbidden(
            "Only the presenter or a host can stop the stream",
        )));
    }
    if let Some(stream) = stream
        .as_mut()
        .filter(|s| requested_id.is_none_or(|id| id == s.id.to_string()))
    {
        stream.end(tx, Some(user_id))?;
    }
    Ok(Ok(()))
}
