//! Read-side auth and capability policy from Agent, AgentCredential and AgentGrant.
//! The full agent lifecycle and grant/credential administration remain WS11 work.

use jiff::SignedDuration;
use rusqlite::params;

use crate::sql::{CachedStatements, exists, query_all, query_one};
use crate::{Connection, Result, Timestamp, Tx, User};

pub const CAPABILITIES: [&str; 7] = [
    "read_messages",
    "post_messages",
    "react",
    "manage_threads",
    "external_action",
    "fizzy",
    "dm_anyone",
];
pub const PERSONAL_CAPABILITIES: [&str; 4] =
    ["read_messages", "post_messages", "react", "manage_threads"];

/// SQL expressions here and in capability_sql are supplied by code, never requests.
pub fn agent_live(agent: &str) -> String {
    format!(
        "{agent}.suspended_at IS NULL AND EXISTS (SELECT 1 FROM users live_bot WHERE live_bot.id={agent}.user_id AND live_bot.status=0) AND ({agent}.kind!='personal' OR EXISTS (SELECT 1 FROM users live_owner WHERE live_owner.id={agent}.owner_id AND live_owner.status=0))"
    )
}

pub fn legacy_sql(agent: &str) -> String {
    format!(
        "{agent}.kind!='personal' AND NOT EXISTS (SELECT 1 FROM agent_grants legacy_grant WHERE legacy_grant.agent_id={agent}.id)"
    )
}

/// The single capability policy, shared by single, batch and event readers.
/// Workspace membership remains a caller check; personal membership is a boundary here.
pub fn capability_sql(agent: &str, room: &str, cap: &str) -> String {
    let live = agent_live("policy_agent");
    let legacy = legacy_sql("policy_agent");
    format!("EXISTS (SELECT 1 FROM agents policy_agent WHERE policy_agent.id=({agent}) AND ({live}) AND ({cap}) IN ('read_messages','post_messages','react','manage_threads','external_action','fizzy','dm_anyone') AND (
        (policy_agent.kind!='personal' AND NOT EXISTS (SELECT 1 FROM rooms policy_deleted WHERE policy_deleted.id=({room}) AND policy_deleted.deleted_at IS NOT NULL) AND (
            (({cap}) IN ('read_messages','post_messages','react') AND ({legacy})) OR EXISTS (SELECT 1 FROM agent_grants policy_grant WHERE policy_grant.agent_id=policy_agent.id AND policy_grant.capability=({cap}) AND policy_grant.revoked_at IS NULL AND (policy_grant.room_id IS NULL OR policy_grant.room_id=({room})))) )
        OR (policy_agent.kind='personal' AND ({cap}) IN ('read_messages','post_messages','react','manage_threads') AND EXISTS (
            SELECT 1 FROM rooms policy_room JOIN memberships policy_member ON policy_member.room_id=policy_room.id AND policy_member.user_id=policy_agent.user_id
            WHERE policy_room.id=({room}) AND policy_room.deleted_at IS NULL AND (
                (policy_room.type!='Rooms::Direct' AND EXISTS (SELECT 1 FROM memberships policy_owner WHERE policy_owner.room_id=policy_room.id AND policy_owner.user_id=policy_agent.owner_id) AND EXISTS (SELECT 1 FROM agent_grants policy_grant WHERE policy_grant.agent_id=policy_agent.id AND policy_grant.capability=({cap}) AND policy_grant.room_id=policy_room.id AND policy_grant.revoked_at IS NULL))
                OR (policy_room.type='Rooms::Direct' AND ({cap}) IN ('read_messages','post_messages','react') AND policy_agent.owner_id!=policy_agent.user_id AND (SELECT COUNT(*) FROM memberships dm_member WHERE dm_member.room_id=policy_room.id)=2 AND EXISTS (SELECT 1 FROM memberships dm_owner WHERE dm_owner.room_id=policy_room.id AND dm_owner.user_id=policy_agent.owner_id))
            )))
    ))")
}

pub fn is_personal_user(conn: &Connection, user_id: i64) -> Result<bool> {
    exists(
        conn,
        "SELECT 1 FROM agents WHERE user_id=? AND kind='personal'",
        [user_id],
    )
}

pub fn check_direct_members(conn: &Connection, member_ids: &[i64]) -> Result<()> {
    let members: std::collections::HashSet<_> = member_ids.iter().copied().collect();
    let personal = query_all(
        conn,
        "SELECT user_id,owner_id FROM agents WHERE kind='personal' AND user_id IN (SELECT value FROM json_each(?))",
        [serde_json::json!(member_ids).to_string()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
    )?;
    for (bot, owner) in personal {
        if members.len() != 2 || owner.is_none_or(|owner| owner == bot || !members.contains(&owner))
        {
            return Err(crate::Error::Other("PersonalAgentDirect".into()));
        }
    }
    Ok(())
}

pub fn capability_for_agent(
    conn: &Connection,
    agent_id: i64,
    capability: &str,
    room_id: Option<i64>,
) -> Result<bool> {
    Ok(conn.query_row(
        &format!("SELECT {}", capability_sql("?1", "?2", "?3")),
        params![agent_id, room_id, capability],
        |row| row.get(0),
    )?)
}

pub fn capabilities_for_agents(
    conn: &Connection,
    capability: &str,
    requests: &[(i64, Option<i64>)],
) -> Result<std::collections::HashMap<(i64, Option<i64>), bool>> {
    Ok(query_all(
        conn, &format!("SELECT json_extract(request.value,'$[0]'),json_extract(request.value,'$[1]'),{} FROM json_each(?1) request", capability_sql("json_extract(request.value,'$[0]')", "json_extract(request.value,'$[1]')", "?2")),
        params![serde_json::json!(requests).to_string(),capability],
        |row| Ok(((row.get(0)?, row.get(1)?), row.get(2)?)))?
    .into_iter()
    .collect())
}

pub fn capabilities_for_user_in_room(
    conn: &Connection,
    user_id: i64,
    room_id: i64,
    capabilities: &[&str],
) -> Result<std::collections::HashMap<String, Option<bool>>> {
    Ok(query_all(
        conn, &format!("SELECT requested.value,CASE WHEN a.id IS NULL THEN NULL ELSE EXISTS (SELECT 1 FROM rooms WHERE id=?2 AND deleted_at IS NULL) AND ({}) END FROM json_each(?1) requested LEFT JOIN agents a ON a.user_id=?3",capability_sql("a.id","?2","requested.value")),
        params![serde_json::json!(capabilities).to_string(),room_id,user_id],
        |row| Ok((row.get(0)?, row.get(1)?)))?
    .into_iter()
    .collect())
}

pub fn capabilities_for_users_in_rooms(
    conn: &Connection,
    users: &[i64],
    rooms: &[i64],
    capability: &str,
) -> Result<std::collections::HashSet<(i64, i64)>> {
    Ok(query_all(
        conn,
        &format!("SELECT a.user_id,r.id FROM agents a CROSS JOIN rooms r WHERE a.user_id IN (SELECT value FROM json_each(?1)) AND r.id IN (SELECT value FROM json_each(?2)) AND r.deleted_at IS NULL AND ({})",capability_sql("a.id","r.id","?3")
        ),params![serde_json::json!(users).to_string(),serde_json::json!(rooms).to_string(),capability],
        |row| Ok((row.get(0)?,row.get(1)?)))?.into_iter().collect())
}

pub fn has_capability_anywhere(conn: &Connection, agent_id: i64, capability: &str) -> Result<bool> {
    Ok(conn.query_row(
        &format!(
            "SELECT ({}) OR EXISTS (SELECT 1 FROM rooms anywhere_room WHERE ({}))",
            capability_sql("?1", "NULL", "?2"),
            capability_sql("?1", "anywhere_room.id", "?2")
        ),
        params![agent_id, capability],
        |row| row.get(0),
    )?)
}

/// AgentCredential.authenticate plus Agent#active?, record_use! and touch_last_seen!.
/// Both activity stamps use conditional updates (Rails update_all, without callbacks).
#[derive(Debug, Clone)]
pub struct AuthenticatedAgent {
    pub user: User,
    pub agent_id: i64,
    pub credential_id: i64,
}

pub fn authenticate(tx: &Tx<'_>, secret: &str, ip: &str) -> Result<Option<User>> {
    Ok(authenticate_identity(tx, secret, ip)?.map(|identity| identity.user))
}
/// Request identity for the REST/MCP callers; the credential policy also works independently
/// of an Agent's activity, as AgentCredential.authenticate does in Rails.
pub fn authenticate_identity(
    tx: &Tx<'_>,
    secret: &str,
    ip: &str,
) -> Result<Option<AuthenticatedAgent>> {
    let Some(mut credential) =
        super::agent_credential::AgentCredential::authenticate(tx.conn(), secret, tx.now())?
    else {
        return Ok(None);
    };
    let Some(identity) = identity_for_credential(tx.conn(), &credential)? else {
        return Ok(None);
    };
    let agent_id = identity.agent_id;
    credential.record_use(tx, Some(ip))?;
    let now = tx.now();
    let cutoff = now.ago(SignedDuration::from_mins(1));
    tx.conn().execute_cached(
        "UPDATE agents SET last_seen_at=? WHERE id=? AND (last_seen_at IS NULL OR last_seen_at<=?)",
        params![now, agent_id, cutoff],
    )?;
    Ok(Some(identity))
}

/// Recheck a credential's current policy inside a later write without recording another
/// authentication. The request's original activity timestamps and IP belong to its before-action.
pub fn verify_identity(
    conn: &Connection,
    secret: &str,
    now: Timestamp,
) -> Result<Option<AuthenticatedAgent>> {
    let Some(credential) =
        super::agent_credential::AgentCredential::authenticate(conn, secret, now)?
    else {
        return Ok(None);
    };
    identity_for_credential(conn, &credential)
}
fn identity_for_credential(
    conn: &Connection,
    credential: &super::agent_credential::AgentCredential,
) -> Result<Option<AuthenticatedAgent>> {
    let agent_id = credential.agent_id;
    let identity = query_one(
        conn,
        &format!(
            "SELECT a.user_id FROM agents a WHERE a.id=? AND ({})",
            agent_live("a")
        ),
        [agent_id],
        |r| r.get::<_, i64>(0),
    )?;
    let Some(user_id) = identity else {
        return Ok(None);
    };
    let Some(user) = User::find_by_id(conn, user_id)?.filter(User::is_active) else {
        return Ok(None);
    };
    Ok(Some(AuthenticatedAgent {
        user,
        agent_id,
        credential_id: credential.id,
    }))
}

/// None for a legacy bot without an Agent row. A revoked grant still disables the legacy
/// fallback; workspace-wide and room-scoped active grants both count. No policy cache.
pub fn capability_for_user(
    conn: &Connection,
    user_id: i64,
    capability: &str,
    room_id: i64,
) -> Result<Option<bool>> {
    let agent = query_one(
        conn,
        "SELECT id FROM agents WHERE user_id=? LIMIT 1",
        [user_id],
        |r| r.get::<_, i64>(0),
    )?;
    let Some(agent) = agent else {
        return Ok(None);
    };
    if !exists(
        conn,
        "SELECT 1 FROM rooms WHERE id=? AND deleted_at IS NULL",
        [room_id],
    )? {
        return Ok(Some(false));
    }
    Ok(Some(capability_for_agent(
        conn,
        agent,
        capability,
        Some(room_id),
    )?))
}

/// Agent#ensure_webhook_signing_secret!, shared by every agent payload type.
pub fn ensure_webhook_signing_secret(
    tx: &Tx<'_>,
    encryption: &rails_compat::ar_encryption::ArEncryption,
    agent_id: i64,
) -> Result<String> {
    let encrypted: Option<String> = tx.conn().query_row_cached(
        "SELECT webhook_signing_secret FROM agents WHERE id = ?",
        [agent_id],
        |row| row.get(0),
    )?;
    if let Some(encrypted) = encrypted {
        let secret = encryption
            .decrypt(&encrypted)
            .map_err(|error| crate::Error::Other(error.to_string()))?;
        if !campfire_richtext::ruby::is_blank(&secret) {
            return Ok(secret);
        }
    }
    reset_webhook_signing_secret(tx, encryption, agent_id)
}

/// `reset_webhook_signing_secret!`: the writer's BEGIN IMMEDIATE is the
/// SQLite row lock. Refuse an unprotected after-commit write.
pub fn reset_webhook_signing_secret(
    tx: &Tx<'_>,
    encryption: &rails_compat::ar_encryption::ArEncryption,
    agent_id: i64,
) -> Result<String> {
    if !tx.in_transaction() {
        return Err(crate::Error::Other(
            "agent signing secret reset requires the writer transaction".into(),
        ));
    }
    let (secret, encrypted) = super::webhook::new_signing_secret(encryption);
    let previous: Timestamp = tx.conn().query_row(
        "SELECT updated_at FROM agents WHERE id=?",
        [agent_id],
        |row| row.get(0),
    )?;
    let revision = tx.revision_after(previous);
    tx.conn().execute_cached(
        "UPDATE agents SET webhook_signing_secret = ?, updated_at = ? WHERE id = ?",
        params![encrypted, revision, agent_id],
    )?;
    Ok(secret)
}
