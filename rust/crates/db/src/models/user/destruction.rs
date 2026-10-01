//! Slack dependencies supplied to User's shared hard-destruction callbacks.
use super::{User, removal::DependencyPhase};
use crate::sql::query_all;
use crate::{Event, Result, Tx};
use rusqlite::params;

impl User {
    pub fn destroy_for_slack_undo(&self, tx: &mut Tx<'_>) -> Result<()> {
        self.destroy_with_dependencies(tx, |tx, user, phase| {
            match phase {
                DependencyPhase::BeforeDestroy => {
                    tx.conn().execute("UPDATE huddle_grants SET revoked_at=?1,updated_at=?1 WHERE user_id=?2 AND revoked_at IS NULL", params![tx.now(),user.id])?;
                }
                DependencyPhase::Connections => user.destroy_slack_dependencies(tx)?,
            }
            Ok(())
        })
    }

    fn destroy_slack_dependencies(&self, tx: &mut Tx<'_>) -> Result<()> {
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
        Ok(())
    }
}
fn delete(tx: &Tx<'_>, table: &str, key: &str, user: i64) -> Result<()> {
    tx.conn().execute(&format!("DELETE FROM {table} WHERE {key}=?"), [user])?;
    Ok(())
}
