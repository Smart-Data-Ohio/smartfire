//! `app/models/rooms/stage.rb`: synchronous last-host departure and succession.
use super::huddle_grant::HuddleGrant;
use super::room_delete::HuddleConfig;
use super::stream::Stream;
use crate::sql::{self, query_all};
use crate::{Connection, Event, Membership, Message, NewMessage, Result, Room, StageRole, Tx, User};

/// Rails' per-instance `live_streams` association cache. A loaded empty
/// association is distinct from an unloaded one and performs no extra query.
pub struct StageRoom {
    pub room: Room,
    live_streams: Option<Vec<Stream>>,
}
impl StageRoom {
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        Ok(Room::find_by_id(conn, id)?.filter(Room::stage).map(|room| Self { room, live_streams: None }))
    }
    pub fn preload_live_streams(&mut self, conn: &Connection) -> Result<()> {
        self.live_streams = Some(Stream::live_for_room(conn, self.room.id)?.into_iter().collect());
        Ok(())
    }
    pub fn loaded_live_streams(&self) -> Option<&[Stream]> {
        self.live_streams.as_deref()
    }
    pub fn live_stream(&self, conn: &Connection) -> Result<Option<Stream>> {
        match &self.live_streams {
            Some(streams) => Ok(streams.first().cloned()),
            None => Stream::live_for_room(conn, self.room.id),
        }
    }
}

pub fn host_departed(
    tx: &mut Tx<'_>,
    room_id: i64,
    departed_host: i64,
    config: &HuddleConfig,
) -> Result<()> {
    let Some(_room) = Room::find_by_id(tx.conn(), room_id)?.filter(Room::stage) else {
        return Ok(());
    };
    // The immediate writer transaction is the room lock used by Rails.
    if sql::exists(
        tx.conn(),
        "SELECT 1 FROM memberships WHERE room_id=? AND stage_role='host' LIMIT 1",
        [room_id],
    )? {
        return Ok(());
    }
    let remaining: Vec<i64> = query_all(
        tx.conn(),
        "SELECT id FROM memberships WHERE room_id=? ORDER BY created_at",
        [room_id],
        |r| r.get(0),
    )?;
    if remaining.is_empty() {
        return Ok(());
    }
    Stream::end_for_room(tx, room_id)?;
    HuddleGrant::revoke_for_room(tx, room_id, config)?;
    let note = Message::create(
        tx,
        NewMessage {
            room_id,
            creator_id: departed_host,
            system_note: true,
            body: Some("The stage ended because the last host left.".into()),
            ..Default::default()
        },
    )?;
    tx.emit_after_commit(Event::broadcast(&super::huddle_effects::StageEndedNote {
        message_id: note.id,
    }));
    let mut successor = remaining[0];
    for id in remaining {
        let member = Membership::find(tx.conn(), id)?;
        let user = User::find(tx.conn(), member.user_id)?;
        if user.is_active() && user.is_administrator() {
            successor = id;
            break;
        }
    }
    Membership::find(tx.conn(), successor)?.change_stage_role(tx, StageRole::Host)?;
    Ok(())
}

pub fn hosted_room_ids(tx: &Tx<'_>, user_id: i64) -> Result<Vec<i64>> {
    query_all(
        tx.conn(),
        "SELECT m.room_id FROM memberships m JOIN rooms r ON r.id=m.room_id WHERE m.user_id=? AND m.stage_role='host' AND r.type='Rooms::Stage' ORDER BY r.id",
        [user_id],
        |r| r.get(0),
    )
}
