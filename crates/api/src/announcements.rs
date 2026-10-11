//! Same-instance announcement settings, publications and follows.
use crate::{
    dto,
    endpoints::{before_actions, body, set_room},
    error::fail,
};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::models::announcement::{self, Follow};
use campfire_db::{Message, Room, User};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_rooms::controllers::rooms::ensure_can_administer;
use campfire_runtime::concerns::{cast_integer, require_current_user};
use campfire_runtime::context::db_error;

endpoint!(set => set_announcement);
endpoint!(publish => publish_message);
endpoint!(index => list_follows);
endpoint!(create => create_follow);
endpoint!(destroy => delete_follow);

fn id(c: &Ctx, key: &str) -> Result<i64> {
    c.param_str(key)
        .and_then(cast_integer)
        .ok_or(Error::NotFound)
}

fn managed(conn: &campfire_db::Connection, viewer: i64, room_id: i64) -> campfire_db::Result<Room> {
    let room = Room::find_for_user(conn, viewer, room_id)?
        .ok_or(campfire_db::Error::RecordNotFound("Room"))?;
    if !User::find(conn, viewer)?.can_administer(Some(room.creator_id), false) {
        return Err(campfire_db::Error::RecordNotFound("Room"));
    }
    Ok(room)
}

async fn set_announcement(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    ensure_can_administer(c, &room)?;
    let input: api::SetAnnouncement = body(c).await?;
    let viewer = require_current_user(c)?.id;
    let updated = c
        .app()
        .db
        .write(move |tx| {
            managed(tx.conn(), viewer, room.id)?;
            announcement::set_flag(tx, room.id, input.announcement)
        })
        .await
        .map_err(db_error)?;
    crate::room_management::broadcast(c, &updated, true).await?;
    c.json(StatusCode::OK, &dto::room(&updated))
}

async fn publish_message(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let viewer = require_current_user(c)?.clone();
    let message_id = id(c, "message_id")?;
    let message = c
        .app()
        .db
        .read(move |conn| Message::find_reachable(conn, viewer.id, message_id))
        .await
        .map_err(db_error)?;
    let viewer = require_current_user(c)?.clone();
    if viewer.id != message.creator_id && !viewer.is_administrator() {
        return Err(fail(
            c,
            api::ApiError::Forbidden {
                message: "Only the author or a workspace administrator can publish".into(),
            },
        ));
    }
    let app = c.app().clone();
    let dto = c
        .app()
        .db
        .write(move |tx| {
            let message = Message::find_reachable(tx.conn(), viewer.id, message_id)?;
            let current = User::find(tx.conn(), viewer.id)?;
            if current.id != message.creator_id && !current.is_administrator() {
                return Err(campfire_db::Error::RecordNotFound("Message"));
            }
            announcement::publish(tx, &message, viewer.id)?;
            dto::message_for_viewer(
                tx.conn(),
                &app,
                &Message::find(tx.conn(), message_id)?,
                viewer.id,
            )
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &dto)
}

fn follow_dto(follow: Follow) -> api::AnnouncementFollow {
    api::AnnouncementFollow {
        source_room_id: follow.source_room_id,
        target_room_id: follow.target_room_id,
        creator_id: follow.creator_id,
        created_at: dto::time(follow.created_at),
    }
}

async fn list_follows(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let viewer = require_current_user(c)?.clone();
    let result = c
        .app()
        .db
        .read(move |conn| {
            let visible =
                |room_id| Room::find_for_user(conn, viewer.id, room_id).map(|room| room.is_some());
            let mut followers = Vec::new();
            if viewer.can_administer(Some(room.creator_id), false) {
                for follow in announcement::followers(conn, room.id)? {
                    if visible(follow.target_room_id)? {
                        followers.push(follow_dto(follow));
                    }
                }
            }
            let mut following = Vec::new();
            for follow in announcement::following(conn, room.id)? {
                if visible(follow.source_room_id)? {
                    following.push(follow_dto(follow));
                }
            }
            Ok(api::AnnouncementFollows {
                followers,
                following,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &result)
}

async fn target(c: &mut Ctx, target_id: i64) -> Result<Room> {
    let viewer = require_current_user(c)?.id;
    let room = c
        .app()
        .db
        .read(move |conn| {
            Room::find_for_user(conn, viewer, target_id)?
                .ok_or(campfire_db::Error::RecordNotFound("Room"))
        })
        .await
        .map_err(db_error)?;
    ensure_can_administer(c, &room)?;
    Ok(room)
}

async fn create_follow(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, source) = set_room(c).await?;
    let input: api::CreateAnnouncementFollow = body(c).await?;
    let target = target(c, input.target_room_id).await?;
    let viewer = require_current_user(c)?.id;
    c.app()
        .db
        .write(move |tx| {
            Room::find_for_user(tx.conn(), viewer, source.id)?
                .ok_or(campfire_db::Error::RecordNotFound("Room"))?;
            managed(tx.conn(), viewer, target.id)?;
            announcement::follow(tx, source.id, target.id, viewer)
        })
        .await
        .map_err(db_error)?;
    list_follows(c).await
}

async fn delete_follow(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    // Unfollow needs target authority even if access to the source has since been revoked.
    let source_id = id(c, "room_id")?;
    let target = target(c, id(c, "target_room_id")?).await?;
    let viewer = require_current_user(c)?.id;
    c.app()
        .db
        .write(move |tx| {
            managed(tx.conn(), viewer, target.id)?;
            announcement::unfollow(tx, source_id, target.id)
        })
        .await
        .map_err(db_error)?;
    Ok(c.head(StatusCode::NO_CONTENT))
}
