//! Event/board pusher execution and WS13's huddle policy/transport seam. Source mutation,
//! reminder/nudge claims, and huddle payload construction belong to WS14/WS12/WS13.
use crate::models::notification_policy::dnd_exceptions_for;
use crate::sql::{query_all, query_one};
use crate::{
    Connection, Event, Job, Membership, NotificationKind, NotificationPolicy, PushPayload,
    PushSubscription, Result, Room, Timestamp, Tx, User, UserStatusSettings,
};
use jiff::SignedDuration;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct PushDelivery {
    pub payload: PushPayload,
    pub subscriptions: Vec<PushSubscription>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventReminderJob {
    pub event_id: i64,
}
impl Job for EventReminderJob {
    const CLASS: &'static str = "Event::ReminderPushJob";
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardNudgeJob {
    pub nudge_id: i64,
}
impl Job for BoardNudgeJob {
    const CLASS: &'static str = "BoardAutomations::NudgePushJob";
}

/// WS13's final wire contract. Its `huddle_notices::PushRequest` serializes this exact
/// shape; no dependency on that owner's unmerged models is required by the transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuddlePushKind {
    Huddle,
    HuddleJoin,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuddlePushRequest {
    pub kind: HuddlePushKind,
    pub recipient_id: i64,
    pub sender_id: i64,
    pub room_id: i64,
    pub room_membership_id: Option<i64>,
    pub payload: PushPayload,
}
impl Job for HuddlePushRequest {
    const CLASS: &'static str = "Notifications::HuddlePushJob";
}

/// Execute a WS13 intent against current policy. This transaction owns the single join
/// throttle claim and the durable delivery enqueue. Do not call WS13's `prepare_push`
/// before or after this adapter: that would claim the same window twice.
pub fn enqueue_huddle_request(tx: &mut Tx<'_>, request: HuddlePushRequest) -> Result<bool> {
    match request.kind {
        HuddlePushKind::Huddle => enqueue_huddle_invitation(
            tx,
            request.room_id,
            request.recipient_id,
            request.sender_id,
            request.payload,
        ),
        HuddlePushKind::HuddleJoin => {
            let membership = Membership::find_by_room_and_user(
                tx.conn(),
                request.room_id,
                request.recipient_id,
            )?;
            if request.room_membership_id.is_none()
                || membership.as_ref().map(|m| m.id) != request.room_membership_id
            {
                return Ok(false);
            }
            enqueue_huddle_join(
                tx,
                request.room_id,
                request.recipient_id,
                request.sender_id,
                request.payload,
            )
        }
    }
}

/// Minimal live source reader until WS14's event model lands. No reminder claim/create here.
#[derive(Debug)]
pub struct EventReminderSource {
    pub id: i64,
    pub room_id: i64,
    pub organizer_id: i64,
    pub title: String,
    pub starts_at: Timestamp,
    pub ends_at: Option<Timestamp>,
    pub venue_room_id: Option<i64>,
}
impl EventReminderSource {
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(conn,"SELECT id,room_id,organizer_id,title,starts_at,ends_at,venue_room_id FROM events WHERE id=?",[id],|r|Ok(Self{id:r.get(0)?,room_id:r.get(1)?,organizer_id:r.get(2)?,title:r.get(3)?,starts_at:r.get(4)?,ends_at:r.get(5)?,venue_room_id:r.get(6)?}))?.ok_or(crate::Error::RecordNotFound("Event"))
    }
    pub fn stale(&self, now: Timestamp) -> bool {
        self.ends_at.is_some_and(|end| end <= now)
            || self.starts_at < now.since(-SignedDuration::from_secs(5 * 60))
    }
    pub fn payload(&self, conn: &Connection, now: Timestamp) -> Result<PushPayload> {
        let room = Room::find(conn, self.room_id)?;
        let title = if room.direct() {
            User::find(conn, self.organizer_id)?.name
        } else {
            room.name.unwrap_or_default()
        };
        let minutes = ((self.starts_at.as_microsecond() - now.as_microsecond()) as f64
            / 60_000_000.0)
            .round() as i64;
        let relative = if minutes <= 0 {
            "Starting now".into()
        } else {
            format!(
                "Starts in {minutes} {}",
                if minutes == 1 { "minute" } else { "minutes" }
            )
        };
        let mut body = format!("{relative}: {}", self.title);
        if let Some(venue) = self.venue_room_id {
            body.push_str(&format!(
                " in {}",
                Room::find(conn, venue)?.name.unwrap_or_default()
            ));
        }
        Ok(PushPayload::new(
            title,
            body,
            format!("/rooms/{}/events/{}", self.room_id, self.id),
            Some(format!("event-{}", self.id)),
        ))
    }
}
fn allowed_reminders(conn: &Connection, ids: &[i64], now: Timestamp) -> Result<Vec<i64>> {
    let users = UserStatusSettings::for_ids(conn, ids)?;
    Ok(ids
        .iter()
        .copied()
        .filter(|id| {
            NotificationPolicy {
                recipient: users.get(id),
                kind: NotificationKind::Reminder,
                room_involvement: None,
                thread_involvement: None,
                mentioned: false,
                reply_to_recipient: false,
                keyword_matched: false,
                dnd_exception: false,
                now,
            }
            .push()
        })
        .collect())
}
pub fn event_reminder_push(
    conn: &Connection,
    id: i64,
    now: Timestamp,
) -> Result<Option<PushDelivery>> {
    let event = EventReminderSource::find(conn, id)?;
    if event.stale(now) {
        return Ok(None);
    }
    let ids = query_all(
        conn,
        "SELECT id FROM users WHERE status=0 AND role!=2 AND id IN (SELECT user_id FROM event_attendances WHERE event_id=? AND response IN ('going','maybe')) AND id IN (SELECT user_id FROM memberships WHERE room_id=?)",
        params![id, event.room_id],
        |r| r.get::<_, i64>(0),
    )?;
    let ids = allowed_reminders(conn, &ids, now)?;
    Ok(Some(PushDelivery {
        payload: event.payload(conn, now)?,
        subscriptions: PushSubscription::for_users(conn, &ids)?,
    }))
}

/// Minimal live source reader until WS12's BoardSlaNudge model lands.
#[derive(Debug)]
pub struct BoardNudgeSource {
    pub id: i64,
    pub room_id: i64,
    pub thread_id: i64,
    pub recipient_id: i64,
    pub work_status: String,
    pub stage: String,
}
impl BoardNudgeSource {
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(conn,"SELECT id,room_id,channel_thread_id,recipient_id,work_status,stage FROM board_sla_nudges WHERE id=?",[id],|r|Ok(Self{id:r.get(0)?,room_id:r.get(1)?,thread_id:r.get(2)?,recipient_id:r.get(3)?,work_status:r.get(4)?,stage:r.get(5)?}))?.ok_or(crate::Error::RecordNotFound("BoardSlaNudge"))
    }
    pub fn payload(&self, conn: &Connection) -> Result<PushPayload> {
        let room = Room::find(conn, self.room_id)?;
        let name: Option<String> = query_one(
            conn,
            "SELECT name FROM channel_threads WHERE id=?",
            [self.thread_id],
            |r| r.get(0),
        )?
        .ok_or(crate::Error::RecordNotFound("ChannelThread"))?;
        let label = match self.work_status.as_str() {
            "planned" => "Planned".into(),
            "in_progress" => "In progress".into(),
            "blocked" => "Blocked".into(),
            "done" => "Done".into(),
            other => humanize(other),
        };
        let prefix = if self.stage == "escalation" {
            "Escalated"
        } else {
            "SLA breach"
        };
        Ok(PushPayload::new(
            room.name.unwrap_or_default(),
            format!("{prefix}: {} sitting in {label}", name.unwrap_or_default()),
            format!("/rooms/{}?thread={}", self.room_id, self.thread_id),
            Some(format!("board-nudge-{}", self.thread_id)),
        ))
    }
}
fn humanize(value: &str) -> String {
    // ActiveSupport::Inflector.humanize and config/initializers/inflections.rb.
    let spaces = value.replace('_', " ");
    let mut text = spaces.trim_start_matches(['\0', ' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    if value.ends_with("_id") { text = text.strip_suffix(" id").unwrap_or(text); }
    let mut result = String::new();
    let mut run = String::new();
    let append = |result: &mut String, run: &mut String| {
        let lowered = rails_compat::unicode::downcase(run);
        result.push_str(match lowered.as_str() { "http" => "HTTP", "oauth" => "OAuth", _ => &lowered });
        run.clear();
    };
    for c in text.chars() {
        if rails_compat::unicode::alphabetic(c) || c.is_ascii_digit() {
            run.push(c);
        } else {
            append(&mut result, &mut run);
            result.push(c);
        }
    }
    append(&mut result, &mut run);
    if let Some(c) = result.chars().next().filter(|c| rails_compat::unicode::alphabetic(*c)) {
        let mut bytes = [0; 4];
        let first = rails_compat::unicode::upcase(c.encode_utf8(&mut bytes));
        result.replace_range(..c.len_utf8(), &first);
    }
    result
}
pub fn board_nudge_push(
    conn: &Connection,
    id: i64,
    now: Timestamp,
) -> Result<Option<PushDelivery>> {
    let nudge = BoardNudgeSource::find(conn, id)?;
    let recipient = UserStatusSettings::find(conn, nudge.recipient_id)?;
    if !recipient.active_human()
        || Membership::find_by_room_and_user(conn, nudge.room_id, nudge.recipient_id)?.is_none()
    {
        return Ok(None);
    }
    if allowed_reminders(conn, &[nudge.recipient_id], now)?.is_empty() {
        return Ok(None);
    }
    Ok(Some(PushDelivery {
        payload: nudge.payload(conn)?,
        subscriptions: PushSubscription::for_user(conn, nudge.recipient_id)?,
    }))
}

/// Source owner builds these bytes at the same point Rails invokes its pusher. The durable
/// adapter replaces its immediate pool handoff; it stores no grant/call lifecycle logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuddleInvitationDeliveryJob {
    pub payload: PushPayload,
    pub subscription_ids: Vec<i64>,
}
impl Job for HuddleInvitationDeliveryJob {
    const CLASS: &'static str = "Huddle::InvitationDeliveryJob";
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuddleJoinDeliveryJob {
    pub payload: PushPayload,
    pub subscription_ids: Vec<i64>,
}
impl Job for HuddleJoinDeliveryJob {
    const CLASS: &'static str = "Huddle::JoinDeliveryJob";
}

fn huddle_inbox_enabled(conn: &Connection, user_id: i64) -> Result<bool> {
    let raw: Option<String> = conn.query_row(
        "SELECT inbox_preferences FROM users WHERE id=?",
        [user_id],
        |r| r.get(0),
    )?;
    let value = raw.and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
    let value = value
        .as_ref()
        .and_then(|v| v.as_object())
        .and_then(|v| v.get("huddle_invitations"));
    // User::InboxPreferences.cast: only recognized false forms turn a default-true key off.
    Ok(!value.is_some_and(|v| {
        v == &serde_json::json!(false) || v == &serde_json::json!(0) || v == "0" || v == "false"
    }))
}
fn huddle_recipient(
    conn: &Connection,
    room_id: i64,
    recipient_id: i64,
    sender_id: i64,
    join: bool,
    now: Timestamp,
) -> Result<Option<(Option<i64>, Vec<i64>)>> {
    // WS13 validates the grant/source and builds the payload. Presence uses the room's
    // connected_at clock, not workspace leases. SQL NULL involvement is scoped out by Rails.
    match Room::find(conn, room_id) {
        Err(crate::Error::RecordNotFound(_)) => return Ok(None),
        Err(error) => return Err(error),
        Ok(_) => {}
    }
    if User::find_by_id(conn, sender_id)?.is_none() {
        return Ok(None);
    }
    let membership = Membership::find_by_room_and_user(conn, room_id, recipient_id)?;
    let excluded = if join {
        "('invisible','nothing','muted')"
    } else {
        "('invisible','nothing')"
    };
    let eligible:bool=conn.query_row(&format!("SELECT EXISTS(SELECT 1 FROM memberships WHERE id=? AND involvement NOT IN {excluded} AND (connected_at IS NULL OR connected_at<?))"),params![membership.as_ref().map(|m|m.id),now.since(-crate::models::membership::CONNECTION_TTL)],|r|r.get(0))?;
    if join && (!eligible || !huddle_inbox_enabled(conn, recipient_id)?) {
        return Ok(None);
    }
    let users = UserStatusSettings::for_ids(conn, &[recipient_id])?;
    let exceptions = dnd_exceptions_for(conn, &[recipient_id], Some(sender_id))?;
    if !(NotificationPolicy {
        recipient: users.get(&recipient_id),
        kind: if join {
            NotificationKind::HuddleJoin
        } else {
            NotificationKind::Huddle
        },
        room_involvement: membership.as_ref().map(|m| m.involvement),
        thread_involvement: None,
        mentioned: false,
        reply_to_recipient: false,
        keyword_matched: false,
        dnd_exception: exceptions.contains(&recipient_id),
        now,
    })
    .push()
    {
        return Ok(None);
    }
    let subscriptions = if eligible {
        PushSubscription::for_user(conn, recipient_id)?
    } else {
        Vec::new()
    }
    .into_iter()
    .map(|s| s.id)
    .collect::<Vec<_>>();
    Ok((!join || !subscriptions.is_empty()).then_some((membership.map(|m| m.id), subscriptions)))
}
pub fn enqueue_huddle_invitation(
    tx: &mut Tx<'_>,
    room_id: i64,
    recipient_id: i64,
    sender_id: i64,
    payload: PushPayload,
) -> Result<bool> {
    let Some((_, subscription_ids)) =
        huddle_recipient(tx.conn(), room_id, recipient_id, sender_id, false, tx.now())?
    else {
        return Ok(false);
    };
    tx.emit_after_commit(Event::job(&HuddleInvitationDeliveryJob {
        payload,
        subscription_ids,
    }));
    Ok(true)
}
pub fn enqueue_huddle_join(
    tx: &mut Tx<'_>,
    room_id: i64,
    recipient_id: i64,
    sender_id: i64,
    payload: PushPayload,
) -> Result<bool> {
    let now = tx.now();
    let Some((membership_id, subscription_ids)) =
        huddle_recipient(tx.conn(), room_id, recipient_id, sender_id, true, now)?
    else {
        return Ok(false);
    };
    let membership_id = membership_id.expect("join eligibility requires a persisted membership");
    // Strictly older than ten minutes. Exactly the boundary remains throttled in Rails.
    if tx.conn().execute("UPDATE memberships SET last_huddle_join_push_at=? WHERE id=? AND (last_huddle_join_push_at IS NULL OR last_huddle_join_push_at<?)",params![now,membership_id,now.since(-SignedDuration::from_secs(10*60))])?!=1{return Ok(false)};
    tx.emit_after_commit(Event::job(&HuddleJoinDeliveryJob {
        payload,
        subscription_ids,
    }));
    Ok(true)
}
