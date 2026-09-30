//! `HuddleGrant#after_issued!` and `Huddle::InvitationResolver`.
//! WS17 supplies the sound policy; a typed durable ring request preserves the full payload.
use crate::broadcasts::Broadcast;
use crate::models::{
    huddle_grant::HuddleGrant,
    huddle_notices::{self, PushInvitationJob},
};
use crate::sql::query_all;
use crate::{ActivityItem, CachedStatements, Connection, Event, Job, NotificationKind, NotificationPolicy, Result, Room, Timestamp, Tx, User, UserStatusSettings};
use jiff::SignedDuration;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RingRequest {
    pub recipient_id: i64,
    pub sender_id: i64,
    pub invitation: serde_json::Value,
}
impl Job for RingRequest {
    const CLASS: &'static str = "Notifications::HuddleRingJob";
}

/// `Huddle::RingPolicy.ring?`: an override replaces the entire sound decision.
/// Production reads the current WS17 settings/cache and the actual caller's allowance.
pub fn ring_allowed(
    conn: &Connection,
    recipient_id: i64,
    caller_id: Option<i64>,
    now: Timestamp,
    quiet_check: Option<&dyn Fn(&UserStatusSettings) -> bool>,
) -> Result<bool> {
    let recipient = UserStatusSettings::find(conn, recipient_id)?;
    if let Some(check) = quiet_check {
        return Ok(!check(&recipient));
    }
    let exceptions = super::notification_policy::dnd_exceptions_for(conn, &[recipient_id], caller_id)?;
    Ok(NotificationPolicy {
        recipient: Some(&recipient),
        kind: NotificationKind::Huddle,
        room_involvement: None,
        thread_involvement: None,
        mentioned: false,
        reply_to_recipient: false,
        keyword_matched: false,
        dnd_exception: exceptions.contains(&recipient_id),
        now,
    }.sound())
}

pub fn publish_ring_with_policy(
    tx: &mut Tx<'_>,
    request: &RingRequest,
    quiet_check: Option<&dyn Fn(&UserStatusSettings) -> bool>,
) -> Result<()> {
    if User::find_by_id(tx.conn(), request.recipient_id)?.is_none() {
        return Ok(());
    }
    let sound = ring_allowed(tx.conn(), request.recipient_id, Some(request.sender_id), tx.now(), quiet_check)?;
    publish_ring(tx, request, sound)
}

/// WS17 evaluates kind=huddle sound policy, then calls this in its write transaction.
/// The request is not a policy decision and must never be published with an assumed decision.
pub fn publish_ring(tx: &mut Tx<'_>, request: &RingRequest, sound_allowed: bool) -> Result<()> {
    if !User::find_by_id(tx.conn(), request.recipient_id)?
        .is_some_and(|u| u.is_active() && !u.is_bot())
    {
        return Ok(());
    }
    let mut invitation = request.invitation.clone();
    invitation["silent"] = serde_json::Value::Bool(!sound_allowed);
    tx.emit_after_commit(Event::broadcast(&Broadcast::Cable {
        stream: format!("user_{}_activity", request.recipient_id),
        payload: serde_json::json!({"activityItemId":invitation["activityItemId"], "huddleInvitation":invitation}),
    }));
    Ok(())
}

/// Capture the Rails activity payload at the mutation; WS17 owns its sound decision.
/// Returning false preserves the ordinary activity frame for other source/event types.
pub(crate) fn enqueue_item_ring(tx: &mut Tx<'_>, id: i64) -> Result<bool> {
    let item = ActivityItem::find(tx.conn(), id)?;
    if item.source_type != "HuddleGrant"
        || !matches!(item.event_type.as_str(), "huddle_started" | "huddle_missed")
    {
        return Ok(false);
    }
    let Some(grant) = HuddleGrant::find_by_id(tx.conn(), item.source_id)? else {
        return Ok(false);
    };
    let Some(room) = Room::find_by_id(tx.conn(), grant.room_id)? else {
        return Ok(false);
    };
    let Some(caller) = User::find_by_id(tx.conn(), grant.user_id)? else {
        return Ok(false);
    };
    let viewer = User::find(tx.conn(), item.user_id)?;
    let name = if room.direct() {
        let names = query_all(
            tx.conn(),
            "SELECT users.name FROM users INNER JOIN memberships ON users.id=memberships.user_id WHERE memberships.room_id=? AND users.id!=?",
            params![room.id, viewer.id],
            |r| r.get::<_, String>(0),
        )?;
        Some(match names.len() {
            0 => viewer.name.clone(),
            1 => names[0].clone(),
            2 => format!("{} and {}", names[0], names[1]),
            n => format!("{}, and {}", names[..n - 1].join(", "), names[n - 1]),
        })
    } else {
        room.name.clone()
    };
    tx.emit_after_commit(Event::job(&RingRequest {
        recipient_id:viewer.id,sender_id:caller.id,
        invitation:serde_json::json!({
            "activityItemId":item.id,"eventType":item.event_type,
            "state":if item.handled_at.is_some(){"handled"}else if item.read_at.is_some(){"read"}else{"unread"},
            "roomId":room.id,"roomName":name,"roomPath":format!("/rooms/{}",room.id),"callerName":caller.name,
            "readPath":format!("/activity/{}/read?state=read",item.id),
            "handledPath":format!("/activity/{}/handled?state=handled",item.id),
        }),
    }));
    Ok(true)
}

pub(crate) fn after_issued(
    tx: &mut Tx<'_>,
    grant: &HuddleGrant,
    previous_issue: Option<Timestamp>,
) -> Result<()> {
    let open = query_all(
        tx.conn(),
        "SELECT ai.id FROM activity_items ai JOIN huddle_grants g ON g.id=ai.source_id WHERE ai.source_type='HuddleGrant' AND ai.event_type IN ('huddle_started','huddle_missed') AND ai.user_id=? AND g.room_id=? AND ai.handled_at IS NULL ORDER BY ai.id",
        params![grant.user_id, grant.room_id],
        |r| r.get::<_, i64>(0),
    )?;
    for id in open {
        ActivityItem::find(tx.conn(), id)?.mark_handled(tx)?;
    }
    let dedup = tx.now().ago(SignedDuration::from_secs(120));
    if previous_issue.is_some_and(|at|at>=dedup) || tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM huddle_grants WHERE room_id=? AND user_id=? AND id!=? AND created_at>=?)",params![grant.room_id,grant.user_id,grant.id,dedup],|r|r.get::<_,bool>(0))? { return Ok(()); }
    let room = Room::find(tx.conn(), grant.room_id)?;
    if !room.direct() {
        return Ok(());
    }
    let recipients = query_all(
        tx.conn(),
        "SELECT u.* FROM users u JOIN memberships m ON m.user_id=u.id WHERE m.room_id=? AND u.id!=? AND u.status=0 AND u.role!=2 AND (m.involvement IS NULL OR m.involvement NOT IN ('nothing','invisible')) ORDER BY u.id",
        params![room.id, grant.user_id],
        User::from_row,
    )?;
    for recipient in recipients {
        let in_call = tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM huddle_grants WHERE room_id=? AND user_id=? AND last_seen_at>?)",params![room.id,recipient.id,tx.now().ago(SignedDuration::from_secs(20))],|r|r.get::<_,bool>(0))?;
        let recent = tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM activity_items ai JOIN huddle_grants g ON g.id=ai.source_id WHERE ai.source_type='HuddleGrant' AND ai.event_type IN ('huddle_started','huddle_missed') AND ai.user_id=? AND g.room_id=? AND ai.created_at>=?)",params![recipient.id,room.id,dedup],|r|r.get::<_,bool>(0))?;
        if in_call || recent {
            continue;
        }
        if !huddle_notices::invitations_enabled(tx.conn(), recipient.id)? {
            let caller = User::find(tx.conn(), grant.user_id)?;
            let name = room.direct_display_name(tx.conn(), Some(&recipient), None)?;
            tx.emit_after_commit(Event::job(&RingRequest {recipient_id:recipient.id,sender_id:caller.id,invitation:serde_json::json!({"activityItemId":0,"eventType":"huddle_started","state":"unread","roomId":room.id,"roomName":name,"roomPath":format!("/rooms/{}",room.id),"callerName":caller.name,"readPath":"","handledPath":""})}));
            continue;
        }
        let owned = ActivityItem::find_by_user_and_source(
            tx.conn(),
            recipient.id,
            "HuddleGrant",
            grant.id,
        )?;
        let item = if let Some(item) = owned {
            if item.handled_at.is_none()
                && item.event_type == "huddle_started"
                && item.created_at >= dedup
            {
                continue;
            }
            refresh(tx, &item, grant.id)?
        } else {
            let attempt = tx.conn().query_row_cached("SELECT ai.id FROM activity_items ai JOIN huddle_grants g ON g.id=ai.source_id WHERE ai.source_type='HuddleGrant' AND ai.event_type IN ('huddle_started','huddle_missed') AND ai.user_id=? AND g.room_id=? AND g.user_id=? AND ai.handled_at IS NULL AND ai.created_at>=? ORDER BY ai.created_at DESC LIMIT 1",params![recipient.id,room.id,grant.user_id,tx.now().ago(SignedDuration::from_secs(600))],|r|r.get::<_,i64>(0)).optional()?;
            if let Some(id) = attempt {
                let item = ActivityItem::find(tx.conn(), id)?;
                if item.handled_at.is_some() || item.created_at >= dedup {
                    continue;
                }
                refresh(tx, &item, grant.id)?
            } else {
                ActivityItem::refresh_unread(
                    tx,
                    recipient.id,
                    "HuddleGrant",
                    grant.id,
                    "huddle_started",
                )?
            }
        };
        tx.emit_after_commit(Event::job(&PushInvitationJob {
            activity_item_id: item.id,
        }));
    }
    Ok(())
}

fn refresh(tx: &mut Tx<'_>, item: &ActivityItem, source: i64) -> Result<ActivityItem> {
    tx.conn().execute_cached("UPDATE activity_items SET source_id=?,event_type='huddle_started',read_at=NULL,handled_at=NULL,created_at=?,updated_at=? WHERE id=?",params![source,tx.now(),tx.now(),item.id])?;
    ActivityItem::broadcast_change(tx, item.user_id, item.id)?;
    ActivityItem::find(tx.conn(), item.id)
}
/// Also used lazily by WS12's inbox. Revoked grants intentionally count as join evidence,
/// as Rails' resolver tests issuance/liveness without the `active` scope.
pub fn resolve_overdue(tx: &mut Tx<'_>, user_id: Option<i64>) -> Result<()> {
    let ids = overdue_ids(tx.conn(), tx.now(), user_id)?;
    for id in ids {
        resolve_item(tx, id)?;
    }
    Ok(())
}

/// The in-process loop commits one item at a time, matching Rails' per-item transactions.
pub fn overdue_ids(
    conn: &crate::Connection,
    now: Timestamp,
    user_id: Option<i64>,
) -> Result<Vec<i64>> {
    query_all(
        conn,
        "SELECT id FROM activity_items WHERE event_type='huddle_started' AND handled_at IS NULL AND created_at<? AND (? IS NULL OR user_id=?) ORDER BY id",
        params![now.ago(SignedDuration::from_secs(45)), user_id, user_id],
        |r| r.get(0),
    )
}

pub fn resolve_item(tx: &mut Tx<'_>, id: i64) -> Result<()> {
    let item = match ActivityItem::find(tx.conn(), id) {
        Ok(item) => item,
        Err(crate::Error::RecordNotFound(_)) => return Ok(()),
        Err(error) => return Err(error),
    };
    // An issuance or another resolver may have handled it after the loop's read.
    if item.source_type != "HuddleGrant"
        || item.event_type != "huddle_started"
        || item.handled_at.is_some()
        || item.created_at >= tx.now().ago(SignedDuration::from_secs(45))
    {
        return Ok(());
    }
    let Some(grant) = HuddleGrant::find_by_id(tx.conn(), item.source_id)? else {
        return Ok(());
    };
    let joined=tx.conn().query_row_cached("SELECT EXISTS(SELECT 1 FROM huddle_grants WHERE room_id=? AND user_id=? AND (last_issued_at>=? OR last_seen_at>?))",params![grant.room_id,item.user_id,item.created_at,tx.now().ago(SignedDuration::from_secs(20))],|r|r.get::<_,bool>(0))?;
    if joined {
        item.mark_handled(tx)?;
    } else {
        tx.conn().execute_cached(
            "UPDATE activity_items SET event_type='huddle_missed',updated_at=? WHERE id=?",
            params![tx.now(), item.id],
        )?;
        ActivityItem::broadcast_change(tx, item.user_id, item.id)?;
    }
    Ok(())
}
