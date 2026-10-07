//! `Rooms::InvolvementsController` (reference/app/controllers/rooms/involvements_controller.rb).

use askama::Template;
use campfire_db::{Account, Involvement, Membership, Room};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_views::rooms::{InvolvementShow, InvolvementView};

use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions};
use crate::controllers::presenters::page::{self, db_error};
use crate::controllers::presenters::{Presenter, room_kind};
use crate::controllers::rooms::render_membership_sidebar;

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (membership, room) = concerns::set_room(c).await?;
    let involvement = InvolvementView {
        room_id: room.id,
        kind: room_kind(room.room_type),
        involvement: membership.involvement.map(|i| i.name().to_string()).unwrap_or_default(),
    };
    page::content(c, StatusCode::OK, |ctx| {
        InvolvementShow {
            ctx,
            involvement: &involvement,
        }
        .render()
    })
    .await
}

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
    let (membership, previous, cleared_unread) = c
        .app()
        .db
        .write(move |tx| {
            let mut membership = Membership::find(tx.conn(), membership_id)?;
            let previous = membership.involvement;
            let was_unread = membership.unread();
            membership.update_involvement(tx, involvement)?;
            if membership.involved_in(Involvement::Muted) { membership.read(tx)?; }
            let cleared_unread = was_unread && !membership.unread();
            Ok((membership, previous, cleared_unread))
        })
        .await
        .map_err(db_error)?;

    // broadcast_visibility_changes
    let partials = if room.direct() {
        let app = c.app().clone();
        let base_url = page::renderer_base_url(c);
        let membership = membership.clone();
        c.app()
            .db
            .read(move |conn| {
                let direct = Presenter::new(conn, &app, None).sidebar_direct(&membership)?;
                let account = Account::first(conn)?;
                let html = page::render_detached_at(&app, account.as_ref(), &base_url, |ctx| {
                    campfire_views::users::direct_room(ctx, &direct)
                });
                Ok(page::Rendered {
                    direct_rooms: vec![(membership.id, html)],
                    ..page::Rendered::default()
                })
            })
            .await
            .map_err(db_error)?
    } else {
        render_membership_sidebar(c, room, &membership, if previous == Some(Involvement::Invisible) { None } else { Some(membership.unread()) }).await?
    };
    c.app().broadcasts.involvement_change(room, &membership, previous, &partials);
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
