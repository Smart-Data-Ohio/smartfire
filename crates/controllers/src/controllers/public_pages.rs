//! Public pages inherit ActionController::Base, bypassing the workspace callback chain.

use crate::app::AppCtx;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_views::{
    helpers as h,
    public_pages::{self, Page},
};

pub async fn about(c: &mut Ctx) -> Result {
    show(c, Page::About)
}
pub async fn privacy(c: &mut Ctx) -> Result {
    show(c, Page::Privacy)
}
pub async fn terms(c: &mut Ctx) -> Result {
    show(c, Page::Terms)
}

fn show(c: &mut Ctx, page: Page) -> Result {
    let formats = c.formats()?;
    if !formats
        .first()
        .is_some_and(|value| **value == format::HTML || **value == format::ALL)
    {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    c.respond_to(&[&format::HTML])?;
    let policy = &c.app().config.public_policy;
    let stylesheet =
        h::raw(campfire_assets::stylesheet_link_tag(&["public"], &[("media", "all")]).html);
    let body = public_pages::render(
        page,
        &policy.operator_name,
        &policy.contact_email,
        &policy.effective_date,
        stylesheet,
    )
    .map_err(Error::internal)?;
    Ok(c.render_html(StatusCode::OK, body))
}
