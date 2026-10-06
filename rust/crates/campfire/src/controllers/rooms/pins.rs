//! `app/controllers/rooms/pins_controller.rb`.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions};
use crate::controllers::message_features;
use crate::controllers::presenters::page;
use askama::Template;
use campfire_kit::{Ctx, Result, StatusCode, format};

pub(crate) use crate::controllers::presenters::pins::list;

pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = message_features::room(c).await?;
    c.start_action();
    c.respond_to(&[&format::HTML])?;
    let app = c.app().clone();
    let list = c
        .app()
        .db
        .read(move |conn| list(conn, &app, &room))
        .await
        .map_err(page::db_error)?;
    page::content(c, StatusCode::OK, |ctx| {
        campfire_views::pins::Index { ctx, list: &list }.render()
    })
    .await
}
