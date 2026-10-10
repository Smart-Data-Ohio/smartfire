//! Request adapter for the full sidebar; domain and notification seams stay untouched.
use crate::app::App;
use crate::controllers::presenters::{self};
use campfire_db::{Connection, User};
use campfire_presentation::users::sidebar_composition::Person;
use rails_compat::unicode;
pub fn person(app: &App, user: &User) -> Person {
    Person {
        id: user.id,
        name: user.name.clone(),
        avatar_path: presenters::avatar_path(&app.secrets, user),
    }
}

#[derive(Clone, Default)]
pub struct CallFacts {
    pub participants: Vec<campfire_presentation::huddle::Participant>,
    pub live: bool,
    pub live_name: String,
}

pub fn call_facts(app: &App, conn: &Connection, user_id: i64) -> campfire_db::Result<std::collections::BTreeMap<i64, CallFacts>> {
    use campfire_db::models::huddle_grant::IN_CALL_WINDOW;
    use rusqlite::params;
    let mut result = std::collections::BTreeMap::<i64, CallFacts>::new();
    // Users::SidebarHelper loads participants once for all the viewer's rooms,
    // deduplicates devices, and sorts by case-insensitive display name.
    if app.config.huddle.configured() {
        let mut statement = conn.prepare_cached("SELECT DISTINCT g.room_id,u.* FROM huddle_grants g JOIN users u ON u.id=g.user_id WHERE g.revoked_at IS NULL AND g.last_seen_at>? AND g.room_id IN (SELECT room_id FROM memberships WHERE user_id=?)")?;
        let rows = statement.query_map(params![app.db.env().now().ago(jiff::SignedDuration::from_secs(IN_CALL_WINDOW)), user_id], |r| Ok((r.get::<_, i64>("room_id")?, User::from_row(r)?)))?.collect::<Result<Vec<_>, _>>()?;
        let mut users = std::collections::BTreeMap::<i64, Vec<User>>::new();
        for (room_id, user) in rows { users.entry(room_id).or_default().push(user); }
        for (room_id, mut users) in users {
            users.sort_by_key(|u| unicode::downcase(&u.name));
            result.entry(room_id).or_default().participants = users.iter().map(|u| campfire_presentation::huddle::Participant {id: u.id, name: u.name.clone(), avatar_path: presenters::avatar_path(&app.secrets,u)}).collect();
        }
    }
    // Users::SidebarsController preloads Stage live_streams and their presenter.
    let mut statement = conn.prepare_cached("SELECT s.room_id,u.name FROM streams s LEFT JOIN users u ON u.id=s.user_id WHERE s.ended_at IS NULL AND s.room_id IN (SELECT m.room_id FROM memberships m JOIN rooms r ON r.id=m.room_id WHERE m.user_id=? AND r.type='Rooms::Stage') ORDER BY s.id")?;
    let rows = statement.query_map([user_id], |r| Ok((r.get::<_,i64>(0)?,r.get::<_,Option<String>>(1)?)))?.collect::<Result<Vec<_>, _>>()?;
    for (room_id, name) in rows {
        let facts = result.entry(room_id).or_default();
        if !facts.live {
            facts.live = true;
            facts.live_name = name.ok_or(campfire_db::Error::RecordNotFound("User"))?;
        }
    }
    Ok(result)
}

pub fn direct_members(
    conn: &Connection,
    user_id: i64,
) -> campfire_db::Result<std::collections::BTreeMap<i64, Vec<User>>> {
    // SELECT * preserves the same SQLite access path and association order as
    // Rails' preloaded memberships; labels sort independently of avatar order.
    let mut statement=conn.prepare_cached("SELECT memberships.* FROM memberships WHERE room_id IN (SELECT memberships.room_id FROM memberships INNER JOIN rooms ON rooms.id=memberships.room_id WHERE rooms.type='Rooms::Direct' AND memberships.user_id=?)")?;
    let pairs = statement
        .query_map([user_id], |r| {
            Ok((r.get::<_, i64>("room_id")?, r.get::<_, i64>("user_id")?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    // Preload users as Rails' includes(:user) does. Reassemble in the membership
    // query's order, so batching cannot change the group avatar order.
    let ids = pairs.iter().map(|(_, id)| *id).collect::<std::collections::BTreeSet<_>>();
    let users = User::where_ids(conn, &ids.into_iter().collect::<Vec<_>>())?
        .into_iter().map(|user| (user.id, user)).collect::<std::collections::BTreeMap<_, _>>();
    let mut grouped = std::collections::BTreeMap::<i64, Vec<User>>::new();
    for (room_id, id) in pairs {
        let user = users.get(&id).ok_or(campfire_db::Error::RecordNotFound("User"))?;
        grouped.entry(room_id).or_default().push(user.clone());
    }
    Ok(grouped)
}
