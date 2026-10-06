//! GitHub's read-only entries for Accounts::IntegrationsHealthController.
use campfire_db::{Connection, Result, Timestamp};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub workspace_token: bool,
    pub app_configured: bool,
    pub webhook_secret: bool,
    pub connected: i64,
    pub app_tokens: i64,
    pub disconnected: Vec<(String, String)>,
    pub last_errors: Vec<(String, String)>,
    pub deliveries_24h: i64,
    pub fetch_errors: Vec<(String, String, i64, String)>,
}
pub fn snapshot(
    conn: &Connection,
    now: Timestamp,
    env: impl Fn(&str) -> Option<String>,
) -> Result<Snapshot> {
    let present = |key: &str| env(key).is_some_and(|s| !super::blank(&s));
    let pairs = |column: &str| -> Result<Vec<(String, String)>> {
        Ok(conn.prepare(&format!("SELECT github_login,{column} FROM github_connected_accounts WHERE {column} IS NOT NULL ORDER BY updated_at DESC LIMIT 10"))?.query_map([],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?)
    };
    Ok(Snapshot {workspace_token:present("GITHUB_TOKEN"),app_configured:present("GITHUB_APP_CLIENT_ID")&&present("GITHUB_APP_CLIENT_SECRET"),webhook_secret:present("GITHUB_WEBHOOK_SECRET"),
 connected:conn.query_row("SELECT COUNT(*) FROM github_connected_accounts WHERE disconnected_reason IS NULL",[],|r|r.get(0))?,
 app_tokens:conn.query_row("SELECT COUNT(*) FROM github_connected_accounts WHERE disconnected_reason IS NULL AND token_source='app'",[],|r|r.get(0))?,
 disconnected:pairs("disconnected_reason")?,last_errors:pairs("last_error")?,
 deliveries_24h:conn.query_row("SELECT COUNT(*) FROM github_webhook_deliveries WHERE created_at>=?",[now.ago(jiff::SignedDuration::from_hours(24))],|r|r.get(0))?,
 fetch_errors:conn.prepare("SELECT owner,repo,number,fetch_error FROM github_pull_requests WHERE fetch_error IS NOT NULL ORDER BY updated_at DESC LIMIT 10")?.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?.collect::<rusqlite::Result<_>>()? })
}
