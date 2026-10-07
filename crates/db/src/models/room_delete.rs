//! `app/jobs/room/destroy_job.rb` and Room's asynchronous deletion lifecycle.
use crate::sql::query_all;
use crate::{
    ActivityItem, CachedStatements, ChannelThread, Database, Errors, Event, Job, Message, Result,
    Room, ScheduledMessage, Tx,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};

pub const BATCH_SIZE: usize = 500;
pub const STUCK_GRACE: i64 = 10 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestroyJob {
    pub room_id: i64,
}
impl Job for DestroyJob {
    const CLASS: &'static str = "Room::DestroyJob";
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupJob {
    pub cleanup_id: i64,
}
impl Job for CleanupJob {
    const CLASS: &'static str = "Huddle::CleanupJob";
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RemoteDeleteJob(pub (i64, String));
impl Job for RemoteDeleteJob {
    const CLASS: &'static str = "Calendar::RemoteDeleteJob";
}

/// Capture at boot/call boundary; deterministic callers can supply the same configuration as Rails.
#[derive(Default, Clone)]
pub struct HuddleConfig {
    pub api_secret: Option<String>,
    pub admin_configured: bool,
}
impl HuddleConfig {
    pub fn from_env() -> Self {
        let value = |name| std::env::var(name).ok().filter(|s| !s.trim().is_empty());
        let secret = value("LIVEKIT_API_SECRET");
        let signing = value("LIVEKIT_API_KEY").is_some() && secret.is_some();
        Self {
            api_secret: signing.then_some(secret).flatten(),
            admin_configured: signing && value("LIVEKIT_INTERNAL_URL").is_some(),
        }
    }
}

pub fn begin_destroy(tx: &mut Tx<'_>, room: &Room, config: &HuddleConfig) -> Result<()> {
    // update! validates unchanged direct names too. The icon/type are unchanged: their
    // change-conditional validations do not run. All Room STI classes require a creator.
    validate_room(tx, room)?;
    tx.conn().execute_cached(
        "UPDATE rooms SET deleted_at=?,direct_member_key=NULL,updated_at=? WHERE id=?",
        params![tx.now(), tx.now(), room.id],
    )?;
    crate::ActivityItem::emit_hidden_in_room(tx, room.id, None)?;
    ScheduledMessage::emit_pending_in_room(tx, room.id, None)?;
    tx.conn()
        .execute_cached("DELETE FROM memberships WHERE room_id=?", [room.id])?;
    revoke_huddle_grants(tx, room.id, config)?;
    tx.conn().execute_cached(
        "UPDATE agent_grants SET revoked_at=?,updated_at=? WHERE room_id=? AND revoked_at IS NULL",
        params![tx.now(), tx.now(), room.id],
    )?;
    // Stream#update! validates quality. Rails does not revalidate unchanged optional/FK
    // associations after memberships.delete_all. Ended broadcasts look up an alive Stage,
    // so a deleted room emits none. Creation/normal stream lifecycle belongs to WS13.
    let qualities: Vec<String> = query_all(
        tx.conn(),
        "SELECT quality FROM streams WHERE room_id=? AND ended_at IS NULL",
        [room.id],
        |r| r.get(0),
    )?;
    for quality in qualities {
        if !["720p15", "1080p15", "1080p30"].contains(&quality.as_str()) {
            let mut e = Errors::default();
            e.add("quality", "is not included in the list");
            e.into_result()?;
        }
    }
    tx.conn().execute_cached(
        "UPDATE streams SET ended_at=?,updated_at=? WHERE room_id=? AND ended_at IS NULL",
        params![tx.now(), tx.now(), room.id],
    )?;
    // Unlike Rails' Redis after-commit enqueue, persist in the triggering transaction.
    tx.conn().execute_cached(
        "UPDATE rooms SET destroy_enqueued_at=? WHERE id=?",
        params![tx.now(), room.id],
    )?;
    tx.emit_after_commit(Event::job(&DestroyJob { room_id: room.id }));
    Ok(())
}
fn validate_room(tx: &Tx<'_>, room: &Room) -> Result<()> {
    if room.direct() {
        crate::models::direct_room::validate_name(room.name.as_deref())?;
    }
    let mut errors = Errors::default();
    if crate::User::find_by_id(tx.conn(), room.creator_id)?.is_none() {
        errors.add("creator", "must exist");
    }
    errors.into_result()
}

fn revoke_huddle_grants(tx: &mut Tx<'_>, room_id: i64, config: &HuddleConfig) -> Result<()> {
    crate::models::huddle_grant::HuddleGrant::revoke_for_room(tx, room_id, config)
}

pub fn reenqueue_stuck(tx: &mut Tx<'_>, grace_seconds: i64) -> Result<usize> {
    let cutoff = tx.now().ago(jiff::SignedDuration::from_secs(grace_seconds));
    let ids: Vec<i64> = query_all(
        tx.conn(),
        "SELECT id FROM rooms WHERE deleted_at < ? AND (destroy_enqueued_at IS NULL OR destroy_enqueued_at < ?) ORDER BY id",
        params![cutoff, cutoff],
        |r| r.get(0),
    )?;
    let mut claimed = 0;
    for id in ids {
        if tx.conn().execute_cached("UPDATE rooms SET destroy_enqueued_at=? WHERE id=? AND (destroy_enqueued_at IS NULL OR destroy_enqueued_at < ?)",params![tx.now(),id,cutoff])? == 1 {
            tx.emit_after_commit(Event::job(&DestroyJob{room_id:id})); claimed+=1;
        }
    }
    Ok(claimed)
}

fn ids(tx: &Tx<'_>, table: &str, room_id: i64, limit: usize) -> Result<Vec<i64>> {
    query_all(
        tx.conn(),
        &format!("SELECT id FROM {table} WHERE room_id=? ORDER BY id LIMIT ?"),
        params![room_id, limit as i64],
        |r| r.get(0),
    )
}
pub(crate) fn destroy_grant(tx: &mut Tx<'_>, id: i64) -> Result<()> {
    ActivityItem::destroy_for_source(tx, "HuddleGrant", id)?;
    tx.conn()
        .execute_cached("DELETE FROM huddle_grants WHERE id=?", [id])?;
    Ok(())
}
pub(crate) fn destroy_event(tx: &mut Tx<'_>, id: i64) -> Result<()> {
    let entries: Vec<(i64, String)> = query_all(
        tx.conn(),
        "SELECT user_id,google_event_id FROM event_calendar_entries WHERE event_id=? ORDER BY id",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    for (user, google_id) in entries {
        tx.emit_after_commit(Event::job(&RemoteDeleteJob((user, google_id))));
    }
    for table in [
        "event_attendances",
        "event_calendar_entries",
        "event_references",
    ] {
        tx.conn()
            .execute(&format!("DELETE FROM {table} WHERE event_id=?"), [id])?;
    }
    ActivityItem::destroy_for_source(tx, "Event", id)?;
    tx.conn()
        .execute_cached("DELETE FROM events WHERE id=?", [id])?;
    Ok(())
}
fn destroy_row(tx: &mut Tx<'_>, table: &str, id: i64) -> Result<()> {
    match table {
        "huddle_grants" => destroy_grant(tx, id),
        "scheduled_messages" => {
            if let Some(row) = ScheduledMessage::find_by_id(tx.conn(), id)? {
                row.destroy(tx)?;
            }
            Ok(())
        }
        "messages" => {
            if let Some(row) = Message::find_by_id(tx.conn(), id)? {
                row.destroy_with_conversation(tx)?;
            }
            Ok(())
        }
        "channel_threads" => {
            if let Some(row) = ChannelThread::find_by_id(tx.conn(), id)? {
                row.destroy(tx)?;
            }
            Ok(())
        }
        "events" => destroy_event(tx, id),
        _ => unreachable!(),
    }
}
const CHILDREN: [&str; 5] = [
    "huddle_grants",
    "scheduled_messages",
    "messages",
    "channel_threads",
    "events",
];

/// One transaction per record like Rails destroy!, with the claim committed before any
/// destructive work. A failure retains completed progress; duplicate runs are harmless.
pub async fn perform(db: &Database, room_id: i64) -> Result<()> {
    perform_with_config(db, room_id, HuddleConfig::from_env()).await
}
pub async fn perform_with_config(db: &Database, room_id: i64, config: HuddleConfig) -> Result<()> {
    let claimed=db.write(move |tx| {
        let Some(room)=Room::find_by_id(tx.conn(),room_id)?.filter(Room::deleted) else { return Ok(false) };
        tx.conn().execute_cached("UPDATE rooms SET destroy_enqueued_at=? WHERE id=?",params![tx.now(),room.id])?;
        tx.conn().execute_cached("UPDATE huddle_cleanups SET huddle_grant_id=NULL WHERE huddle_grant_id IN (SELECT id FROM huddle_grants WHERE room_id=?)",[room_id])?;
        Ok(true)
    }).await?;
    if !claimed {
        return Ok(());
    }
    for table in CHILDREN {
        loop {
            let batch = db
                .read(move |conn| {
                    query_all(
                        conn,
                        &format!("SELECT id FROM {table} WHERE room_id=? ORDER BY id LIMIT ?"),
                        params![room_id, BATCH_SIZE as i64],
                        |r| r.get::<_, i64>(0),
                    )
                })
                .await?;
            if batch.is_empty() {
                break;
            }
            for id in batch {
                db.write(move |tx| destroy_row(tx, table, id)).await?;
            }
        }
    }
    db.write(move |tx| {
        if let Some(room) = Room::find_by_id(tx.conn(), room_id)?.filter(Room::deleted) {
            finish_destroy(tx, &room, &config)?;
        }
        Ok(())
    })
    .await
}

/// Room#destroy's declared dependencies, distinct from DestroyJob's preliminary deletes.
/// Huddle grants are only revoked by before_destroy; their cleanup links survive. Scheduled
/// messages use dependent:delete_all, so their dropped-message inbox items survive too.
pub(crate) fn destroy(tx: &mut Tx<'_>, room: &Room) -> Result<()> {
    tx.conn().execute_cached("DELETE FROM memberships WHERE room_id=?", [room.id])?;
    for table in ["messages", "channel_threads", "events"] {
        loop {
            let batch = ids(tx, table, room.id, BATCH_SIZE)?;
            if batch.is_empty() {
                break;
            }
            for id in batch {
                destroy_row(tx, table, id)?;
            }
        }
    }
    finish_destroy(tx, room, &HuddleConfig::from_env())
}
fn finish_destroy(tx: &mut Tx<'_>, room: &Room, config: &HuddleConfig) -> Result<()> {
    // Room's before_destroy callbacks still run even after begin_destroy; cleanup creation
    // is idempotent. Keep the captured remote room name when unlinking grants.
    revoke_huddle_grants(tx, room.id, config)?;
    tx.conn().execute_cached(
        "UPDATE agent_grants SET revoked_at=?,updated_at=? WHERE room_id=? AND revoked_at IS NULL",
        params![tx.now(), tx.now(), room.id],
    )?;
    for table in ["agent_events", "agent_approvals"] {
        tx.conn().execute(
            &format!("UPDATE {table} SET room_id=NULL WHERE room_id=?"),
            [room.id],
        )?;
    }
    tx.conn().execute_cached(
        "UPDATE events SET venue_room_id=NULL WHERE venue_room_id=?",
        [room.id],
    )?;
    // RepositorySubscription#remove_bot_from_room_unless_subscribed is satisfied by the
    // room membership delete_all; notification records have no destruction callbacks.
    tx.conn().execute_cached("DELETE FROM github_notifications WHERE subscription_id IN (SELECT id FROM github_repository_subscriptions WHERE room_id=?)",[room.id])?;
    for table in [
        "github_repository_subscriptions",
        "message_pins",
        "board_tag_assignments",
        "board_sla_rules",
        "board_sla_nudges",
        "board_stale_digests",
        "agent_slash_commands",
        "scheduled_messages",
        "memberships",
    ] {
        tx.conn()
            .execute(&format!("DELETE FROM {table} WHERE room_id=?"), [room.id])?;
    }
    // Stage owns streams dependent:destroy (no destroy callbacks). AgentGrant has no
    // room FK/dependent destroy: Rails retains its revoked grant with the old room id.
    if room.stage() {
        tx.conn()
            .execute_cached("DELETE FROM streams WHERE room_id=?", [room.id])?;
    }
    tx.conn().execute_cached("DELETE FROM github_pull_request_threads WHERE room_id=?", [room.id])?;
    tx.conn()
        .execute_cached("DELETE FROM rooms WHERE id=?", [room.id])?;
    Ok(())
}
