//! Personal-agent creation and access replacement, using the shared S1 policy.
use crate::{Agent, AgentKind, NewAgent, NewUser, Result, Tx, User};

pub fn create(tx: &mut Tx<'_>, mut user: NewUser, owner_id: i64) -> Result<Agent> {
    user.role = crate::Role::Bot;
    user.bot_token_digest = None;
    let bot = User::create_with_open_room_grant(tx, user, false)?;
    Agent::create(
        tx,
        NewAgent {
            user_id: bot.id,
            owner_id: Some(owner_id),
            kind: AgentKind::Personal,
            daily_message_cap: Some(200),
            ..Default::default()
        },
    )
}

pub fn owned(conn: &crate::Connection, owner_id: i64, id: i64) -> Result<Agent> {
    let agent = Agent::find(conn, id)?
        .filter(|a| a.kind == AgentKind::Personal && a.owner_id == Some(owner_id))
        .ok_or(crate::Error::RecordNotFound("Agent"))?;
    if !User::find(conn, agent.user_id)?.is_active() {
        return Err(crate::Error::RecordNotFound("Agent"));
    }
    Ok(agent)
}

pub fn for_owner(conn: &crate::Connection, owner_id: i64) -> Result<Vec<Agent>> {
    let ids = crate::sql::query_all(
        conn,
        "SELECT a.id FROM agents a JOIN users u ON u.id=a.user_id WHERE a.kind='personal' AND a.owner_id=? AND u.status=0 ORDER BY a.id DESC",
        [owner_id],
        |r| r.get::<_, i64>(0),
    )?;
    ids.into_iter()
        .map(|id| owned(conn, owner_id, id))
        .collect()
}

/// A room a member can grant their personal agent. Private rooms list their members, so the
/// agent becomes visible there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantableRoom {
    pub id: i64,
    pub name: String,
    pub private: bool,
}

pub fn grantable_rooms(conn: &crate::Connection, owner_id: i64) -> Result<Vec<GrantableRoom>> {
    let policy = super::agent_access::personal_room_sql("?", "r.id");
    crate::sql::query_all(
        conn,
        &format!("SELECT r.id,r.name,r.type!='Rooms::Open' FROM rooms r WHERE ({policy}) ORDER BY r.name,r.id"),
        [owner_id],
        |r| {
            Ok(GrantableRoom {
                id: r.get(0)?,
                name: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                private: r.get(2)?,
            })
        },
    )
}

pub fn replace_access(
    tx: &mut Tx<'_>,
    agent: &Agent,
    owner_id: i64,
    room_ids: &[i64],
    capabilities: &[String],
) -> Result<()> {
    use crate::{AgentGrant, Membership, NewGrant, Room};
    let mut errors = crate::Errors::default();
    let offered = grantable_rooms(tx.conn(), owner_id)?;
    // A room the owner can't choose gets this one error and no other check, so the answer is the
    // same whether it doesn't exist, is someone else's private room, or is a direct room.
    let (rooms, refused): (Vec<i64>, Vec<i64>) = room_ids
        .iter()
        .partition(|room| offered.iter().any(|offered| offered.id == **room));
    if !refused.is_empty() {
        errors.add("room_ids", "must be a non-direct room you belong to");
    }
    for capability in capabilities {
        // Validate even an empty room set against S1's capability allowlist.
        if !super::agent_access::PERSONAL_CAPABILITIES.contains(&capability.as_str()) {
            errors.add("capabilities", "is not available to personal agents");
        }
        for room in &rooms {
            let shape = AgentGrant::validate_request_shape(
                tx.conn(),
                &NewGrant {
                    agent_id: agent.id,
                    room_id: Some(*room),
                    capability: capability.clone(),
                    granted_by_id: owner_id,
                    ..Default::default()
                },
            )?;
            errors.0.extend(shape.0);
        }
    }
    errors.into_result()?;
    AgentGrant::revoke_for_agent(tx, agent.id)?;
    for membership in Membership::for_user(tx.conn(), agent.user_id)? {
        let room = Room::find(tx.conn(), membership.room_id)?;
        if !room.direct() && !room_ids.contains(&room.id) {
            room.revoke_from(tx, &[agent.user_id])?;
        }
    }
    for room in room_ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
    {
        Room::find(tx.conn(), room)?.grant_to(tx, &[agent.user_id])?;
        for capability in capabilities
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
        {
            AgentGrant::create(
                tx,
                NewGrant {
                    agent_id: agent.id,
                    room_id: Some(room),
                    capability: capability.clone(),
                    granted_by_id: owner_id,
                    ..Default::default()
                },
            )?;
        }
    }
    Ok(())
}
