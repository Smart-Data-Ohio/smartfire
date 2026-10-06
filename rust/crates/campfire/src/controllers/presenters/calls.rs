//! The call (voice and stage channel) view models: sidebar rows, and the stage's members and
//! live stream, shared by the call controllers, the room navigation and the huddle broadcasts.
use campfire_db::{Membership, Room, User};
use campfire_views::helpers::IconSource;
use campfire_views::rooms::calls::CallRow;

use super::Presenter;

pub(crate) fn row(
    app: &crate::app::App,
    conn: &campfire_db::Connection,
    room: &Room,
) -> campfire_db::Result<CallRow> {
    let participants = if app.config.huddle.configured() {
        campfire_db::models::huddle_grant::HuddleGrant::participants_for(
            conn,
            room.id,
            app.db.env().now(),
        )?
        .iter()
        .map(|u| campfire_views::huddle::Participant {
            id: u.id,
            name: u.name.clone(),
            avatar_path: crate::controllers::presenters::avatar_path(&app.secrets, u),
        })
        .collect()
    } else {
        Vec::new()
    };
    let live = campfire_db::models::stream::Stream::live_for_room(conn, room.id)?;
    let live_name = live
        .as_ref()
        .map(|s| User::find(conn, s.user_id).map(|u| u.name))
        .transpose()?
        .unwrap_or_default();
    Ok(row_with_call_facts(app, conn, room, participants, live.is_some(), live_name))
}

pub(crate) fn row_with_call_facts(
    app: &crate::app::App,
    conn: &campfire_db::Connection,
    room: &Room,
    participants: Vec<campfire_views::huddle::Participant>,
    live: bool,
    live_name: String,
) -> CallRow {
    CallRow {
        id: room.id,
        name: room.name.clone().unwrap_or_default(),
        stage: room.stage(),
        icon: room
            .icon_name
            .as_deref()
            .and_then(|n| Presenter::new(conn, app, None).resolve_avatar_icon(n)),
        participants,
        live,
        live_name,
        unread: false,
        muted: false,
        membership: false,
        favorited: false,
        favorite_position: None,
        category_id: None,
        can_delete: false,
    }
}

pub(crate) fn stage_model(
    app: &crate::app::AppState,
    conn: &campfire_db::Connection,
    room_id: i64,
    viewer_id: i64,
) -> campfire_db::Result<campfire_views::huddle_stage::Stage> {
    use campfire_db::CachedStatements;
    use rusqlite::OptionalExtension;
    let members = Membership::for_room(conn, room_id)?
        .into_iter()
        .map(|m| {
            let user = campfire_db::User::find(conn, m.user_id)?;
            Ok(campfire_views::huddle_stage::Member {
                id: m.id,
                user_id: user.id,
                name: user.name.clone(),
                avatar_path: crate::controllers::presenters::avatar_path(&app.secrets, &user),
                administrator: user.is_administrator(),
                role: m.stage_role.map_or(String::new(), |r| r.name().into()),
                hand: m.hand_raised_at.map(|at| at.as_microsecond()),
                muted: m.server_muted_at.is_some(),
            })
        })
        .collect::<campfire_db::Result<Vec<_>>>()?;
    let live=campfire_db::models::stream::Stream::live_for_room(conn,room_id)?.map(|stream| {
        let user=campfire_db::User::find(conn,stream.user_id)?;
        let identity=conn.query_row_cached("SELECT identity FROM huddle_grants WHERE room_id=? AND membership_id=? AND revoked_at IS NULL ORDER BY last_issued_at DESC LIMIT 1",rusqlite::params![room_id,stream.membership_id],|r|r.get(0)).optional()?;
        Ok::<_,campfire_db::Error>(campfire_views::huddle_stage::Live {id:stream.id,membership_id:stream.membership_id,name:user.name,identity})
    }).transpose()?;
    Ok(campfire_views::huddle_stage::Stage {
        room_id,
        viewer_id,
        members,
        live,
    })
}
