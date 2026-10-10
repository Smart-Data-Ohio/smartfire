//! `app/controllers/searches_controller.rb`.
pub use crate::controllers::presenters::params::display_query;
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, require_current_user};
use crate::controllers::presenters::page::db_error;
use campfire_presentation::searches::search_path;
use campfire_db::{Search, search_query::SearchQuery};
use campfire_kit::{Ctx, Result};

fn parsed(c: &Ctx) -> SearchQuery {
    SearchQuery::parse(&c.param("q").map(super::message_features::param_string).unwrap_or_default())
}
pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    if parsed(c).blank_query() {
        return super::message_features::redirect(
            c,
            &campfire_routes::searches(),
            Some("Enter a word to search for.".into()),
            None,
            true,
            None,
        );
    }
    let query = display_query(c).unwrap_or_default();
    let recorded = query.clone();
    let user_id = require_current_user(c)?.id;
    c.app()
        .db
        .write(move |tx| Search::record(tx, user_id, &recorded).map(|_| ()))
        .await
        .map_err(db_error)?;
    let url = c.url_for(&search_path(&query));
    c.redirect_to(&url)
}
pub async fn clear(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    c.respond_to(&[&campfire_kit::format::HTML, &campfire_kit::format::TURBO_STREAM])?;
    let user_id = require_current_user(c)?.id;
    c.app()
        .db
        .write(move |tx| Search::destroy_all_for_user(tx, user_id))
        .await
        .map_err(db_error)?;
    super::message_features::redirect(c, &campfire_routes::searches(), None, None, true, None)
}
// The existing root presenter/partial remains owned by WS8b-m. Supply search's
// show_room_icon local only on a cache miss; a shared fragment hit stays unchanged.
