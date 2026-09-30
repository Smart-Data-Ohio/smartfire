//! Read projection for `Rooms::MembersController`. Agent behavior remains in WS11;
//! human status/presence uses the flagged, imported WS17 readers.
use std::collections::HashSet;

use rusqlite::{Connection, params};

use super::{
    user_status_settings::UserStatusSettings,
    workspace_presence_lease::{Presence, WorkspacePresenceLease},
};
use crate::sql::query_all;
use crate::{Agent, Result, Timestamp, User};

pub struct Member {
    pub user: User,
    pub agent: Option<Agent>,
    pub settings: UserStatusSettings,
    pub lease: Presence,
    pub starred: bool,
}

pub fn for_room(conn: &Connection, room: i64, viewer: i64, now: Timestamp) -> Result<Vec<Member>> {
    let users = query_all(
        conn,
        "SELECT users.* FROM users JOIN memberships ON memberships.user_id=users.id WHERE memberships.room_id=? AND users.status=0 ORDER BY LOWER(users.name),users.id",
        [room],
        User::from_row,
    )?;
    let ids: Vec<_> = users.iter().map(|user| user.id).collect();
    let mut settings = UserStatusSettings::for_ids(conn, &ids)?;
    let leases = WorkspacePresenceLease::presence_by_user_id(conn, &ids, now)?;
    // FLAGGED WS8b User::Starring reader seam (app/models/user/starring.rb).
    // Viewer qualification belongs in this query, including when the viewer is a member.
    let stars: HashSet<i64> = query_all(conn,
        "SELECT s.starred_user_id FROM user_stars s JOIN memberships m ON m.user_id=s.starred_user_id WHERE s.user_id=? AND m.room_id=?",
        params![viewer,room], |row| row.get(0))?.into_iter().collect();
    users
        .into_iter()
        .map(|user| {
            let agent = if user.is_bot() {
                Agent::for_user(conn, user.id)?
            } else {
                None
            };
            Ok(Member {
                settings: settings
                    .remove(&user.id)
                    .ok_or(crate::Error::RecordNotFound("User"))?,
                lease: leases.get(&user.id).copied().unwrap_or(Presence::Offline),
                starred: stars.contains(&user.id),
                agent,
                user,
            })
        })
        .collect()
}
