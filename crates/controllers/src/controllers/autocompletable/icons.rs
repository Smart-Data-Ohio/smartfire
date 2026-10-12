//! app/controllers/autocompletable/icons_controller.rb and Icons.search.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions};
use crate::controllers::presenters::page::db_error;
use campfire_kit::{Ctx, Param, Result, StatusCode, format};
use campfire_runtime::presenters::icons::suggestions;
pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let custom = c.param("custom").is_some_and(Param::is_present);
    let query = c
        .param("q")
        .filter(|p| p.is_present())
        .or_else(|| c.param("query"))
        .map(crate::controllers::message_features::param_string)
        .unwrap_or_default();
    let icons = c
        .app()
        .db
        .read(move |conn| suggestions(conn, &query, custom))
        .await
        .map_err(db_error)?;
    c.respond_to(&[&format::JSON])?;
    Ok(c.render(
        StatusCode::OK,
        &format::JSON,
        campfire_presentation::helpers::to_rails_json(&icons),
    ))
}
