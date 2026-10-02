//! Public credential/grant page facts; plaintext secrets never enter these records.
use campfire_db::{Agent, Connection, Result, Room, User};
use campfire_views::accounts::bot_access::{Credential, CredentialExpiry, Grant};
pub fn credentials(
    conn: &Connection,
    agent_id: i64,
    zone: &campfire_views::time::Zone,
) -> Result<Vec<Credential>> {
    let mut statement=conn.prepare("SELECT c.id,c.name,c.token_last_four,u.name,c.created_at,c.expires_at,c.last_used_at,c.revoked_at IS NOT NULL FROM agent_credentials c JOIN users u ON u.id=c.created_by_id WHERE c.agent_id=? ORDER BY c.created_at DESC")?;
    Ok(statement
        .query_map([agent_id], |r| {
            Ok(Credential {
                id: r.get(0)?,
                name: r.get(1)?,
                last_four: r.get(2)?,
                created_by: r.get(3)?,
                created_at: r.get::<_, campfire_db::Timestamp>(4)?.jiff(),
                expires_at: r.get::<_, Option<campfire_db::Timestamp>>(5)?.map(|t| {
                    CredentialExpiry::Extended {
                        datetime:
                            crate::controllers::accounts::bots::input_casts::extended_datetime(
                                t, zone, true,
                            ),
                        microseconds: t.as_microsecond(),
                    }
                }),
                last_used_at: r
                    .get::<_, Option<campfire_db::Timestamp>>(6)?
                    .map(|t| t.jiff()),
                revoked: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}
type GrantPage = (bool, Vec<Grant>, Vec<(String, String)>);
pub fn grants(conn: &Connection, agent_id: i64, bot_id: i64, viewer: &User) -> Result<GrantPage> {
    let agent = Agent::find(conn, agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
    let mut statement=conn.prepare("SELECT g.id,g.capability,g.room_id,u.name,g.created_at,g.revoked_at IS NOT NULL FROM agent_grants g JOIN users u ON u.id=g.granted_by_id WHERE g.agent_id=? ORDER BY g.revoked_at,g.capability,g.room_id")?;
    let mut grants = Vec::new();
    let rows = statement.query_map([agent_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<i64>>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, campfire_db::Timestamp>(4)?,
            r.get::<_, bool>(5)?,
        ))
    })?;
    for row in rows {
        let (id, capability, room_id, granted_by, created_at, revoked) = row?;
        let room_name = match room_id {
            None => "Workspace-wide".into(),
            Some(id) => match Room::find_by_id(conn, id)? {
                Some(room) => super::room_display_name(conn, &room, viewer)?,
                None => "Deleted room".into(),
            },
        };
        grants.push(Grant {
            id,
            capability,
            room_name,
            granted_by,
            created_at: created_at.jiff(),
            revoked,
        });
    }
    let mut rooms = Room::for_user(conn, bot_id)?;
    super::sort_by_lower_name(&mut rooms, |room| room.name.as_deref().unwrap_or(""));
    let rooms = rooms
        .iter()
        .map(|r| Ok((super::room_display_name(conn, r, viewer)?, r.id.to_string())))
        .collect::<Result<_>>()?;
    Ok((agent.legacy_capabilities(conn)?, grants, rooms))
}
