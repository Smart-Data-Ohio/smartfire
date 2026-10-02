//! Agents::DirectoryController: a signed-in human-only directory.
use crate::{
    app::AppCtx,
    concerns::{self, Before, require_current_user},
    controllers::presenters::{self, page::framed_page},
};
use campfire_kit::{Ctx, Error, Result, StatusCode, format};

pub async fn index(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    if require_current_user(c)?.is_bot() {
        return Ok(c.head(StatusCode::FORBIDDEN));
    }
    c.respond_to(&[&format::HTML])?;
    let secrets = c.app().secrets.clone();
    let agents = c
        .app()
        .db
        .read(move |conn| presenters::agents::directory(conn, &secrets))
        .await
        .map_err(Error::internal)?;
    let now = c.app().clock.now();
    framed_page!(c, StatusCode::OK, |ctx| campfire_views::agents::Directory {
        ctx,
        agents: agents.clone(),
        now
    })
    .await
}
