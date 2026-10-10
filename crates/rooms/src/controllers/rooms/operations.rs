//! Format-independent writes shared by the classic room forms and the JSON API.
use crate::app::AppCtx;
use crate::controllers::presenters::page::db_error;
use campfire_db::{Room, RoomType, User};
use campfire_kit::{Ctx, Result};

pub async fn create(
    c: &Ctx,
    kind: RoomType,
    name: Option<String>,
    icon: Option<String>,
    creator: i64,
    grantee_ids: Vec<i64>,
) -> campfire_db::Result<Room> {
    c.app()
        .db
        .write(move |tx| {
            let grantees = if kind == RoomType::Open {
                vec![creator]
            } else {
                super::existing_user_ids(tx.conn(), &grantee_ids)?
            };
            Room::create_for_with_icon(
                tx,
                kind,
                name.as_deref(),
                icon.as_deref(),
                creator,
                &grantees,
                crate::rich_text::room_icon_resolves,
            )
        })
        .await
}

pub async fn update(
    c: &Ctx,
    room: Room,
    name: Option<Option<String>>,
    icon: Option<Option<String>>,
    kind: Option<RoomType>,
) -> campfire_db::Result<Room> {
    c.app()
        .db
        .write(move |tx| {
            let mut room = room;
            room.update_with_icon(
                tx,
                name.as_ref().map(|n| n.as_deref()),
                kind,
                icon.as_ref().map(|i| i.as_deref()),
                crate::rich_text::room_icon_resolves,
            )?;
            Ok(room)
        })
        .await
}

pub async fn revise_members(c: &Ctx, room: &Room, grantee_ids: Vec<i64>) -> Result<()> {
    let revised = room.clone();
    let changes = c
        .app()
        .db
        .write(move |tx| {
            let before = revised.user_ids(tx.conn())?;
            let granted = super::existing_user_ids(tx.conn(), &grantee_ids)?;
            let revoked: Vec<i64> = before
                .iter()
                .copied()
                .filter(|id| !grantee_ids.contains(id))
                .collect();
            revised.revise(tx, &granted, &revoked)?;
            let after = revised.user_ids(tx.conn())?;
            let granted: Vec<_> = after
                .into_iter()
                .filter(|id| !before.contains(id))
                .collect();
            if granted.is_empty() && revoked.is_empty() {
                return Ok(None);
            }
            let users = User::where_ids(
                tx.conn(),
                &granted.iter().chain(&revoked).copied().collect::<Vec<_>>(),
            )?;
            let names = |ids: &[i64]| {
                ids.iter()
                    .filter_map(|id| {
                        users
                            .iter()
                            .find(|user| user.id == *id)
                            .map(|user| user.display_name().to_owned())
                    })
                    .collect::<Vec<_>>()
            };
            Ok(Some(
                serde_json::json!({"granted":names(&granted),"revoked":names(&revoked)}),
            ))
        })
        .await
        .map_err(db_error)?;
    if let Some(changes) = changes {
        super::audit_room(c, room, "room.membership.change", changes).await?;
    }
    Ok(())
}
