//! Policy and mutation order from app/controllers/rooms/call_moderation_controller.rb.
use super::huddle_effects::{RoleEvent, StageRoster};
use super::{huddle_grant::HuddleGrant, room_delete::HuddleConfig, stream::Stream};
use crate::{Event, Membership, Result, Room, StageRole, Tx, User};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Mute,
    Unmute,
    Disconnect,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denial {
    NotFound,
    TargetNotFound,
    Forbidden,
    AdministratorRank,
    SelfTarget,
}

pub fn moderate(
    tx: &mut Tx<'_>,
    room_id: i64,
    user_id: i64,
    target_id: Option<i64>,
    action: Action,
    config: &HuddleConfig,
) -> Result<std::result::Result<(), Denial>> {
    let Some(viewer) = Membership::find_by_room_and_user(tx.conn(), room_id, user_id)? else {
        return Ok(Err(Denial::NotFound));
    };
    let room = Room::find(tx.conn(), room_id)?;
    if room.deleted_at.is_some() || (!room.stage() && !room.voice()) {
        return Ok(Err(Denial::NotFound));
    }
    let user = User::find(tx.conn(), user_id)?;
    if !user.is_administrator() && !(room.stage() && viewer.stage_role == Some(StageRole::Host)) {
        return Ok(Err(Denial::Forbidden));
    }
    let Some(mut target) = Membership::for_room(tx.conn(), room_id)?
        .into_iter()
        .find(|m| Some(m.id) == target_id)
    else {
        return Ok(Err(Denial::TargetNotFound));
    };
    let target_user = User::find(tx.conn(), target.user_id)?;
    if target_user.is_administrator() && !user.is_administrator() {
        return Ok(Err(Denial::AdministratorRank));
    }
    if target.id == viewer.id && !(action == Action::Unmute && user.is_administrator()) {
        return Ok(Err(Denial::SelfTarget));
    }
    let changed = match action {
        Action::Mute => {
            let changed = target.set_server_muted(tx, true, config)?;
            Stream::end_for_membership(tx, room_id, target.id)?;
            changed
        }
        Action::Unmute => target.set_server_muted(tx, false, config)?,
        Action::Disconnect => {
            HuddleGrant::revoke_for_membership(tx, target.id, config)?;
            Stream::end_for_membership(tx, room_id, target.id)?;
            false
        }
    };
    if changed {
        if room.stage() {
            tx.emit_after_commit(Event::broadcast(&StageRoster { room_id }));
        }
        tx.emit_after_commit(Event::broadcast(&RoleEvent {
            room_id,
            membership_id: target.id,
        }));
    }
    Ok(Ok(()))
}
