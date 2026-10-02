//! Per-viewer cross-room quote frame. Same-room root/card cache integration remains WS8b-m.
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::{
    message_features as features,
    presenters::{Presenter, page},
};
use askama::Template;
use campfire_db::{Room, User, message_quote};
use campfire_kit::{Ctx, Error, Result, StatusCode, format, halt};
pub async fn show(c: &mut Ctx) -> Result {
    match render(c).await {
        Err(Error::NotFound) => halt(concerns::head(StatusCode::NOT_FOUND)),
        result => result,
    }
}
async fn render(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = features::room(c).await?;
    let viewer = require_current_user(c)?.id;
    let id = c
        .param_str("id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let app = c.app().clone();
    let card = c
        .app()
        .db
        .read(move |conn| {
            let source = message_quote::source(conn, room.id, id)?;
            if !message_quote::visible(conn, &source, viewer)? {
                return Ok(None);
            }
            let source_room = Room::find(conn, source.room_id)?;
            let p = Presenter::new(conn, &app, None);
            Ok(Some(campfire_views::message_links::Card {
                author: User::find(conn, source.creator_id)?.name,
                room_label: if source_room.direct() {
                    "a direct message".into()
                } else {
                    source_room.name.unwrap_or_default()
                },
                excerpt: campfire_views::helpers::truncate(
                    &p.plain_text_body(&source)?,
                    200,
                    "...",
                ),
                created_at: source.created_at.jiff(),
                message_path: campfire_db::message_pin::message_path(&source),
            }))
        })
        .await
        .map_err(page::db_error)?;
    // Rails resolves the reference before choosing the response template's format.
    c.respond_to(&[&format::HTML])?;
    page::bare(c, StatusCode::OK, &format::HTML, |ctx| {
        campfire_views::message_links::Frame {
            ctx,
            reference_id: id,
            card: card.as_ref(),
        }
        .render()
    })
    .await
}
