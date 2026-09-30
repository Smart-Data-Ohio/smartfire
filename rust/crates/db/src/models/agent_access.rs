//! Read-side auth and capability policy from Agent, AgentCredential and AgentGrant.
//! The full agent lifecycle and grant/credential administration remain WS11 work.

use jiff::SignedDuration;
use rusqlite::params;

use crate::sql::{CachedStatements, exists, query_one};
use crate::{Connection, Result, Tx, User};

pub const CAPABILITIES: [&str; 7] = [
    "read_messages",
    "post_messages",
    "react",
    "manage_threads",
    "external_action",
    "fizzy",
    "dm_anyone",
];
const LEGACY_CAPABILITIES: [&str; 3] = ["read_messages", "post_messages", "react"];

fn active_agent(conn: &Connection, agent_id: i64) -> Result<Option<(i64, bool)>> {
    let user = query_one(
        conn,
        "SELECT a.user_id FROM agents a JOIN users u ON u.id=a.user_id
         WHERE a.id=? AND a.suspended_at IS NULL AND u.status=0",
        [agent_id],
        |r| r.get::<_, i64>(0),
    )?;
    user.map(|user| {
        Ok((
            user,
            !exists(
                conn,
                "SELECT 1 FROM agent_grants WHERE agent_id=?",
                [agent_id],
            )?,
        ))
    })
    .transpose()
}

/// Agent#can? with a resolved optional Room association. Membership is a caller check.
pub fn capability_for_agent(
    conn: &Connection,
    agent_id: i64,
    capability: &str,
    room_id: Option<i64>,
) -> Result<bool> {
    let Some((_, legacy)) = active_agent(conn, agent_id)? else {
        return Ok(false);
    };
    if !CAPABILITIES.contains(&capability) {
        return Ok(false);
    }
    if let Some(room) = room_id
        && exists(
            conn,
            "SELECT 1 FROM rooms WHERE id=? AND deleted_at IS NOT NULL",
            [room],
        )?
    {
        return Ok(false);
    }
    if legacy {
        return Ok(LEGACY_CAPABILITIES.contains(&capability));
    }
    exists(
        conn,
        "SELECT 1 FROM agent_grants WHERE agent_id=? AND capability=? AND revoked_at IS NULL
         AND (room_id IS NULL OR (? IS NOT NULL AND room_id=?))",
        params![agent_id, capability, room_id, room_id],
    )
}

pub fn has_capability_anywhere(conn: &Connection, agent_id: i64, capability: &str) -> Result<bool> {
    let Some((_, legacy)) = active_agent(conn, agent_id)? else {
        return Ok(false);
    };
    if !CAPABILITIES.contains(&capability) {
        return Ok(false);
    }
    if legacy {
        return Ok(LEGACY_CAPABILITIES.contains(&capability));
    }
    exists(
        conn,
        "SELECT 1 FROM agent_grants WHERE agent_id=? AND capability=? AND revoked_at IS NULL",
        params![agent_id, capability],
    )
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
    let agent_id = credential.agent_id;
    let identity = query_one(
        tx.conn(),
        "SELECT user_id FROM agents WHERE id=? AND suspended_at IS NULL",
        [agent_id],
        |r| r.get::<_, i64>(0),
    )?;
    let Some(user_id) = identity else {
        return Ok(None);
    };
    let Some(user) = User::find_by_id(tx.conn(), user_id)?.filter(User::is_active) else {
        return Ok(None);
    };
    credential.record_use(tx, Some(ip))?;
    let now = tx.now();
    let cutoff = now.ago(SignedDuration::from_mins(1));
    tx.conn().execute_cached(
        "UPDATE agents SET last_seen_at=? WHERE id=? AND (last_seen_at IS NULL OR last_seen_at<=?)",
        params![now, agent_id, cutoff],
    )?;
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
        "SELECT id,suspended_at FROM agents WHERE user_id=? LIMIT 1",
        [user_id],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)),
    )?;
    let Some((agent_id, suspended_at)) = agent else {
        return Ok(None);
    };
    if suspended_at.is_some()
        || !User::find(conn, user_id)?.is_active()
        || !CAPABILITIES.contains(&capability)
    {
        return Ok(Some(false));
    }
    if !exists(
        conn,
        "SELECT 1 FROM rooms WHERE id=? AND deleted_at IS NULL LIMIT 1",
        [room_id],
    )? {
        return Ok(Some(false));
    }
    if !exists(
        conn,
        "SELECT 1 FROM agent_grants WHERE agent_id=? LIMIT 1",
        [agent_id],
    )? {
        return Ok(Some(LEGACY_CAPABILITIES.contains(&capability)));
    }
    Ok(Some(exists(
        conn,
        "SELECT 1 FROM agent_grants WHERE agent_id=? AND capability=? AND revoked_at IS NULL AND (room_id=? OR room_id IS NULL) LIMIT 1",
        params![agent_id, capability, room_id],
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
    if !tx.in_transaction() { return Err(crate::Error::Other("agent signing secret reset requires the writer transaction".into())); }
    let (secret, encrypted) = super::webhook::new_signing_secret(encryption);
    tx.conn().execute_cached(
        "UPDATE agents SET webhook_signing_secret = ?, updated_at = ? WHERE id = ?",
        params![encrypted, tx.now(), agent_id],
    )?;
    Ok(secret)
}
