//! Read-only Accounts::IntegrationsHealthController rollups. Credentials never enter the DTO.
use campfire_db::{Connection, Result, Timestamp};
use jiff::SignedDuration;
use serde_json::{Value, json};
pub fn snapshot(
    conn: &Connection,
    now: Timestamp,
    env: impl Fn(&str) -> Option<String>,
) -> Result<Value> {
    let present = |key: &str| env(key).is_some_and(|s| !super::github::blank(&s));
    let google_configured = present("GOOGLE_CLIENT_ID") && present("GOOGLE_CLIENT_SECRET");
    let count = |sql: &str| -> Result<i64> { Ok(conn.query_row(sql, [], |r| r.get(0))?) };
    let disconnected=conn.prepare("SELECT email,disconnected_reason FROM google_accounts WHERE disconnected_reason IS NOT NULL ORDER BY updated_at DESC LIMIT 10")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let entries=conn.prepare("SELECT event_id,user_id,last_error FROM event_calendar_entries WHERE last_error IS NOT NULL ORDER BY updated_at DESC LIMIT 10")?.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let expiring=conn.prepare("SELECT user_id,expires_at,last_error FROM calendar_push_channels WHERE expires_at IS NULL OR expires_at<=? ORDER BY expires_at LIMIT 10")?.query_map([now.since(SignedDuration::from_hours(24))],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,Option<Timestamp>>(1)?.map(|t|t.jiff().strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),r.get::<_,Option<String>>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let errors=conn.prepare("SELECT agent_id,event_type,webhook_last_error FROM agent_events WHERE webhook_last_error IS NOT NULL ORDER BY created_at DESC LIMIT 10")?.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(
        json!({"github":super::github::health::snapshot(conn,now,&env)?,
 "google":{"configured":google_configured,"connected":count("SELECT COUNT(*) FROM google_accounts WHERE disconnected_reason IS NULL")?,"disconnected":disconnected,"entry_errors":entries,"push":{"enabled":google_configured&&present("GOOGLE_CALENDAR_WEBHOOK_URL"),"count":count("SELECT COUNT(*) FROM calendar_push_channels")?,"expiring":expiring}},
 // This is the pinned Rails FizzyStatus seam's actual response, even when its client is present.
 "fizzy":{"configured":false,"note":"No Fizzy integration is configured in this workspace."},
 "agent_delivery":{"pending":count("SELECT COUNT(*) FROM agent_events WHERE webhook_status='pending'")?,"failed":conn.query_row("SELECT COUNT(*) FROM agent_events WHERE webhook_status='failed' AND created_at>=?",[now.ago(SignedDuration::from_hours(24))],|r|r.get::<_,i64>(0))?,"recent_errors":errors},
 "email":{"enabled":present("INBOUND_EMAIL_DOMAIN"),"rooms_with_addresses":count("SELECT COUNT(*) FROM rooms WHERE deleted_at IS NULL AND inbound_email_token IS NOT NULL")?}}),
    )
}
