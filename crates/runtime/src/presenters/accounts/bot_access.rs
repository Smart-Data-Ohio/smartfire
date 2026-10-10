//! Public credential/grant page facts; plaintext secrets never enter these records.
use campfire_db::{Agent, Connection, Result, Room, User};
use campfire_presentation::accounts::bot_access::{Credential, CredentialExpiry, Grant};
pub fn credentials(
    conn: &Connection,
    agent_id: i64,
    zone: &campfire_presentation::time::Zone,
) -> Result<Vec<Credential>> {
    let mut statement=conn.prepare("SELECT u.*,c.id AS credential_id,c.name AS credential_name,c.token_last_four,c.created_at AS credential_created_at,c.expires_at,c.last_used_at,c.revoked_at IS NOT NULL AS revoked FROM agent_credentials c JOIN users u ON u.id=c.created_by_id WHERE c.agent_id=? ORDER BY c.created_at DESC")?;
    Ok(statement
        .query_map([agent_id], |r| {
            Ok(Credential {
                id: r.get("credential_id")?,
                name: r.get("credential_name")?,
                last_four: r.get("token_last_four")?,
                created_by: User::from_row(r)?.display_name().to_owned(),
                created_at: r.get::<_, campfire_db::Timestamp>("credential_created_at")?.jiff(),
                expires_at: r.get::<_, Option<campfire_db::Timestamp>>("expires_at")?.map(|t| {
                    CredentialExpiry::Extended {
                        datetime:
                            crate::controllers::presenters::bot_input_casts::extended_datetime(
                                t, zone, true,
                            ),
                        microseconds: t.as_wide_microsecond(),
                    }
                }),
                last_used_at: r
                    .get::<_, Option<campfire_db::Timestamp>>("last_used_at")?
                    .map(|t| t.jiff()),
                revoked: r.get("revoked")?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}
type GrantPage = (bool, Vec<Grant>, Vec<(String, String)>);
pub fn grants(conn: &Connection, agent_id: i64, bot_id: i64, viewer: &User) -> Result<GrantPage> {
    let agent = Agent::find(conn, agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
    let mut statement=conn.prepare("SELECT u.*,g.id AS grant_id,g.capability,g.room_id,g.created_at AS grant_created_at,g.revoked_at IS NOT NULL AS revoked FROM agent_grants g JOIN users u ON u.id=g.granted_by_id WHERE g.agent_id=? ORDER BY g.revoked_at,g.capability,g.room_id")?;
    let mut grants = Vec::new();
    let rows = statement
        .query_map([agent_id], |r| {
            Ok((
                r.get::<_, i64>("grant_id")?,
                r.get::<_, String>("capability")?,
                r.get::<_, Option<i64>>("room_id")?,
                User::from_row(r)?.display_name().to_owned(),
                r.get::<_, campfire_db::Timestamp>("grant_created_at")?,
                r.get::<_, bool>("revoked")?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut ids: Vec<_> = rows.iter().filter_map(|row| row.2).collect();
    ids.sort_unstable();
    ids.dedup();
    let names: std::collections::HashMap<_, _> = Room::for_ids(conn, &ids)?
        .iter()
        .map(|room| Ok((room.id, super::room_display_name(conn, room, viewer)?)))
        .collect::<Result<_>>()?;
    for (id, capability, room_id, granted_by, created_at, revoked) in rows {
        let room_name = match room_id {
            None => "Workspace-wide".into(),
            Some(id) => names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| "Deleted room".into()),
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
