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
const LEGACY_CAPABILITIES: [&str; 3] = ["read_messages", "post_messages", "react"];

/// Request-local batch of Agent#can? facts, with the same policy as capability_for_agent.
/// Membership remains the caller's check. A revoked grant disables the legacy fallback.
pub fn capabilities_for_agents(
    conn: &Connection,
    capability: &str,
    requests: &[(i64, Option<i64>)],
) -> Result<std::collections::HashMap<(i64, Option<i64>), bool>> {
    if requests.is_empty() {
        return Ok(Default::default());
    }
    if !CAPABILITIES.contains(&capability) {
        return Ok(requests.iter().map(|&key| (key, false)).collect());
    }
    Ok(query_all(
        conn,
        "SELECT json_extract(request.value,'$[0]'), json_extract(request.value,'$[1]'),
         a.id IS NOT NULL AND a.suspended_at IS NULL AND u.id IS NOT NULL AND u.status=0
         AND (r.id IS NULL OR r.deleted_at IS NULL)
         AND ((? AND NOT EXISTS (SELECT 1 FROM agent_grants g WHERE g.agent_id=a.id))
           OR EXISTS (SELECT 1 FROM agent_grants g WHERE g.agent_id=a.id
             AND g.capability=? AND g.revoked_at IS NULL
             AND (g.room_id IS NULL OR g.room_id=json_extract(request.value,'$[1]'))))
         FROM json_each(?) request
         LEFT JOIN agents a ON a.id=json_extract(request.value,'$[0]')
         LEFT JOIN users u ON u.id=a.user_id
         LEFT JOIN rooms r ON r.id=json_extract(request.value,'$[1]')",
        params![
            LEGACY_CAPABILITIES.contains(&capability),
            capability,
            serde_json::json!(requests).to_string()
        ],
        |row| Ok(((row.get(0)?, row.get(1)?), row.get(2)?)),
    )?
    .into_iter()
    .collect())
}

/// Request-local capability facts for a current user. None preserves the human/
/// missing-agent behavior of capability_for_user; membership remains separate.
pub fn capabilities_for_user_in_room(
    conn: &Connection,
    user_id: i64,
    room_id: i64,
    capabilities: &[&str],
) -> Result<std::collections::HashMap<String, Option<bool>>> {
    Ok(query_all(
        conn,
        "SELECT requested.value, CASE WHEN a.id IS NULL THEN NULL ELSE
         a.suspended_at IS NULL AND u.status=0 AND r.id IS NOT NULL AND r.deleted_at IS NULL
         AND ((requested.value IN ('read_messages','post_messages','react')
           AND NOT EXISTS (SELECT 1 FROM agent_grants g WHERE g.agent_id=a.id))
           OR EXISTS (SELECT 1 FROM agent_grants g WHERE g.agent_id=a.id
             AND g.capability=requested.value AND g.revoked_at IS NULL
             AND (g.room_id IS NULL OR g.room_id=?))) END
         FROM json_each(?) requested LEFT JOIN agents a ON a.user_id=?
         LEFT JOIN users u ON u.id=a.user_id LEFT JOIN rooms r ON r.id=?",
        params![
            room_id,
            serde_json::json!(capabilities).to_string(),
            user_id,
            room_id
        ],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<bool>>(1)?)),
    )?
    .into_iter()
    .map(|(cap, allowed)| {
        let allowed = if CAPABILITIES.contains(&cap.as_str()) {
            allowed
        } else {
            allowed.map(|_| false)
        };
        (cap, allowed)
    })
    .collect())
}

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

/// Bulk Agent#can? decisions for real rooms. Membership remains a caller check.
/// No state escapes the current reader connection.
pub fn capabilities_for_users_in_rooms(
    conn: &Connection,
    users: &[i64],
    rooms: &[i64],
    capability: &str,
) -> Result<std::collections::HashSet<(i64, i64)>> {
    use crate::sql::placeholders;
    use crate::sql::query_all;
    let mut allowed = std::collections::HashSet::new();
    if users.is_empty() || rooms.is_empty() || !CAPABILITIES.contains(&capability) {
        return Ok(allowed);
    }
    let agents = query_all(
        conn,
        &format!(
            "SELECT a.id,a.user_id FROM agents a JOIN users u ON u.id=a.user_id WHERE a.user_id IN ({}) AND a.suspended_at IS NULL AND u.status=0",
            placeholders(users.len())
        ),
        rusqlite::params_from_iter(users),
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
    )?;
    let ids = agents.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    if ids.is_empty() {
        return Ok(allowed);
    }
    let grants = query_all(
        conn,
        &format!(
            "SELECT agent_id,capability,room_id,revoked_at IS NULL FROM agent_grants WHERE agent_id IN ({})",
            placeholders(ids.len())
        ),
        rusqlite::params_from_iter(ids),
        |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, bool>(3)?,
            ))
        },
    )?;
    let rooms = query_all(
        conn,
        &format!(
            "SELECT id FROM rooms WHERE id IN ({}) AND deleted_at IS NULL",
            placeholders(rooms.len())
        ),
        rusqlite::params_from_iter(rooms),
        |r| r.get::<_, i64>(0),
    )?;
    for (agent, user) in agents {
        let legacy = !grants.iter().any(|(id, _, _, _)| *id == agent);
        for &room in &rooms {
            if (legacy && LEGACY_CAPABILITIES.contains(&capability))
                || grants.iter().any(|(id, cap, scope, active)| {
                    *id == agent
                        && cap == capability
                        && *active
                        && (scope.is_none() || *scope == Some(room))
                })
            {
                allowed.insert((user, room));
            }
        }
    }
    Ok(allowed)
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
        "SELECT user_id FROM agents WHERE id=? AND suspended_at IS NULL",
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
    if !tx.in_transaction() {
        return Err(crate::Error::Other(
            "agent signing secret reset requires the writer transaction".into(),
        ));
    }
    let (secret, encrypted) = super::webhook::new_signing_secret(encryption);
    tx.conn().execute_cached(
        "UPDATE agents SET webhook_signing_secret = ?, updated_at = ? WHERE id = ?",
        params![encrypted, tx.now(), agent_id],
    )?;
    Ok(secret)
}
