//! Rooms::Stage::{Roles,Hands}Controller policies and committed render descriptions.
use super::huddle_effects::{RoleEvent, StagePanel, StageRoster};
use super::{room_delete::HuddleConfig, stream::Stream};
use crate::{Event, Membership, Result, Room, StageRole, Tx, User};

#[derive(Debug)]
pub enum Denial {
    NotFound,
    TargetNotFound,
    Forbidden,
    PlainForbidden(&'static str),
    UnknownRole,
    ListenerOnly,
}
#[derive(Debug, Clone, Copy)]
pub enum HandTarget {
    Own,
    Other(Option<i64>),
}
fn scope(tx: &Tx<'_>, room_id: i64, user_id: i64) -> Result<Option<Membership>> {
    let Some(member) = Membership::find_by_room_and_user(tx.conn(), room_id, user_id)? else {
        return Ok(None);
    };
    let room = Room::find(tx.conn(), room_id)?;
    Ok((room.stage() && room.deleted_at.is_none()).then_some(member))
}
pub fn change_role(
    tx: &mut Tx<'_>,
    room_id: i64,
    user_id: i64,
    target_id: Option<i64>,
    role: &str,
    config: &HuddleConfig,
) -> Result<std::result::Result<(), Denial>> {
    let Some(viewer) = scope(tx, room_id, user_id)? else {
        return Ok(Err(Denial::NotFound));
    };
    let administrator = User::find(tx.conn(), user_id)?.is_administrator();
    if viewer.stage_role != Some(StageRole::Host) && !administrator {
        return Ok(Err(Denial::Forbidden));
    }
    let Some(mut target) = Membership::for_room(tx.conn(), room_id)?
        .into_iter()
        .find(|m| Some(m.id) == target_id)
    else {
        return Ok(Err(Denial::TargetNotFound));
    };
    if target.user_id != user_id
        && User::find(tx.conn(), target.user_id)?.is_administrator()
        && !administrator
    {
        return Ok(Err(Denial::PlainForbidden(
            "Only administrators can change an administrator's stage role",
        )));
    }
    let Some(role) = StageRole::from_name(role) else {
        return Ok(Err(Denial::UnknownRole));
    };
    let before = target.stage_role;
    target.change_stage_role_with_config(tx, role, config)?;
    if role == StageRole::Listener {
        Stream::end_for_membership(tx, room_id, target.id)?;
    }
    tx.emit_after_commit(Event::broadcast(&StageRoster { room_id }));
    tx.emit_after_commit(Event::broadcast(&StagePanel {
        room_id,
        membership_id: target.id,
    }));
    if (before == Some(StageRole::Listener)) != (role == StageRole::Listener) {
        tx.emit_after_commit(Event::broadcast(&RoleEvent {
            room_id,
            membership_id: target.id,
        }));
    }
    Ok(Ok(()))
}
pub fn raise_hand(
    tx: &mut Tx<'_>,
    room_id: i64,
    user_id: i64,
) -> Result<std::result::Result<(), Denial>> {
    let Some(mut member) = scope(tx, room_id, user_id)? else {
        return Ok(Err(Denial::NotFound));
    };
    if member.stage_role != Some(StageRole::Listener) {
        return Ok(Err(Denial::ListenerOnly));
    }
    if member.raise_hand(tx)? {
        tx.emit_after_commit(Event::broadcast(&StageRoster { room_id }));
    }
    Ok(Ok(()))
}
pub fn lower_hand(
    tx: &mut Tx<'_>,
    room_id: i64,
    user_id: i64,
    target: HandTarget,
) -> Result<std::result::Result<(), Denial>> {
    let Some(viewer) = scope(tx, room_id, user_id)? else {
        return Ok(Err(Denial::NotFound));
    };
    let mut target = match target {
        HandTarget::Own => viewer,
        HandTarget::Other(id) => {
            if viewer.stage_role != Some(StageRole::Host)
                && !User::find(tx.conn(), user_id)?.is_administrator()
            {
                return Ok(Err(Denial::PlainForbidden(
                    "Only hosts can lower another member's hand",
                )));
            }
            let Some(target) = Membership::for_room(tx.conn(), room_id)?
                .into_iter()
                .find(|m| Some(m.id) == id)
            else {
                return Ok(Err(Denial::TargetNotFound));
            };
            target
        }
    };
    target.lower_hand(tx)?;
    tx.emit_after_commit(Event::broadcast(&StageRoster { room_id }));
    Ok(Ok(()))
}
