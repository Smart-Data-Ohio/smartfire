//! Rooms::CallModerationController; policy/mutations live in the database layer.
use crate::controllers::presenters::page::{self, db_error};
use crate::{
    app::AppCtx,
    concerns::{self, Before, before_actions, cast_integer},
};
use campfire_db::models::call_moderation::{Action, Denial};
use campfire_kit::{Ctx, Result, StatusCode, format};

pub async fn mute(c: &mut Ctx) -> Result {
    moderate(c, Action::Mute).await
}
pub async fn unmute(c: &mut Ctx) -> Result {
    moderate(c, Action::Unmute).await
}
pub async fn disconnect(c: &mut Ctx) -> Result {
    moderate(c, Action::Disconnect).await
}
async fn moderate(c: &mut Ctx, action: Action) -> Result {
    before_actions(c, Before::default()).await?;
    let (viewer, room) = match concerns::set_room(c).await {
        Ok(scope) => scope,
        Err(campfire_kit::Error::NotFound) => return Ok(concerns::head(StatusCode::NOT_FOUND)),
        Err(error) => return Err(error),
    };
    let room_id = room.id;
    let user_id = viewer.user_id;
    let target = c.param_str("membership_id").and_then(cast_integer);
    let config = campfire_db::models::room_delete::HuddleConfig {
        api_secret: c.app().config.huddle.api_secret.clone(),
        admin_configured: c.app().config.huddle.admin_configured(),
    };
    let result = c
        .app()
        .db
        .write(move |tx| {
            campfire_db::models::call_moderation::moderate(
                tx, room_id, user_id, target, action, &config,
            )
        })
        .await
        .map_err(db_error)?;
    match result {
        Err(Denial::TargetNotFound) => return Ok(c.head(StatusCode::NOT_FOUND)),
        Err(Denial::NotFound) => return Ok(concerns::head(StatusCode::NOT_FOUND)),
        Err(Denial::Forbidden) => return Ok(concerns::head(StatusCode::FORBIDDEN)),
        Err(Denial::AdministratorRank) => {
            return Ok(c.render_as(
                StatusCode::FORBIDDEN,
                "text/plain; charset=utf-8",
                "Only administrators can moderate an administrator",
            ));
        }
        Err(Denial::SelfTarget) => {
            return Ok(c.render_as(
                StatusCode::UNPROCESSABLE_ENTITY,
                "text/plain; charset=utf-8",
                "You cannot moderate your own call session",
            ));
        }
        Ok(()) => (),
    }
    match c.respond_to(&[&format::TURBO_STREAM, &format::HTML, &format::JSON])? {
        f if *f == format::HTML => {
            let url = c.url_for(&campfire_routes::room(room_id));
            c.redirect_to(&url)
        }
        f if *f == format::TURBO_STREAM && room.stage() => {
            let app = c.app().clone();
            let stage = c
                .app()
                .db
                .read(move |conn| {
                    crate::channels::huddle_effects::stage_model(&app, conn, room_id, viewer.id)
                })
                .await
                .map_err(db_error)?;
            page::bare(c, StatusCode::OK, &format::TURBO_STREAM, |_| {
                Ok(campfire_cable::turbo::action_tag(
                    campfire_cable::turbo::Action::Replace,
                    campfire_cable::turbo::Target::Target(&stage.dom_id("stage_roster")),
                    Some(&stage.render("roster")),
                    &[],
                ))
            })
            .await
        }
        _ => Ok(c.head(StatusCode::NO_CONTENT)),
    }
}
