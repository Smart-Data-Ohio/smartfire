//! JSON picker for built-ins and registered-agent metadata; execution uses WS11's domain dispatcher.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions};
use crate::controllers::{message_features as features, presenters::page::db_error};
use campfire_kit::{Ctx, Error, Result, StatusCode};
pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    c.start_action();
    // The Ruby method rescues locally, so this normal return still runs after-actions.
    match commands(c).await {
        Err(Error::NotFound) => Ok(c.head(StatusCode::NOT_FOUND)),
        result => result,
    }
}
async fn commands(c: &mut Ctx) -> Result {
    let room = features::room(c).await?;
    let thread = super::super::rooms::slash_commands::thread_id(c, &room).await?;
    let query = c
        .param("query")
        .filter(|p| p.is_present())
        .map(crate::controllers::message_features::param_string);
    let commands = c
        .app()
        .db
        .read(move |conn| {
            campfire_db::command_suggestions::for_room(
                conn,
                room.id,
                thread.is_some(),
                query.as_deref(),
            )
        })
        .await
        .map_err(db_error)?;
    // Rails' explicit render json ignores Accept/extension negotiation here.
    c.json(StatusCode::OK, &commands)
}
