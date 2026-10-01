//! User's dependent destruction, used by Slack undo after its claim/authorship checks.
use super::User;
use crate::sql::query_all;
use crate::{
    Attachment, Boost, ChannelThread, Event, Message, MessagePin, Result, RoomCategory, Session, Tx,
};
use rusqlite::params;

impl User {
    /// `app/models/user.rb`: destroy dependents in declaration order. Callers rescuing a
    /// failed destroy must use a savepoint, as Rails wraps `destroy!` in a transaction.
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        // The huddle revocation's stream/presence payloads are WS13's model seam, also
        // pending in User's status callback. Persist the revocation before dependents.
        tx.conn().execute("UPDATE huddle_grants SET revoked_at=?1,updated_at=?1 WHERE user_id=?2 AND revoked_at IS NULL", params![tx.now(),self.id])?;
        crate::AgentGrant::revoke_for_user(tx, self.id)?;
        delete(tx, "memberships", "user_id", self.id)?;
        for message in Message::by_creator(tx.conn(), self.id)? {
            message.destroy(tx)?;
        }
        for id in ids(tx, "channel_threads", "creator_id", self.id)? {
            ChannelThread::find(tx.conn(), id)?.destroy(tx)?;
        }
        delete(tx, "thread_memberships", "user_id", self.id)?;
        delete(tx, "push_subscriptions", "user_id", self.id)?;
        for table in [
            "google_accounts",
            "calendar_meeting_caches",
            "google_identities",
            "github_connected_accounts",
            "fizzy_connected_accounts",
        ] {
            delete(tx, table, "user_id", self.id)?;
        }
        // SlackConnection dependent:nullify; SlackImport owns only its records and issues.
        tx.conn().execute("UPDATE slack_imports SET slack_connection_id=NULL WHERE slack_connection_id IN (SELECT id FROM slack_connections WHERE user_id=?)", [self.id])?;
        delete(tx, "slack_connections", "user_id", self.id)?;
        for table in ["slack_import_records", "slack_import_issues"] {
            tx.conn().execute(&format!("DELETE FROM {table} WHERE slack_import_id IN (SELECT id FROM slack_imports WHERE user_id=?)"), [self.id])?;
        }
        delete(tx, "slack_imports", "user_id", self.id)?;
        let entries: Vec<String> = query_all(
            tx.conn(),
            "SELECT google_event_id FROM event_calendar_entries WHERE user_id=? ORDER BY id",
            [self.id],
            |r| r.get(0),
        )?;
        for google_id in entries {
            tx.emit_after_commit(Event::job(&crate::models::room_delete::RemoteDeleteJob((
                self.id, google_id,
            ))));
        }
        delete(tx, "event_calendar_entries", "user_id", self.id)?;
        for id in ids(tx, "boosts", "booster_id", self.id)? {
            Boost::find(tx.conn(), id)?.destroy(tx)?;
        }
        delete(tx, "poll_votes", "user_id", self.id)?;
        for id in ids(tx, "message_pins", "pinner_id", self.id)? {
            MessagePin::find(tx.conn(), id)?.unpin(tx)?;
        }
        for table in ["saved_items", "scheduled_messages", "searches"] {
            delete(tx, table, "user_id", self.id)?;
        }
        for category in RoomCategory::ordered_for_user(tx.conn(), self.id)? {
            category.destroy(tx)?;
        }
        for id in ids(tx, "sessions", "user_id", self.id)? {
            Session::find(tx.conn(), id)?.destroy(tx)?;
        }
        for table in ["user_devices", "workspace_presence_leases", "bans"] {
            delete(tx, table, "user_id", self.id)?;
        }
        if let Some(avatar) = Attachment::find_for(tx.conn(), "User", self.id, "avatar")? {
            avatar.delete(tx)?;
            tx.emit_after_commit(Event::PurgeBlob {
                blob_id: avatar.blob_id,
            });
        }
        tx.conn()
            .execute("DELETE FROM users WHERE id=?", [self.id])?;
        Ok(())
    }
}
fn ids(tx: &Tx<'_>, table: &str, key: &str, user: i64) -> Result<Vec<i64>> {
    query_all(
        tx.conn(),
        &format!("SELECT id FROM {table} WHERE {key}=? ORDER BY id"),
        [user],
        |r| r.get(0),
    )
}
fn delete(tx: &Tx<'_>, table: &str, key: &str, user: i64) -> Result<()> {
    tx.conn()
        .execute(&format!("DELETE FROM {table} WHERE {key}=?"), [user])?;
    Ok(())
}
