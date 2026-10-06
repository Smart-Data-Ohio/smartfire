use crate::controllers::presenters::page::{self, db_error};
use crate::{
    app::AppCtx,
    concerns::{self, Before, before_actions},
};
use campfire_db::models::stage_streams::Denial;
use campfire_kit::{Ctx, Result, StatusCode, format};

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (member, room) = match concerns::set_room(c).await {
        Ok(scope) => scope,
        Err(campfire_kit::Error::NotFound) => return Ok(concerns::head(StatusCode::NOT_FOUND)),
        Err(error) => return Err(error),
    };
    let quality = c
        .param("quality")
        .and_then(|p| p.to_s())
        .unwrap_or_default();
    let room_id = room.id;
    let user_id = member.user_id;
    match c
        .app()
        .db
        .write(move |tx| campfire_db::models::stage_streams::start(tx, room_id, user_id, &quality))
        .await
        .map_err(db_error)?
    {
        Ok(stream) => c.set_header("X-Stream-Id", &stream.id.to_string()),
        Err(denial) => return deny(c, denial),
    }
    panel(c, room_id, member.id).await
}
pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (member, room) = match concerns::set_room(c).await {
        Ok(scope) => scope,
        Err(campfire_kit::Error::NotFound) => return Ok(concerns::head(StatusCode::NOT_FOUND)),
        Err(error) => return Err(error),
    };
    let requested = c
        .param("stream_id")
        .filter(|p| !p.is_blank())
        .and_then(|p| p.to_s());
    let room_id = room.id;
    let user_id = member.user_id;
    if let Err(denial) = c
        .app()
        .db
        .write(move |tx| {
            campfire_db::models::stage_streams::stop(tx, room_id, user_id, requested.as_deref())
        })
        .await
        .map_err(db_error)?
    {
        return deny(c, denial);
    }
    panel(c, room_id, member.id).await
}
fn deny(c: &mut Ctx, denial: Denial) -> Result {
    let (status, message) = match denial {
        Denial::NotFound => return Ok(concerns::head(StatusCode::NOT_FOUND)),
        Denial::Forbidden(message) => (StatusCode::FORBIDDEN, message.to_string()),
        Denial::UnknownQuality => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Unknown stream quality".into(),
        ),
        Denial::AlreadyLive(name) => (StatusCode::CONFLICT, format!("{name} is already live")),
    };
    Ok(c.render_as(status, "text/plain; charset=utf-8", message))
}
async fn panel(c: &mut Ctx, room_id: i64, member_id: i64) -> Result {
    if *c.respond_to(&[&format::TURBO_STREAM, &format::HTML])? == format::HTML {
        let url = c.url_for(&campfire_routes::room(room_id));
        return c.redirect_to(&url);
    }
    let app = c.app().clone();
    let stage = c
        .app()
        .db
        .read(move |conn| {
            crate::channels::huddle_effects::stage_model(&app, conn, room_id, member_id)
        })
        .await
        .map_err(db_error)?;
    page::bare(c, StatusCode::OK, &format::TURBO_STREAM, |_| {
        Ok(campfire_cable::turbo::action_tag(
            campfire_cable::turbo::Action::Replace,
            campfire_cable::turbo::Target::Target(&stage.dom_id("stage_panel")),
            Some(&stage.render("panel_body")),
            &[],
        ))
    })
    .await
}
