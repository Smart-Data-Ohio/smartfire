//! Read-side auth and capability policy from Agent, AgentCredential and AgentGrant.
//! The full agent lifecycle and grant/credential administration remain WS11 work.

use jiff::SignedDuration;
use rusqlite::params;

use crate::{Connection, Result, Tx, User};
use crate::sql::{exists, query_one, CachedStatements};
use crate::user::digest_bot_token;

pub const CAPABILITIES: [&str; 7] = ["read_messages", "post_messages", "react", "manage_threads", "external_action", "fizzy", "dm_anyone"];
const LEGACY_CAPABILITIES: [&str; 3] = ["read_messages", "post_messages", "react"];

/// AgentCredential.authenticate plus Agent#active?, record_use! and touch_last_seen!.
/// Both activity stamps use conditional updates (Rails update_all, without callbacks).
pub fn authenticate(tx: &Tx<'_>, secret: &str, ip: &str) -> Result<Option<User>> {
    let secret = campfire_richtext::ruby::strip(secret);
    if campfire_richtext::ruby::is_blank(secret) {
        return Ok(None);
    }
    let identity = query_one(tx.conn(),
        "SELECT agent_credentials.id, agents.id, agents.user_id FROM agent_credentials JOIN agents ON agents.id=agent_credentials.agent_id WHERE token_digest=? AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at>?) AND suspended_at IS NULL LIMIT 1",
        params![digest_bot_token(secret),tx.now()], |r| Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?)))?;
    let Some((credential_id, agent_id, user_id)) = identity else { return Ok(None) };
    let Some(user) = User::find_by_id(tx.conn(),user_id)?.filter(User::is_active) else { return Ok(None) };
    let now = tx.now();
    let cutoff = now.ago(SignedDuration::from_mins(1));
    tx.conn().execute_cached(
        "UPDATE agent_credentials SET last_used_at=?,last_used_ip=?,updated_at=? WHERE id=? AND (last_used_at IS NULL OR last_used_at<=?)",
        params![now,ip,now,credential_id,cutoff])?;
    tx.conn().execute_cached("UPDATE agents SET last_seen_at=? WHERE id=? AND (last_seen_at IS NULL OR last_seen_at<=?)",params![now,agent_id,cutoff])?;
    Ok(Some(user))
}

/// None for a legacy bot without an Agent row. A revoked grant still disables the legacy
/// fallback; workspace-wide and room-scoped active grants both count. No policy cache.
pub fn capability_for_user(conn: &Connection, user_id: i64, capability: &str, room_id: i64) -> Result<Option<bool>> {
    let agent = query_one(conn,"SELECT id,suspended_at FROM agents WHERE user_id=? LIMIT 1",[user_id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,Option<String>>(1)?)))?;
    let Some((agent_id,suspended_at)) = agent else { return Ok(None) };
    if suspended_at.is_some() || !User::find(conn,user_id)?.is_active() || !CAPABILITIES.contains(&capability) {
        return Ok(Some(false));
    }
    if !exists(conn,"SELECT 1 FROM rooms WHERE id=? AND deleted_at IS NULL LIMIT 1",[room_id])? {
        return Ok(Some(false));
    }
    if !exists(conn,"SELECT 1 FROM agent_grants WHERE agent_id=? LIMIT 1",[agent_id])? {
        return Ok(Some(LEGACY_CAPABILITIES.contains(&capability)));
    }
    Ok(Some(exists(conn,"SELECT 1 FROM agent_grants WHERE agent_id=? AND capability=? AND revoked_at IS NULL AND (room_id=? OR room_id IS NULL) LIMIT 1",params![agent_id,capability,room_id])?))
}
