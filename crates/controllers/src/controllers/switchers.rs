//! `app/controllers/switchers_controller.rb`: scoped quick-switcher JSON.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, require_current_user};
use crate::controllers::presenters::page::db_error;
use campfire_kit::{Ctx, Result, StatusCode, format};

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    c.respond_to(&[&format::JSON])?;
    let user = require_current_user(c)?.clone();
    let base_url = c.url_for("");
    let secrets = c.app().secrets.clone();
    let payload = c
        .app()
        .db
        .read(move |conn| super::presenters::switcher::load(conn, &secrets, &user, &base_url))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &payload)
}
