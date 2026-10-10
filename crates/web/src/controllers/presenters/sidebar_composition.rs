//! Request adapter for the full sidebar; domain and notification seams stay untouched.
use crate::app::App;
use crate::controllers::presenters::{self, calls as call_channels};
use campfire_db::{Account, Connection, Involvement, Membership, Room, RoomCategory, User};
use campfire_views::users::sidebar_composition::{Category, Person, Row, Sidebar};
pub fn load(
    app: &App,
    conn: &Connection,
    user: &User,
) -> campfire_db::Result<(Sidebar, presenters::accounts::Sidebar)> {
    let account = Account::first(conn)?;
    let logo = account
        .as_ref()
        .map(|a| presenters::attachments::attached_blob(conn, "Account", a.id, "logo"))
        .transpose()?
        .flatten();
    let direct_members = direct_members(conn, user.id)?;
    let call_facts = call_facts(app, conn, user.id)?;
    let mut favorites = Vec::new();
    let mut channels = Vec::new();
    let mut boards = Vec::new();
    let mut voice = Vec::new();
    let mut stage = Vec::new();
    let mut direct = Vec::new();
    let mut categories: Vec<Category> = RoomCategory::ordered_for_user(conn, user.id)?
        .into_iter()
        .map(|c| Category {
            id: c.id,
            name: c.name,
            collapsed: c.collapsed,
            rows: Vec::new(),
        })
        .collect();
    for (membership, room) in Membership::visible_with_ordered_room(conn, user.id)? {
        let row = row(app, conn, user, &membership, &room, &direct_members, Some(&call_facts))?;
        if membership.favorited() {
            favorites.push((membership.favorite_position, membership.id, row));
            continue;
        }
        // Rails extracts direct/Voice home lists independently of category
        // membership. Its model permits categorized rows of any room type.
        if let Some(id) = membership.room_category_id
            && let Some(category) = categories.iter_mut().find(|c| c.id == id)
        {
            let mut categorized = row.clone();
            categorized.category_row = true;
            categorized.name = room.name.clone().unwrap_or_default();
            category.rows.push(categorized);
        }
        if room.direct() {
            direct.push((room.updated_at, row));
        } else if room.voice() {
            voice.push(row);
        } else if membership.room_category_id.is_none() {
            if room.stage() {
                stage.push(row);
            } else if room.room_type == campfire_db::RoomType::Board {
                boards.push(row);
            } else {
                channels.push(row);
            }
        }
    }
    favorites.sort_by_key(|(position, id, _)| (*position, *id));
    direct.sort_by_key(|(at, _)| *at);
    direct.reverse();
    let legacy = presenters::accounts::sidebar(conn, &app.secrets, user)?;
    let placeholders = legacy
        .direct_placeholder_users
        .iter()
        .map(|u| Person {
            id: u.id,
            name: u.name.clone(),
            avatar_path: u.avatar_path.clone(),
        })
        .collect();
    Ok((
        Sidebar {
            account_name: account.as_ref().map(|a| a.name.clone()).unwrap_or_default(),
            logo_path: logo
                .map(|_| presenters::accounts::fresh_account_logo_path(account.as_ref(), None)),
            actor: person(app, user),
            configured: app.config.huddle.configured(),
            can_create: user.is_administrator()
                || !account
                    .is_some_and(|a| a.settings().restrict_room_creation_to_administrators()),
            favorites: favorites.into_iter().map(|(_, _, r)| r).collect(),
            channels,
            boards,
            voice,
            stage,
            direct: direct.into_iter().map(|(_, r)| r).collect(),
            placeholders,
            categories,
        },
        legacy,
    ))
}
pub fn row(
    app: &App,
    conn: &Connection,
    user: &User,
    membership: &Membership,
    room: &Room,
    direct_members: &std::collections::BTreeMap<i64, Vec<User>>,
    call_facts: Option<&std::collections::BTreeMap<i64, CallFacts>>,
) -> campfire_db::Result<Row> {
    let mut members = if room.direct() {
        direct_members
            .get(&room.id)
            .into_iter()
            .flatten()
            .filter(|u| u.id != user.id)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let group = room.direct()
        && (members.len() > 1 || room.name.as_deref().is_some_and(|n| !n.trim().is_empty()));
    if room.direct() && members.is_empty() {
        members.push(user.clone());
    }
    let name = if room.direct() {
        if group {
            room.direct_display_name(conn, Some(user), Some(&members))?
                .unwrap_or_default()
        } else {
            members[0]
                .name
                .split_whitespace()
                .next()
                .unwrap_or("")
                .into()
        }
    } else {
        room.name.clone().unwrap_or_default()
    };
    let mut call = if let Some(facts) = call_facts {
        let facts = facts.get(&room.id).cloned().unwrap_or_default();
        call_channels::row_with_call_facts(app, conn, room, facts.participants, facts.live, facts.live_name)
    } else {
        call_channels::row(app, conn, room)?
    };
    call.name = name.clone();
    call.unread = membership.unread();
    call.muted = membership.involvement == Some(Involvement::Muted);
    call.membership = true;
    call.favorited = membership.favorited();
    call.favorite_position = membership.favorite_position;
    call.category_id = membership.room_category_id;
    call.can_delete = user.is_administrator() || (!group && room.creator_id == user.id);
    // app/views/users/sidebars/show.html.erb: a peer rename is outside this
    // collection key; a sighting changes participant IDs and invalidates it.
    let direct_cache_key = if room.direct() {
        let record = campfire_views::fragment_cache::cache_key_with_version(
            "memberships", membership.id, membership.updated_at.jiff());
        let participants = call.participants.iter().map(|p| p.id).collect::<Vec<_>>();
        let membership_key = campfire_views::fragment_cache::keys::sidebar_membership(
            &record, Some(&participants), user.is_administrator());
        Some(campfire_views::fragment_cache::keys::fragment(
            "users/sidebars/rooms/_direct",
            &campfire_views::fragment_cache::digest(&[include_str!("../../../../views/templates/users/sidebars/composition/_direct.html")]),
            &membership_key, &campfire_views::time::Zone::utc()))
    } else { None };
    Ok(Row {
        id: room.id,
        kind: presenters::accounts::room_param_key(room.room_type)
            .trim_start_matches("rooms_")
            .into(),
        name,
        raw_name: room.name.clone(),
        category_row: false,
        epoch: presenters::epoch_string(room.updated_at.jiff()),
        direct_cache_key,
        members: members.iter().map(|u| person(app, u)).collect(),
        call,
    })
}

/// Controller broadcasts render shared rows without recipient membership state.
#[cfg(any(test, feature = "test-support"))]
pub fn neutral(app: &App, conn: &Connection, room: &Room) -> campfire_db::Result<Row> {
    let legacy = presenters::Presenter::new(conn, app, None).sidebar_room(room);
    let mut call = call_channels::row(app, conn, room)?;
    call.membership = false;
    call.can_delete = false;
    Ok(Row {
        direct_cache_key: None,
        id: room.id,
        kind: presenters::accounts::room_param_key(room.room_type)
            .trim_start_matches("rooms_")
            .into(),
        name: legacy.name,
        raw_name: room.name.clone(),
        category_row: false,
        epoch: presenters::epoch_string(room.updated_at.jiff()),
        members: Vec::new(),
        call,
    })
}
/// Member ids carry the Rails callback's avatar order. Labels sort independently.
pub fn for_membership(
    app: &App,
    conn: &Connection,
    membership: &Membership,
    member_ids: Option<&[i64]>,
) -> campfire_db::Result<Row> {
    let room = Room::find(conn, membership.room_id)?;
    let legacy = if room.direct() {
        Some(presenters::Presenter::new(conn, app, None).sidebar_direct(membership)?)
    } else {
        None
    };
    let viewer = User::find(conn, membership.user_id)?;
    let members = if !room.direct() {
        Vec::new()
    } else if let Some(ids) = member_ids {
        ids.iter()
            .map(|id| User::find(conn, *id))
            .collect::<campfire_db::Result<Vec<_>>>()?
    } else {
        Membership::for_room(conn, room.id)?
            .iter()
            .map(|m| User::find(conn, m.user_id))
            .collect::<campfire_db::Result<Vec<_>>>()?
    };
    let mut composed = row(
        app,
        conn,
        &viewer,
        membership,
        &room,
        &std::collections::BTreeMap::from([(room.id, members)]),
        None,
    )?;
    if let Some(legacy) = legacy {
        composed.epoch = legacy.updated_at_epoch;
    }
    Ok(composed)
}
pub use campfire_runtime::presenters::sidebar_composition::*;
