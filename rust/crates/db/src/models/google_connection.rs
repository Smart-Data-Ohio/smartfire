//! Connection state changes and durable producers (`Google::ConnectionsController`).
use super::{
    audit_log::{AuditLog, Context, NewAuditLog, Target},
    google_account::{ConnectionGrant, GoogleAccount},
    google_calendar::{
        DisconnectCleanupJob, MeetLinkJob, MeetingRefreshJob, SyncEntryJob, WatchChannelJob,
    },
};
use crate::{Event, Result, Tx, User};
use rails_compat::{Secrets, ar_encryption::ArEncryption};
use serde_json::json;
pub fn finish(
    tx: &mut Tx<'_>,
    enc: &ArEncryption,
    grant: ConnectionGrant,
    user: &User,
    context: &Context,
) -> Result<bool> {
    let account = GoogleAccount::save_connection(tx, enc, grant)?;
    if !account.calendar() {
        return Ok(false);
    }
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "google.account.connect".into(),
            target: Some(Target::from(user)),
            changes: Some(json!({"email":account.email})),
            ..Default::default()
        },
        context,
    )?;
    let events=tx.conn().prepare("SELECT ea.event_id FROM event_attendances ea JOIN events e ON e.id=ea.event_id WHERE ea.user_id=? AND ea.response IN ('going','maybe') AND e.cancelled_at IS NULL AND COALESCE(e.ends_at,e.starts_at)>=?")?.query_map(rusqlite::params![user.id,tx.now()],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for event_id in events {
        tx.emit_after_commit(Event::job(&SyncEntryJob {
            event_id,
            user_id: user.id,
        }));
    }
    let events=tx.conn().prepare("SELECT id FROM events WHERE organizer_id=? AND meet_link_requested=1 AND (meet_link IS NULL OR meet_link='') ORDER BY id")?.query_map([user.id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for event_id in events {
        tx.emit_after_commit(Event::job(&MeetLinkJob { event_id }));
    }
    tx.emit_after_commit(Event::job(&WatchChannelJob((user.id,))));
    let enabled = tx.conn().query_row(
        "SELECT meeting_status_enabled OR ooo_calendar_enabled FROM users WHERE id=?",
        [user.id],
        |r| r.get::<_, bool>(0),
    )?;
    if enabled {
        tx.emit_after_commit(Event::job(&MeetingRefreshJob { user_id: user.id }));
    }
    Ok(true)
}
pub struct DisconnectPlan {
    account_id: i64,
    email: String,
}

/// Rails commits these local removals and broadcasts before the best-effort remote stop.
pub fn prepare_disconnect(
    tx: &mut Tx<'_>,
    user: &User,
    secrets: &Secrets,
) -> Result<Option<DisconnectPlan>> {
    let id = user.id;
    let Some(account) = GoogleAccount::for_user(tx.conn(), id)? else {
        return Ok(None);
    };
    let ids = tx
        .conn()
        .prepare("SELECT google_event_id FROM event_calendar_entries WHERE user_id=?")?
        .query_map([id], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let plan = DisconnectPlan {
        account_id: account.id,
        email: account.email.clone(),
    };
    // Durable cleanup must commit with the entries that contain its remote IDs,
    // before the controller's best-effort remote stop and account removal.
    if let Some(blob) = account.cleanup_snapshot(secrets, tx.now().jiff()) {
        tx.emit_after_commit(Event::job(&DisconnectCleanupJob((
            ids,
            json!(blob),
            Some(account.id),
        ))));
    }
    tx.conn()
        .execute("DELETE FROM event_calendar_entries WHERE user_id=?", [id])?;
    let removed = tx
        .conn()
        .execute("DELETE FROM calendar_meeting_caches WHERE user_id=?", [id])?;
    if removed != 0 {
        let status = crate::UserStatusSettings::find(tx.conn(), id)?;
        status.claim_ooo_broadcast(tx, status.out_of_office(tx.now()), tx.now())?;
        // claim_ooo_broadcast deliberately keeps the loaded attributes, as update_all does.
        status.announce_ooo(tx)?;
    }
    tx.conn().execute("UPDATE events SET meet_link=NULL WHERE organizer_id=? AND meet_link IS NOT NULL AND meet_link!=''", [id])?;
    Ok(Some(plan))
}

pub fn finish_disconnect(
    tx: &mut Tx<'_>,
    user: &User,
    plan: DisconnectPlan,
    channel: Option<i64>,
    context: &Context,
) -> Result<()> {
    if let Some(id) = channel {
        tx.conn()
            .execute("DELETE FROM calendar_push_channels WHERE id=?", [id])?;
    }
    tx.conn()
        .execute("DELETE FROM google_accounts WHERE id=?", [plan.account_id])?;
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "google.account.disconnect".into(),
            target: Some(Target::from(user)),
            changes: Some(json!({"email":plan.email})),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

/// Transactional caller seam; HTTP controllers use the two phases around the remote stop.
pub fn disconnect(
    tx: &mut Tx<'_>,
    user: &User,
    secrets: &Secrets,
    context: &Context,
) -> Result<()> {
    use rusqlite::OptionalExtension;
    if let Some(plan) = prepare_disconnect(tx, user, secrets)? {
        let channel = tx
            .conn()
            .query_row(
                "SELECT id FROM calendar_push_channels WHERE user_id=?",
                [user.id],
                |r| r.get(0),
            )
            .optional()?;
        finish_disconnect(tx, user, plan, channel, context)?;
    }
    Ok(())
}

/// Google state in User#deactivate. Durable cleanup and entry syncs share the user write.
/// The authorized application caller stops the remote channel before taking the writer.
pub fn deactivate(tx: &mut Tx<'_>, user_id: i64, secrets: &Secrets) -> Result<()> {
    let ids = tx
        .conn()
        .prepare("SELECT event_id FROM event_calendar_entries WHERE user_id=?")?
        .query_map([user_id], |r| r.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut account = GoogleAccount::for_user(tx.conn(), user_id)?;
    if let Some(account) = &mut account
        && account.usable(tx, &ArEncryption::new(secrets))?
    {
        let snapshot = account.cleanup_snapshot(secrets, tx.now().jiff());
        tx.emit_after_commit(Event::job(&DisconnectCleanupJob((
            vec![],
            json!(snapshot),
            Some(account.id),
        ))));
    }
    tx.conn().execute(
        "DELETE FROM calendar_push_channels WHERE user_id=?",
        [user_id],
    )?;
    tx.conn().execute(
        "DELETE FROM calendar_meeting_caches WHERE user_id=?",
        [user_id],
    )?;
    if let Some(account) = &mut account {
        account.mark_disconnected(tx, "Account deactivated")?;
    }
    for event_id in ids {
        tx.emit_after_commit(Event::job(&SyncEntryJob { event_id, user_id }));
    }
    Ok(())
}
