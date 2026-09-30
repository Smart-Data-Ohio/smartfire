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
pub fn disconnect(
    tx: &mut Tx<'_>,
    user: &User,
    secrets: &Secrets,
    context: &Context,
) -> Result<()> {
    let id = user.id;
    let Some(account) = GoogleAccount::for_user(tx.conn(), id)? else {
        return Ok(());
    };
    let ids = tx
        .conn()
        .prepare("SELECT google_event_id FROM event_calendar_entries WHERE user_id=?")?
        .query_map([id], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let snapshot = account.cleanup_snapshot(&secrets, tx.now().jiff());
    tx.conn()
        .execute("DELETE FROM event_calendar_entries WHERE user_id=?", [id])?;
    tx.conn()
        .execute("DELETE FROM calendar_meeting_caches WHERE user_id=?", [id])?;
    tx.conn().execute("UPDATE events SET meet_link=NULL WHERE organizer_id=? AND meet_link IS NOT NULL AND meet_link!=''",[id])?;
    tx.conn()
        .execute("DELETE FROM calendar_push_channels WHERE user_id=?", [id])?;
    tx.conn()
        .execute("DELETE FROM google_accounts WHERE id=?", [account.id])?;
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "google.account.disconnect".into(),
            target: Some(Target::from(user)),
            changes: Some(json!({"email":account.email})),
            ..Default::default()
        },
        context,
    )?;
    if let Some(blob) = snapshot {
        tx.emit_after_commit(Event::job(&DisconnectCleanupJob((
            ids,
            json!(blob),
            Some(account.id),
        ))));
    }
    Ok(())
}
