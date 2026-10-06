//! Users::CardsController: authenticated lookup, rendered without an application layout.
use super::super::presenters::{page, people};
use crate::{
    app::AppCtx,
    concerns::{self, Before},
};
use askama::Template;
use campfire_db::{Timestamp, models::user::presentation};
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_views::users;

pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let viewer = concerns::require_current_user(c)?.id;
    let id = c
        .param_str("id")
        .and_then(concerns::cast_integer)
        .ok_or(Error::NotFound)?;
    let now = Timestamp::from_jiff(c.now());
    let facts = c
        .app()
        .db
        .read(move |conn| presentation::card(conn, viewer, id, now))
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)?;
    let person = people::person(&c.app().secrets, facts);
    page::bare(c, StatusCode::OK, &format::HTML, |ctx| {
        users::Card {
            ctx,
            person: person.clone(),
        }
        .render()
    })
    .await
}
