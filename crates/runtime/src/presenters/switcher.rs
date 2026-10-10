//! Quick-switcher read model. Five scoped queries regardless of collection size; no HTML.
use campfire_db::{Connection, Membership, RoomType, User};
use rails_compat::Secrets;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct Switcher {
    pub rooms: Vec<SwitcherRoom>,
    pub people: Vec<SwitcherPerson>,
    pub threads: Vec<SwitcherThread>,
}
#[derive(Serialize)]
pub struct SwitcherRoom {
    pub id: i64,
    pub name: Option<String>,
    pub kind: &'static str,
    pub url: String,
    pub icon_name: Option<String>,
    pub unread: bool,
    pub muted: bool,
    pub favorite: bool,
}
#[derive(Serialize)]
pub struct SwitcherPerson {
    pub id: i64,
    pub name: String,
    pub avatar_url: String,
    pub dm_url: Option<String>,
    /// The one-to-one room `dm_url` opens.
    #[serde(skip)]
    pub dm_room_id: Option<i64>,
}
#[derive(Serialize)]
pub struct SwitcherThread {
    pub id: i64,
    pub name: String,
    pub room_name: Option<String>,
    pub room_id: i64,
    pub url: String,
}

pub fn load(
    conn: &Connection,
    secrets: &Secrets,
    user: &User,
    base_url: &str,
) -> campfire_db::Result<Switcher> {
    let memberships = Membership::visible_with_ordered_room(conn, user.id)?;
    let mut members_by_room: HashMap<i64, Vec<User>> = HashMap::new();
    let mut statement = conn.prepare("SELECT memberships.room_id, users.* FROM memberships INNER JOIN users ON users.id=memberships.user_id WHERE memberships.room_id IN (SELECT memberships.room_id FROM memberships INNER JOIN rooms ON rooms.id=memberships.room_id WHERE memberships.user_id=? AND rooms.type='Rooms::Direct') ORDER BY memberships.id")?;
    for row in statement.query_map([user.id], |row| {
        Ok((row.get::<_, i64>("room_id")?, User::from_row(row)?))
    })? {
        let (id, member) = row?;
        members_by_room.entry(id).or_default().push(member);
    }
    let rooms = memberships
        .into_iter()
        .map(|(membership, room)| {
            let members = members_by_room
                .get(&room.id)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let name = if room.direct() {
                let names: Vec<String> = members
                    .iter()
                    .filter(|u| u.id != user.id)
                    .map(|u| u.name.clone())
                    .collect();
                let label = campfire_presentation::helpers::to_sentence(&names, " and ");
                Some(if label.trim().is_empty() {
                    user.name.clone()
                } else {
                    label
                })
            } else {
                room.name
            };
            let kind = match room.room_type {
                RoomType::Direct => {
                    if members.len() > 2 {
                        "group"
                    } else {
                        "dm"
                    }
                }
                RoomType::Voice => "voice",
                RoomType::Stage => "stage",
                RoomType::Board => "board",
                _ => "channel",
            };
            SwitcherRoom {
                id: room.id,
                name,
                kind,
                url: campfire_routes::room(room.id),
                icon_name: room.icon_name,
                unread: membership.unread(),
                muted: membership.involved_in(campfire_db::Involvement::Muted),
                favorite: membership.favorited(),
            }
        })
        .collect();
    let mut statement =
        conn.prepare("SELECT * FROM users WHERE status=0 AND role!=2 AND id!=? ORDER BY name")?;
    let users = statement
        .query_map([user.id], User::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut statement=conn.prepare("SELECT memberships.room_id, memberships.user_id FROM memberships WHERE memberships.user_id!=? AND memberships.room_id IN (SELECT peer.room_id FROM memberships peer INNER JOIN rooms ON rooms.id=peer.room_id WHERE peer.user_id=? AND rooms.type='Rooms::Direct') AND memberships.room_id IN (SELECT room_id FROM memberships GROUP BY room_id HAVING COUNT(*)=2)")?;
    let rooms_by_user: HashMap<i64, i64> = statement
        .query_map([user.id, user.id], |row| {
            Ok((row.get::<_, i64>(1)?, row.get::<_, i64>(0)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let people = users
        .into_iter()
        .map(|u| SwitcherPerson {
            id: u.id,
            name: u.name.clone(),
            avatar_url: format!("{base_url}{}", super::avatar_path(secrets, &u)),
            dm_url: rooms_by_user.get(&u.id).map(|id| campfire_routes::room(*id)),
            dm_room_id: rooms_by_user.get(&u.id).copied(),
        })
        .collect();
    let mut statement=conn.prepare("SELECT channel_threads.id,channel_threads.name,rooms.name,rooms.id FROM channel_threads INNER JOIN rooms ON rooms.id=channel_threads.room_id WHERE rooms.deleted_at IS NULL AND rooms.id IN (SELECT room_id FROM memberships WHERE user_id=?) ORDER BY channel_threads.last_activity_at DESC LIMIT 15")?;
    let threads = statement
        .query_map([user.id], |row| {
            let id = row.get::<_, i64>(0)?;
            let room_id = row.get::<_, i64>(3)?;
            Ok(SwitcherThread {
                id,
                name: row.get(1)?,
                room_name: row.get(2)?,
                room_id,
                url: format!("{base_url}{}?thread={id}", campfire_routes::room(room_id)),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Switcher {
        rooms,
        people,
        threads,
    })
}
