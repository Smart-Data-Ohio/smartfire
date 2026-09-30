//! `Huddle::JoinNotifier` and the grant's call-ended JSON callbacks.
//! Push requests carry a payload to WS17's policy/transport seam, atomically with the notice.
use crate::broadcasts::Broadcast;
use crate::models::huddle_grant::{HuddleGrant, IN_CALL_WINDOW};
use crate::sql::query_all;
use crate::{
    ActivityItem, CachedStatements, Connection, Event, Involvement, Job, Membership, Result, Room,
    Timestamp, Tx, User,
};
use jiff::SignedDuration;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushPayload {
    pub title: String,
    pub body: String,
    pub path: String,
    pub tag: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushKind {
    Huddle,
    HuddleJoin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushInvitationJob {
    pub activity_item_id: i64,
}
impl Job for PushInvitationJob {
    const CLASS: &'static str = "Huddle::PushInvitationJob";
}

/// WS17 supplies Notifications::Policy and the durable delivery handler. `prepare_push` owns
/// the huddle subscription scopes and conditional join throttle. A request authorizes no
/// delivery on its own; this class deliberately has no stub handler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushRequest {
    pub kind: PushKind,
    pub recipient_id: i64,
    pub sender_id: i64,
    pub room_id: i64,
    pub room_membership_id: Option<i64>,
    pub payload: PushPayload,
}
impl Job for PushRequest {
    const CLASS: &'static str = "Notifications::HuddlePushJob";
}

pub fn enqueue_huddle_push(tx: &mut Tx<'_>, request: &PushRequest) {
    tx.emit_after_commit(Event::job(request));
}

pub const JOIN_PUSH_THROTTLE_WINDOW: i64 = 600;

#[derive(Debug, Clone)]
pub struct PushDelivery {
    pub payload: PushPayload,
    pub subscriptions: Vec<crate::PushSubscription>,
}

/// WS17 calls its policy on this transaction's current records, then passes the decision here.
/// It must enqueue delivery in this same transaction so a rejected enqueue rolls the throttle
/// back. Invitation queue calls may carry zero subscriptions; joins never burn an empty window.
pub fn prepare_push(
    tx: &mut Tx<'_>,
    request: &PushRequest,
    policy_allowed: bool,
) -> Result<Option<PushDelivery>> {
    if request.kind == PushKind::HuddleJoin
        && !invitations_enabled(tx.conn(), request.recipient_id)?
    {
        return Ok(None);
    }
    if !policy_allowed {
        return Ok(None);
    }
    let excluded = if request.kind == PushKind::HuddleJoin {
        "'invisible','nothing','muted'"
    } else {
        "'invisible','nothing'"
    };
    // Membership.disconnected is a timestamp scope, independent of its connection counter.
    let ids: Vec<i64> = query_all(
        tx.conn(),
        &format!(
            "SELECT s.id FROM push_subscriptions s JOIN users u ON u.id=s.user_id JOIN memberships m ON m.user_id=u.id WHERE u.id=? AND m.room_id=? AND m.involvement NOT IN ({excluded}) AND (m.connected_at IS NULL OR m.connected_at<?) ORDER BY s.id"
        ),
        params![
            request.recipient_id,
            request.room_id,
            tx.now().ago(SignedDuration::from_secs(60))
        ],
        |r| r.get(0),
    )?;
    if request.kind == PushKind::HuddleJoin {
        if ids.is_empty() {
            return Ok(None);
        }
        let Some(id) = request.room_membership_id else {
            return Ok(None);
        };
        if tx.conn().execute_cached("UPDATE memberships SET last_huddle_join_push_at=? WHERE id=? AND (last_huddle_join_push_at IS NULL OR last_huddle_join_push_at<?)", params![tx.now(),id,tx.now().ago(SignedDuration::from_secs(JOIN_PUSH_THROTTLE_WINDOW))])? != 1 { return Ok(None); }
    }
    let subscriptions = ids
        .into_iter()
        .map(|id| crate::PushSubscription::find(tx.conn(), id))
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(PushDelivery {
        payload: request.payload.clone(),
        subscriptions,
    }))
}

pub fn notify_join(tx: &mut Tx<'_>, grant_id: i64) -> Result<()> {
    let Some(grant) = HuddleGrant::find_by_id(tx.conn(), grant_id)? else {
        return Ok(());
    };
    if grant.revoked() || !grant.in_call(tx.now()) {
        return Ok(());
    }
    let Some(room) = Room::find_by_id(tx.conn(), grant.room_id)?.filter(|r| r.deleted_at.is_none())
    else {
        return Ok(());
    };
    let Some(joiner) = human(tx.conn(), grant.user_id)? else {
        return Ok(());
    };
    let in_call = in_call_user_ids(tx.conn(), room.id, tx.now())?;
    let members = room_users(tx.conn(), room.id)?;
    let rung: BTreeSet<i64> = query_all(tx.conn(), "SELECT DISTINCT a.user_id FROM activity_items a JOIN huddle_grants g ON g.id=a.source_id WHERE a.source_type='HuddleGrant' AND a.event_type='huddle_started' AND a.handled_at IS NULL AND a.created_at>=? AND g.room_id=?", params![tx.now().ago(SignedDuration::from_secs(60)), room.id], |r| r.get(0))?.into_iter().collect();
    let rejoin: bool = tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM huddle_grants WHERE id!=? AND room_id=? AND user_id=? AND revoked_at>=? AND last_seen_at>=?)", params![grant.id, room.id, joiner.id, tx.now().ago(SignedDuration::from_secs(5)), tx.now().ago(SignedDuration::from_secs(IN_CALL_WINDOW))], |r| r.get(0))?;
    for membership in Membership::for_room(tx.conn(), room.id)? {
        if membership.user_id == joiner.id {
            continue;
        }
        let Some(viewer) = human(tx.conn(), membership.user_id)? else {
            continue;
        };
        let viewer_in_call = in_call.contains(&viewer.id);
        if !viewer_in_call
            && (!room.direct()
                || matches!(
                    membership.involvement,
                    Some(Involvement::Nothing | Involvement::Invisible)
                )
                || rung.contains(&viewer.id))
        {
            continue;
        }
        notice(
            tx,
            viewer.id,
            json!({"eventType":"huddle_joined", "roomId":room.id, "roomName":display_name(&room, viewer.id, &viewer.name, &members, true), "roomPath":format!("/rooms/{}",room.id), "joinerId":joiner.id, "joinerName":joiner.name, "inCall":viewer_in_call, "rejoin":rejoin}),
        );
        if !viewer_in_call && invitations_enabled(tx.conn(), viewer.id)? {
            enqueue_huddle_push(
                tx,
                &PushRequest {
                    kind: PushKind::HuddleJoin,
                    recipient_id: viewer.id,
                    sender_id: joiner.id,
                    room_id: room.id,
                    room_membership_id: Some(membership.id),
                    payload: push_payload(&joiner, room.id, true),
                },
            );
        }
    }
    Ok(())
}

pub fn notify_leave(tx: &mut Tx<'_>, grant: &HuddleGrant) -> Result<()> {
    let Some(room) = Room::find_by_id(tx.conn(), grant.room_id)?.filter(|r| r.deleted_at.is_none())
    else {
        return Ok(());
    };
    let Some(leaver) = human(tx.conn(), grant.user_id)? else {
        return Ok(());
    };
    let another: bool = tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM huddle_grants WHERE id!=? AND room_id=? AND user_id=? AND revoked_at IS NULL AND last_seen_at>?)", params![grant.id, room.id, leaver.id, tx.now().ago(SignedDuration::from_secs(IN_CALL_WINDOW))], |r| r.get(0))?;
    if another {
        return Ok(());
    }
    let mut in_call = in_call_user_ids(tx.conn(), room.id, tx.now())?;
    in_call.remove(&leaver.id);
    let members = room_users(tx.conn(), room.id)?;
    if !in_call.is_empty() {
        for id in &in_call {
            if let Some(viewer) = human(tx.conn(), *id)? {
                leave_notice(tx, &room, &leaver, &viewer, &members);
            }
        }
        if room.direct() {
            for membership in Membership::for_room(tx.conn(), room.id)? {
                if membership.user_id == leaver.id || in_call.contains(&membership.user_id) {
                    continue;
                }
                if let Some(viewer) = human(tx.conn(), membership.user_id)? {
                    leave_notice(tx, &room, &leaver, &viewer, &members);
                }
            }
        }
    } else if room.direct() {
        for membership in Membership::for_room(tx.conn(), room.id)? {
            if membership.user_id != leaver.id && human(tx.conn(), membership.user_id)?.is_some() {
                notice(
                    tx,
                    membership.user_id,
                    json!({"eventType":"huddle_ended", "roomId":room.id}),
                );
            }
        }
    }
    Ok(())
}

pub fn call_ended(tx: &mut Tx<'_>, grant: &HuddleGrant) -> Result<()> {
    let Some(room) = Room::find_by_id(tx.conn(), grant.room_id)? else {
        return Ok(());
    };
    let Some(caller) = User::find_by_id(tx.conn(), grant.user_id)? else {
        return Ok(());
    };
    let others: bool = tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM huddle_grants WHERE id!=? AND room_id=? AND revoked_at IS NULL AND last_seen_at>?)", params![grant.id, room.id, tx.now().ago(SignedDuration::from_secs(IN_CALL_WINDOW))], |r| r.get(0))?;
    if others {
        return Ok(());
    }
    let members = room_users(tx.conn(), room.id)?;
    let ids: Vec<i64> = query_all(
        tx.conn(),
        "SELECT a.id FROM activity_items a JOIN huddle_grants g ON g.id=a.source_id WHERE a.source_type='HuddleGrant' AND a.event_type='huddle_started' AND a.handled_at IS NULL AND g.room_id=? ORDER BY a.id",
        [room.id],
        |r| r.get(0),
    )?;
    let mut delivered = BTreeSet::new();
    for id in ids {
        let item = ActivityItem::find(tx.conn(), id)?;
        if item.user_id == caller.id {
            continue;
        }
        let viewer_name = User::find_by_id(tx.conn(), item.user_id)?
            .map(|user| user.name)
            .unwrap_or_default();
        let state = if item.handled_at.is_some() {
            "handled"
        } else if item.read_at.is_some() {
            "read"
        } else {
            "unread"
        };
        activity(
            tx,
            item.user_id,
            id,
            json!({"activityItemId":id, "eventType":"huddle_ended", "state":state, "roomId":room.id, "roomName":display_name(&room,item.user_id,&viewer_name,&members,false), "roomPath":format!("/rooms/{}",room.id), "callerName":caller.name, "readPath":format!("/activity/{id}/read?state=read"), "handledPath":format!("/activity/{id}/handled?state=handled")}),
        );
        delivered.insert(item.user_id);
    }
    if room.direct()
        && grant
            .last_issued_at
            .is_some_and(|issued| issued > tx.now().ago(SignedDuration::from_secs(60)))
    {
        for membership in Membership::for_room(tx.conn(), room.id)? {
            if membership.user_id == caller.id
                || delivered.contains(&membership.user_id)
                || matches!(
                    membership.involvement,
                    Some(Involvement::Invisible | Involvement::Nothing)
                )
            {
                continue;
            }
            let Some(viewer) = human(tx.conn(), membership.user_id)? else {
                continue;
            };
            if invitations_enabled(tx.conn(), viewer.id)? {
                continue;
            }
            activity(
                tx,
                viewer.id,
                0,
                json!({"activityItemId":0, "eventType":"huddle_ended", "state":"unread", "roomId":room.id, "roomName":display_name(&room,viewer.id,&viewer.name,&members,false), "roomPath":format!("/rooms/{}",room.id), "callerName":caller.name, "readPath":"", "handledPath":""}),
            );
        }
    }
    Ok(())
}

pub fn push_invitation(tx: &mut Tx<'_>, item_id: i64) -> Result<()> {
    let item = match ActivityItem::find(tx.conn(), item_id) {
        Ok(item) => item,
        Err(crate::Error::RecordNotFound(_)) => return Ok(()),
        Err(error) => return Err(error),
    };
    if item.source_type != "HuddleGrant" {
        return Ok(());
    }
    let Some(grant) = HuddleGrant::find_by_id(tx.conn(), item.source_id)? else {
        return Ok(());
    };
    let Some(room) = Room::find_by_id(tx.conn(), grant.room_id)? else {
        return Ok(());
    };
    let Some(caller) = User::find_by_id(tx.conn(), grant.user_id)? else {
        return Ok(());
    };
    if User::find_by_id(tx.conn(), item.user_id)?.is_none() {
        return Ok(());
    }
    let membership = Membership::find_by_room_and_user(tx.conn(), room.id, item.user_id)?;
    enqueue_huddle_push(
        tx,
        &PushRequest {
            kind: PushKind::Huddle,
            recipient_id: item.user_id,
            sender_id: caller.id,
            room_id: room.id,
            room_membership_id: membership.map(|m| m.id),
            payload: push_payload(&caller, room.id, false),
        },
    );
    Ok(())
}

fn push_payload(sender: &User, room_id: i64, join: bool) -> PushPayload {
    PushPayload {
        title: format!(
            "{} {}",
            sender.name,
            if join {
                "joined your huddle"
            } else {
                "started a huddle"
            }
        ),
        body: "Join from the conversation".into(),
        path: format!("/rooms/{room_id}"),
        tag: format!("huddle-{room_id}"),
    }
}
fn notice(tx: &mut Tx<'_>, viewer: i64, payload: serde_json::Value) {
    tx.emit_after_commit(Event::broadcast(&Broadcast::Cable {
        stream: format!("user_{viewer}_huddle_notices"),
        payload: json!({"huddleJoinNotice":payload}),
    }));
}
fn activity(tx: &mut Tx<'_>, viewer: i64, id: i64, payload: serde_json::Value) {
    tx.emit_after_commit(Event::broadcast(&Broadcast::Cable {
        stream: format!("user_{viewer}_activity"),
        payload: json!({"activityItemId":id, "huddleInvitation":payload}),
    }));
}
fn leave_notice(tx: &mut Tx<'_>, room: &Room, leaver: &User, viewer: &User, members: &[User]) {
    notice(
        tx,
        viewer.id,
        json!({"eventType":"huddle_left", "roomId":room.id, "roomName":display_name(room,viewer.id,&viewer.name,members,true), "joinerId":leaver.id, "joinerName":leaver.name}),
    );
}
fn human(conn: &Connection, id: i64) -> Result<Option<User>> {
    Ok(User::find_by_id(conn, id)?.filter(|u| u.is_active() && !u.is_bot()))
}
fn in_call_user_ids(conn: &Connection, room: i64, now: Timestamp) -> Result<BTreeSet<i64>> {
    Ok(query_all(conn,"SELECT DISTINCT user_id FROM huddle_grants WHERE room_id=? AND revoked_at IS NULL AND last_seen_at>?",params![room,now.ago(SignedDuration::from_secs(IN_CALL_WINDOW))],|r|r.get(0))?.into_iter().collect())
}
fn room_users(conn: &Connection, room: i64) -> Result<Vec<User>> {
    query_all(
        conn,
        "SELECT u.* FROM users u JOIN memberships m ON m.user_id=u.id WHERE m.room_id=? ORDER BY LOWER(u.name)",
        [room],
        User::from_row,
    )
}
fn display_name(
    room: &Room,
    viewer: i64,
    viewer_name: &str,
    members: &[User],
    ruby_sort: bool,
) -> String {
    if !room.direct()
        || room
            .name
            .as_deref()
            .is_some_and(|name| !campfire_richtext::ruby::is_blank(name))
    {
        return room.name.clone().unwrap_or_default();
    }
    let mut members = members.iter().collect::<Vec<_>>();
    if ruby_sort {
        members.sort_by_key(|u| u.name.to_lowercase());
    }
    let others = members
        .iter()
        .filter(|u| u.id != viewer)
        .collect::<Vec<_>>();
    match others.len() {
        0 => viewer_name.to_string(),
        1 => others[0].name.clone(),
        count => {
            let firsts = others
                .iter()
                .take(3)
                .map(|u| {
                    // Ruby String#split without an argument recognizes ASCII whitespace.
                    u.name
                        .split([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
                        .find(|part| !part.is_empty())
                        .unwrap_or("")
                })
                .collect::<Vec<_>>()
                .join(", ");
            if count > 3 {
                format!("{firsts} +{}", count - 3)
            } else {
                firsts
            }
        }
    }
}
pub(crate) fn invitations_enabled(conn: &Connection, user: i64) -> Result<bool> {
    let raw: Option<String> = conn
        .query_row_cached(
            "SELECT inbox_preferences FROM users WHERE id=?",
            [user],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    let value: serde_json::Value = raw
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    Ok(match value.get("huddle_invitations") {
        Some(serde_json::Value::Bool(false)) => false,
        Some(serde_json::Value::Number(number)) => number.as_f64() != Some(0.0),
        Some(serde_json::Value::String(value)) => !matches!(value.as_str(), "0" | "false"),
        _ => true,
    })
}
