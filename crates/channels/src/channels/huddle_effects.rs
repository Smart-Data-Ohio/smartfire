//! Rendering and cable delivery for huddle effects, after the domain write commits.
use super::broadcasts::{Stream, room_dom_id};
use crate::app::App;
use crate::cable::huddle_sync;
use campfire_db::models::huddle_grant::HuddleGrant;
use campfire_db::{Membership, Room};

pub use crate::controllers::presenters::calls::stage_model;

pub(crate) fn stream_changed(app: &App, room_id: i64) -> anyhow::Result<()> {
    let data = app.db.read_blocking(|conn| {
        let Some(room) =
            Room::find_by_id(conn, room_id)?.filter(|r| r.deleted_at.is_none() && r.stage())
        else {
            return Ok(None);
        };
        // The shared badge/dots also render for a stage with no memberships.
        let stage = stage_model(app, conn, room_id, 0)?;
        Ok(Some((room, stage)))
    })?;
    let Some((room, mut stage)) = data else {
        return Ok(());
    };
    app.broadcasts.replace(
        &Stream::room_messages(&room),
        &room_dom_id(&room, "stage_live_badge"),
        &stage.render("live_badge"),
    );
    let dot = stage.render("live_dot");
    let event_dot = stage.render("venue_live_dot");
    for member in stage.members.clone() {
        stage.viewer_id = member.id;
        let stream = Stream::user_rooms(member.user_id);
        app.broadcasts
            .replace(&stream, &room_dom_id(&room, "sidebar_stage_live"), &dot);
        app.broadcasts
            .replace(&stream, &room_dom_id(&room, "event_stage_live"), &event_dot);
        app.broadcasts.replace(
            &stream,
            &room_dom_id(&room, "stage_panel"),
            &stage.render("panel_body"),
        );
    }
    if app.cable.sync_wanted() {
        let now = app.db.env().now();
        sync_read(app, "stream", room.id, |conn| {
            huddle_sync::publish_stage(&app.cable, conn, room.id);
            if app.config.huddle.configured() {
                huddle_sync::publish_presence(&app.cable, conn, &room, now);
            }
            Ok(())
        });
    }
    Ok(())
}

/// Runs a sync twin's read after the classic frames have gone out: a failure there is logged, not
/// returned, so it can't fail an effect whose frames were already delivered (as `huddle_sync`'s own
/// publishers do).
fn sync_read(
    app: &App,
    what: &str,
    room_id: i64,
    f: impl FnOnce(&campfire_db::Connection) -> campfire_db::Result<()>,
) {
    if let Err(error) = app.db.read_blocking(f) {
        tracing::warn!(%error, room_id, what, "sync: huddle twin not read");
    }
}

pub(crate) fn stream_stopped(app: &App, room_id: i64, user_id: i64) -> anyhow::Result<()> {
    let alive = app.db.read_blocking(|conn| {
        Ok(Room::find_by_id(conn, room_id)?.is_some_and(|r| r.deleted_at.is_none() && r.stage()))
    })?;
    if alive {
        app.broadcasts.append(
            &Stream::user_rooms(user_id),
            "huddle_role_events",
            &campfire_views::huddle_stage::stream_event(room_id),
        );
        huddle_sync::publish_stream_stopped(&app.cable, room_id, user_id);
    }
    Ok(())
}

pub(crate) fn stage_roster(app: &App, room_id: i64) -> anyhow::Result<()> {
    let mut stage = app
        .db
        .read_blocking(|conn| stage_model(app, conn, room_id, 0))?;
    for member in stage.members.clone() {
        stage.viewer_id = member.id;
        app.broadcasts.replace(
            &Stream::user_rooms(member.user_id),
            &stage.dom_id("stage_roster"),
            &stage.render("roster"),
        );
    }
    publish_stage(app, room_id)
}

/// `stage.updated`, for a stage that's still there.
fn publish_stage(app: &App, room_id: i64) -> anyhow::Result<()> {
    if app.cable.sync_wanted() {
        sync_read(app, "stage", room_id, |conn| {
            if Room::find_by_id(conn, room_id)?.is_some_and(|r| r.deleted_at.is_none() && r.stage()) {
                huddle_sync::publish_stage(&app.cable, conn, room_id);
            }
            Ok(())
        });
    }
    Ok(())
}

pub(crate) fn role_event(app: &App, room_id: i64, membership_id: i64) -> anyhow::Result<()> {
    let member = app.db.read_blocking(|conn| {
        Ok(Membership::for_room(conn, room_id)?
            .into_iter()
            .find(|m| m.id == membership_id))
    })?;
    if let Some(member) = member {
        app.broadcasts.append(
            &Stream::user_rooms(member.user_id),
            "huddle_role_events",
            &campfire_views::huddle_stage::role_event(
                room_id,
                member.stage_role.map_or("", |r| r.name()),
                member.server_muted_at.is_some(),
            ),
        );
        huddle_sync::publish_role(&app.cable, &member);
    }
    Ok(())
}

pub fn presence(app: &App, room_id: i64) -> anyhow::Result<()> {
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
    if app.cable.sync_wanted() {
        sync_read(app, "presence", room_id, |conn| {
            huddle_sync::publish_presence(&app.cable, conn, &room, now);
            Ok(())
        });
    }
    Ok(())
}

pub(crate) fn stage_panel(app:&App,room_id:i64,membership_id:i64)->anyhow::Result<()> {
    let stage=app.db.read_blocking(|conn|stage_model(app,conn,room_id,membership_id))?;
    if let Some(member)=stage.members.iter().find(|m|m.id==membership_id) {
        app.broadcasts.replace(&Stream::user_rooms(member.user_id),&stage.dom_id("stage_panel"),&stage.panel(false));
    }
    // No `stage.updated` here: the panel only ever goes out beside a `StageRoster`, which sends it.
    Ok(())
}

// Stage owns this quiet note; the general WS8b message descriptor remains its own seam.
pub fn stage_note_html(app:&App,conn:&campfire_db::Connection,message_id:i64)->campfire_db::Result<Option<(Room,String)>> {
    let Some(message)=campfire_db::Message::find_by_id(conn,message_id)? else {return Ok(None);};
    let Some(room)=Room::find_by_id(conn,message.room_id)?.filter(|r|r.stage() && r.deleted_at.is_none()) else {return Ok(None);};
    if !message.system_note {return Ok(None);}
    let presenter=crate::controllers::presenters::Presenter::new(conn,app,None);
    let view=presenter.message(&message)?;
    let html=crate::controllers::presenters::page::render_detached(app,None,|ctx|campfire_views::messages::message(ctx,&view));
    Ok(Some((room,html)))
}
pub(crate) fn stage_ended_note(app:&App,message_id:i64)->anyhow::Result<()> {
    if let Some((room,html))=app.db.read_blocking(|conn|stage_note_html(app,conn,message_id))? {
        app.broadcasts.append(&Stream::room_messages(&room),&room_dom_id(&room,"messages"),&html);
        // `message.created`, the same twin as any post's.
        sync_read(app, "stage ended note", room.id, |conn| {
            if let Some(message) = campfire_db::Message::find_by_id(conn, message_id)? {
                app.broadcasts.sync_message(conn, &message, true);
            }
            Ok(())
        });
    }
    Ok(())
}

use campfire_views::rendering::*;
use campfire_web::controllers::presenters::{Rendering};
