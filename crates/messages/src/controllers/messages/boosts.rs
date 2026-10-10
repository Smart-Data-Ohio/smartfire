//! `Messages::BoostsController` (reference/app/controllers/messages/boosts_controller.rb). Its
//! `show`/`edit`/`update` routes have no action or template (`action_not_found`).

pub mod by_bots;

use campfire_db::{Boost, Message};
use campfire_kit::{Ctx, Error, Result, StatusCode, permit_keys};
use campfire_presentation::messages as views;

use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, cast_integer, require_current_user};
use crate::controllers::presenters::page::db_error;

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let message = set_message(c).await?;
    // params.require(:boost).permit(:content)
    let content = c.params.require("boost")?.permit(&permit_keys(&["content"])).get("content").and_then(super::string_column).unwrap_or_default();
    let (message_id, booster_id, app) = (message.id, require_current_user(c)?.id, c.app().clone());
    let result = c.app().db.write(move |tx| {
        let p = crate::controllers::presenters::Presenter::new(tx.conn(), &app, None);
        let content = views::reactions::resolve_content(&content, &p);
        let reaction = views::reactions::resolve(&content, &p).is_some();
        Boost::toggle_reaction(tx, message_id, booster_id, &content, reaction)
    }).await;
    match result {
        Ok(()) => broadcast_reactions(c, &message).await?,
        Err(campfire_db::Error::RecordInvalid(_)) => (), // Rails redirects after failed validation.
        Err(error) => return Err(db_error(error)),
    }
    let url = c.url_for(&campfire_routes::message_boosts(message.id));
    c.redirect_to(&url)
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let message = set_message(c).await?;
    let boost = set_boost(c, &message).await?;
    c.app().db.write(move |tx| boost.destroy(tx)).await.map_err(db_error)?;
    broadcast_reactions(c, &message).await?;
    // No destroy template: `head :no_content`.
    Ok(c.head(StatusCode::NO_CONTENT))
}

/// Message#broadcast_reactions_replace, shared by human, bot and agent reactions.
pub async fn broadcast_reactions(c: &Ctx, message: &Message) -> Result<()> {
    c.app().broadcasts.message_reactions_replace(message);
    Ok(())
}

/// `Current.user.reachable_messages.find(params[:message_id])`
pub async fn set_message(c: &mut Ctx) -> Result<Message> {
    let user_id = require_current_user(c)?.id;
    let Some(id) = c.param_str("message_id").and_then(cast_integer) else { return Err(Error::NotFound) };
    c.app().db.read(move |conn| Message::find_reachable(conn, user_id, id)).await.map_err(db_error)
}

/// `@message.boosts.find_by!(id: params[:id], booster: Current.user)`
pub(crate) async fn set_boost(c: &mut Ctx, message: &Message) -> Result<Boost> {
    let user_id = require_current_user(c)?.id;
    let Some(id) = c.param_str("id").and_then(cast_integer) else { return Err(Error::NotFound) };
    let message_id = message.id;
    c.app().db.read(move |conn| Boost::find_by_message_and_booster(conn, message_id, id, user_id)).await.map_err(db_error)
}

/// `@boost.destroy!` then `Message#broadcast_reactions_replace`.
pub(crate) async fn destroy_boost(c: &Ctx, message: &Message, boost: Boost) -> Result<()> {
    let destroyed = boost.clone();
    c.app().db.write(move |tx| destroyed.destroy(tx)).await.map_err(db_error)?;
    broadcast_reactions(c, message).await?;
    Ok(())
}
