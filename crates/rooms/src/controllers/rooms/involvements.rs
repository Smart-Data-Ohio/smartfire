//! `Rooms::InvolvementsController` (reference/app/controllers/rooms/involvements_controller.rb).

use campfire_db::{Involvement, Membership, Room};
use campfire_kit::{Ctx, Error, Result, StatusCode};

use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions};
use crate::controllers::presenters::page::db_error;

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (membership, room) = concerns::set_room(c).await?;
    let involvement = involvement_param(c)?;
    change(c, &room, membership.id, involvement).await?;
    match c.respond_to(&[&campfire_kit::format::HTML, &campfire_kit::format::JSON])? {
        f if *f == campfire_kit::format::JSON => Ok(c.head(StatusCode::OK)),
        _ => c.redirect_to(&c.url_for(&campfire_routes::room_involvement(room.id))),
    }
}

/// The body of `update`, shared with `PUT /api/v1/rooms/:id/involvement`: sets the viewer's
/// involvement (marking the room read when it's now muted) and makes
/// `broadcast_visibility_changes`. The membership is read inside the write, so a message or a
/// read landing after the controller looked it up is seen. Answers the membership as it is now,
/// and whether this change cleared its unread state.
pub async fn change(
    c: &Ctx,
    room: &Room,
    membership_id: i64,
    involvement: Option<Involvement>,
) -> Result<(Membership, bool)> {
    let (membership, cleared_unread) = c
        .app()
        .db
        .write(move |tx| {
            let mut membership = Membership::find(tx.conn(), membership_id)?;
            let was_unread = membership.unread();
            membership.update_involvement(tx, involvement)?;
            if membership.involved_in(Involvement::Muted) { membership.read(tx)?; }
            let cleared_unread = was_unread && !membership.unread();
            Ok((membership, cleared_unread))
        })
        .await
        .map_err(db_error)?;

    c.app().broadcasts.involvement_change(room, &membership);
    Ok((membership, cleared_unread))
}

/// Our fork uses `params.require(:involvement)` before the enum cast.
fn involvement_param(c: &Ctx) -> Result<Option<Involvement>> {
    let param = c.params.require("involvement")?;
    param
        .as_str()
        .and_then(Involvement::from_name)
        .map(Some)
        .ok_or_else(|| Error::internal(anyhow::anyhow!("{:?} is not a valid involvement", param.to_s())))
}
