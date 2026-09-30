//! Rendering and cable delivery for huddle effects, after the domain write commits.
use super::broadcasts::{Stream, room_dom_id};
use crate::app::App;
use campfire_db::models::huddle_grant::HuddleGrant;
use campfire_db::{Membership, Room};

pub(crate) fn presence(app: &App, room_id: i64) -> anyhow::Result<()> {
    if !app.config.huddle.configured() {
        return Ok(());
    }
    let now = app.db.env().now();
    let data = app.db.read_blocking(|conn| {
        let Some(room) = Room::find_by_id(conn, room_id)?.filter(|room| room.deleted_at.is_none())
        else {
            return Ok(None);
        };
        let users = HuddleGrant::participants_for(conn, room_id, now)?;
        let participants = users
            .iter()
            .map(|user| campfire_views::huddle::Participant {
                id: user.id,
                name: user.name.clone(),
                avatar_path: crate::controllers::presenters::avatar_path(&app.secrets, user),
            })
            .collect::<Vec<_>>();
        let member_ids = Membership::for_room(conn, room_id)?
            .into_iter()
            .map(|m| m.user_id)
            .collect::<Vec<_>>();
        Ok(Some((room, participants, member_ids)))
    })?;
    let Some((room, participants, member_ids)) = data else {
        return Ok(());
    };
    // Rails renders the sidebar partial once, even for a room with hundreds of members.
    let sidebar = campfire_views::huddle::participants(
        room.room_type.class_name(),
        room.id,
        "sidebar",
        &participants,
    );
    let target = room_dom_id(&room, "sidebar_voice_participants");
    for user_id in member_ids {
        app.broadcasts
            .replace(&Stream::user_rooms(user_id), &target, &sidebar);
    }
    let header = campfire_views::huddle::participants(
        room.room_type.class_name(),
        room.id,
        "header",
        &participants,
    );
    app.broadcasts.replace(
        &Stream::room_messages(&room),
        &room_dom_id(&room, "header_voice_participants"),
        &header,
    );
    Ok(())
}
