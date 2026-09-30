//! Safe facts for the human directory. No credential, grant or signing secret is read.
use super::{attachments, resolve_avatar_icon, user_summary};
use campfire_db::{Connection, Result};
use campfire_views::agents::DirectoryAgent;

pub fn directory(
    conn: &Connection,
    secrets: &rails_compat::Secrets,
) -> Result<Vec<DirectoryAgent>> {
    campfire_db::models::agent_profile::for_directory(conn)?
        .into_iter()
        .map(|record| {
            let user = &record.user;
            let icon_name: Option<String> =
                conn.query_row("SELECT icon_name FROM users WHERE id=?", [user.id], |row| {
                    row.get(0)
                })?;
            let icon = if user.is_bot()
                && attachments::attached_blob(conn, "User", user.id, "avatar")?.is_none()
            {
                icon_name
                    .as_deref()
                    .and_then(|name| resolve_avatar_icon(conn, name))
            } else {
                None
            };
            let kind_description = match record.owner {
                None => "no owner recorded".into(),
                Some(owner) if record.kind == "personal" => {
                    format!("Personal agent of {}", owner.name)
                }
                Some(owner) => format!("Workspace agent, managed by {}", owner.name),
            };
            Ok(DirectoryAgent {
                id: record.id,
                user: user_summary(secrets, user),
                icon,
                kind_description,
                status: record.status,
                status_note: record.status_note,
                suspended: record.suspended,
                created_at: record.created_at.jiff(),
                status_changed_at: record.status_changed_at.map(|time| time.jiff()),
                last_seen_at: record.last_seen_at.map(|time| time.jiff()),
            })
        })
        .collect()
}
