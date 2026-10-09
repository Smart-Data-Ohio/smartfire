//! S3 sidebar organisation on `/api/v1` (`campfire_api_types::organize` documents each
//! endpoint): the viewer's categories (`room_categories#create/update/destroy`, and a new
//! reorder), a room's category (`rooms/categories#update`), favourites
//! (`rooms/favorites#create/destroy/update`) and notification level (`rooms/involvements#update`,
//! through its shared `change`). The `sidebar.*` twins come from the models
//! (`SidebarOrganized`) and from `Broadcasts::involvement_change`.

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::{Involvement, Membership, Room, RoomCategory, RoomType};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_rooms::controllers::rooms::involvements;
use campfire_web::concerns::{self, cast_integer};
use campfire_web::controllers::presenters::page::db_error;

use crate::dto;
use crate::endpoints::{before_actions, body, set_room};
use crate::error::{fail, validation};

endpoint!(
    /// `POST /api/v1/room_categories`
    create_category => post_category
);
endpoint!(
    /// `PATCH /api/v1/room_categories/:category_id`
    update_category => patch_category
);
endpoint!(
    /// `DELETE /api/v1/room_categories/:category_id`
    destroy_category => delete_category
);
endpoint!(
    /// `PUT /api/v1/room_categories/order`
    order_categories => put_order
);
endpoint!(
    /// `PUT /api/v1/rooms/:room_id/category`
    assign_category => put_category
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/favorite`
    favorite => post_favorite
);
endpoint!(
    /// `DELETE /api/v1/rooms/:room_id/favorite`
    unfavorite => delete_favorite
);
endpoint!(
    /// `PATCH /api/v1/rooms/:room_id/favorite`
    move_favorite => patch_favorite
);
endpoint!(
    /// `PUT /api/v1/rooms/:room_id/involvement`
    involvement => put_involvement
);

fn path_category_id(c: &Ctx) -> Result<i64> {
    c.param_str("category_id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)
}

/// The person's category `id`, read in the write that changes it.
fn owned(
    conn: &campfire_db::Connection,
    user_id: i64,
    id: i64,
) -> campfire_db::Result<RoomCategory> {
    RoomCategory::find_by_id(conn, id)?
        .filter(|row| row.user_id == user_id)
        .ok_or(campfire_db::Error::RecordNotFound("RoomCategory"))
}

async fn post_category(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let api::CreateRoomCategory { name, collapsed } = body(c).await?;
    let created = c
        .app()
        .db
        .write(move |tx| {
            let position = RoomCategory::next_position_for(tx.conn(), user_id)?;
            RoomCategory::create(tx, user_id, &name, position, collapsed.unwrap_or(false))
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::CREATED, &dto::room_category(created))
}

async fn patch_category(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let id = path_category_id(c)?;
    let api::UpdateRoomCategory { name, collapsed } = body(c).await?;
    #[cfg(feature = "test-support")]
    crate::test_hooks::before_category_write(id).await;
    // Read in the write, so a reorder or another field changed in another tab since isn't
    // written back; only the fields sent change.
    let updated = c
        .app()
        .db
        .write(move |tx| {
            let mut row = owned(tx.conn(), user_id, id)?;
            let name = name.unwrap_or_else(|| row.name.clone());
            let collapsed = collapsed.unwrap_or(row.collapsed);
            let position = row.position;
            row.update(tx, &name, position, collapsed)?;
            Ok(row)
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &dto::room_category(updated))
}

async fn delete_category(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let id = path_category_id(c)?;
    c.app()
        .db
        .write(move |tx| owned(tx.conn(), user_id, id)?.destroy(tx))
        .await
        .map_err(db_error)?;
    Ok(c.head(StatusCode::NO_CONTENT))
}

async fn put_order(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let api::ReorderRoomCategories { category_ids } = body(c).await?;
    let ordered = c
        .app()
        .db
        .write(move |tx| RoomCategory::reorder(tx, user_id, &category_ids))
        .await
        .map_err(db_error)?;
    let Some(ordered) = ordered else {
        return Err(fail(
            c,
            api::ApiError::Conflict {
                message: "Your categories changed. Refresh and try again.".into(),
            },
        ));
    };
    c.json(
        StatusCode::OK,
        &api::RoomCategoryList {
            categories: ordered.into_iter().map(dto::room_category).collect(),
        },
    )
}

/// The membership's row as it is now, hidden or not.
async fn row(c: &Ctx, room: Room, membership_id: i64) -> Result<api::SidebarRow> {
    c.app()
        .db
        .read_snapshot(move |conn| {
            let membership = Membership::find(conn, membership_id)?;
            dto::membership_row(conn, &room, &membership)
        })
        .await
        .map_err(db_error)
}

async fn put_category(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (mut membership, room) = set_room(c).await?;
    let api::AssignRoomCategory { room_category_id } = body(c).await?;
    if !matches!(room.room_type, RoomType::Open | RoomType::Closed) {
        return Err(fail(
            c,
            validation("roomCategoryId", "can only be set on a channel"),
        ));
    }
    let membership_id = membership.id;
    let user_id = membership.user_id;
    c.app()
        .db
        .write(move |tx| {
            // Checked in the write: a category deleted meanwhile is a 404, not a failed write.
            let category_id = room_category_id
                .map(|id| owned(tx.conn(), user_id, id).map(|row| row.id))
                .transpose()?;
            membership.update_category(tx, category_id)
        })
        .await
        .map_err(db_error)?;
    let row = row(c, room, membership_id).await?;
    c.json(StatusCode::OK, &row)
}

async fn post_favorite(c: &mut Ctx) -> Result {
    toggle_favorite(c, true).await
}

async fn delete_favorite(c: &mut Ctx) -> Result {
    toggle_favorite(c, false).await
}

async fn toggle_favorite(c: &mut Ctx, favorite: bool) -> Result {
    before_actions(c).await?;
    let (mut membership, room) = set_room(c).await?;
    let membership_id = membership.id;
    c.app()
        .db
        .write(move |tx| {
            if favorite {
                membership.favorite(tx)
            } else {
                membership.unfavorite(tx)
            }
        })
        .await
        .map_err(db_error)?;
    let row = row(c, room, membership_id).await?;
    c.json(StatusCode::OK, &row)
}

async fn patch_favorite(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (mut membership, _) = set_room(c).await?;
    let api::MoveFavorite { position } = body(c).await?;
    let user_id = membership.user_id;
    let rows = c
        .app()
        .db
        .write(move |tx| {
            membership.move_favorite_to(tx, position)?;
            shown_favorites(&campfire_db::Snapshot::of_write(tx)?, user_id)
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &api::FavoriteList { rows })
}

/// The favourites the person's sidebar shows, in order.
fn shown_favorites(
    conn: &campfire_db::Snapshot<'_>,
    user_id: i64,
) -> campfire_db::Result<Vec<api::SidebarRow>> {
    let mut rows = Vec::new();
    for membership in Membership::favorites_for_user(conn, user_id)? {
        let room = membership.room(conn)?;
        rows.extend(dto::sidebar_row(conn, &room, &membership)?);
    }
    Ok(rows)
}

async fn put_involvement(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (membership, room) = set_room(c).await?;
    let api::UpdateInvolvement { involvement } = body(c).await?;
    let involvement = match involvement {
        api::Involvement::Invisible => Involvement::Invisible,
        api::Involvement::Nothing => Involvement::Nothing,
        api::Involvement::Muted => Involvement::Muted,
        api::Involvement::Mentions => Involvement::Mentions,
        api::Involvement::Everything => Involvement::Everything,
    };
    #[cfg(feature = "test-support")]
    crate::test_hooks::before_involvement_write(membership.id).await;
    let (membership, cleared_unread) =
        involvements::change(c, &room, membership.id, Some(involvement)).await?;
    if cleared_unread {
        // Muting marked it read inside the write; the other tabs clear its unread state.
        campfire_app::cable::sync::room_read(&c.app().cable, membership.user_id, room.id);
    }
    c.json(StatusCode::OK, &dto::membership(&membership))
}
