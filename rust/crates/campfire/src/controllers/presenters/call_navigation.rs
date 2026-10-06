//! Request data for the room header. Domain signatures remain unchanged.
use crate::controllers::presenters::{Presenter, avatar_path};
use campfire_db::{CachedStatements, Membership, Room, User};
use campfire_views::{helpers::IconSource, rooms::navigation::Navigation};

pub(crate) fn model(
    app: &crate::app::AppState,
    conn: &campfire_db::Connection,
    room: &Room,
    user: &User,
) -> campfire_db::Result<Navigation> {
    let member = Membership::find_by_room_and_user(conn, room.id, user.id)?;
    let stage = if room.stage() {
        member
            .as_ref()
            .map(|m| super::calls::stage_model(app, conn, room.id, m.id))
            .transpose()?
    } else {
        None
    };
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
            avatar_path: avatar_path(&app.secrets, u),
        })
        .collect()
    } else {
        Vec::new()
    };
    let pins_count = conn.query_row_cached(
        "SELECT COUNT(*) FROM message_pins WHERE room_id=?",
        [room.id],
        |r| r.get(0),
    )?;
    Ok(Navigation {
        room: Presenter::new(conn, app, None).room_view(room, user)?,
        icon: room
            .icon_name
            .as_deref()
            .and_then(|n| Presenter::new(conn, app, None).resolve_avatar_icon(n)),
        pins_count,
        involvement: member
            .and_then(|m| m.involvement)
            .map(|i| i.name())
            .unwrap_or_else(|| room.default_involvement())
            .into(),
        participants,
        stage,
    })
}

pub(crate) fn edit_sections(
    _app: &crate::app::AppState,
    conn: &campfire_db::Connection,
    room: &Room,
    user: &User,
) -> campfire_db::Result<campfire_views::rooms::edit_sections::EditSections> {
    use campfire_views::rooms::edit_sections::{EditSections, Repository};
    let mut statement=conn.prepare_cached("SELECT id,owner,repo,events FROM github_repository_subscriptions WHERE room_id=? ORDER BY owner,repo")?;
    let repositories = statement
        .query_map([room.id], |r| {
            let events: String = r.get(3)?;
            Ok(Repository {
                id: r.get(0)?,
                owner: r.get(1)?,
                repo: r.get(2)?,
                events: serde_json::from_str(&events).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(EditSections {
        room_id: room.id,
        can_administer: user.can_administer(Some(room.creator_id), false),
        administrator: user.is_administrator(),
        repositories,
        inbound_domain: std::env::var("INBOUND_EMAIL_DOMAIN")
            .ok()
            .filter(|s| !campfire_richtext::ruby::is_blank(s)),
        inbound_token: room.inbound_email_token.clone(),
    })
}
