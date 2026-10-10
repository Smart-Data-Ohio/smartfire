//! Redirect retired HTML pages to their supported SPA destination.
use crate::concerns;
use campfire_kit::{Ctx, Error, Result, format};

pub async fn redirect(c: &mut Ctx) -> Result {
    c.respond_to(&[&format::HTML, &format::TURBO_STREAM])?;
    let url = concerns::alias_location(c).await?.ok_or(Error::NotFound)?;
    concerns::keep_waiting_flash(c);
    c.redirect_to(&c.url_for(&url))
}
