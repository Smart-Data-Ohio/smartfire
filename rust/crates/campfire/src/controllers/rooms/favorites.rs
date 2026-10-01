//! `app/controllers/rooms/favorites_controller.rb`.
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, ruby_to_i};
use crate::controllers::presenters::page::db_error;
use campfire_kit::{Ctx, Error, Result, StatusCode};

pub async fn create(c: &mut Ctx) -> Result {
    change(c, Change::Favorite).await
}
pub async fn destroy(c: &mut Ctx) -> Result {
    change(c, Change::Unfavorite).await
}
pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (mut membership, _) = concerns::set_room(c).await?;
    // Rails never coerces a position when the row isn't a favorite.
    if membership.favorited() {
        let position = match c.param("position") {
            None | Some(campfire_kit::Param::Null) => 0,
            Some(campfire_kit::Param::Str(value)) => ruby_to_i(value),
            Some(campfire_kit::Param::Number(n)) => n
                .as_i64()
                .unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64),
            _ => return Err(Error::internal(anyhow::anyhow!("undefined method 'to_i'"))),
        };
        c.app()
            .db
            .write(move |tx| membership.move_favorite_to(tx, position))
            .await
            .map_err(db_error)?;
    }
    Ok(c.head(StatusCode::OK))
}
enum Change {
    Favorite,
    Unfavorite,
}
async fn change(c: &mut Ctx, change: Change) -> Result {
    before_actions(c, Before::default()).await?;
    let (mut membership, _) = concerns::set_room(c).await?;
    c.app()
        .db
        .write(move |tx| match change {
            Change::Favorite => membership.favorite(tx),
            Change::Unfavorite => membership.unfavorite(tx),
        })
        .await
        .map_err(db_error)?;
    Ok(c.head(StatusCode::OK))
}
