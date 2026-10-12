//! Public pages inherit ActionController::Base, bypassing the workspace callback chain. Their
//! retained URLs (`/about`, `/privacy`, `/terms`) have moved to the SPA's.
//!
//! The article content and its JSON endpoint live in `campfire_api::public_pages`.

use campfire_kit::{Ctx, Result, StatusCode, format};

pub async fn about(c: &mut Ctx) -> Result {
    show(c, "/app/about")
}
pub async fn privacy(c: &mut Ctx) -> Result {
    show(c, "/app/privacy")
}
pub async fn terms(c: &mut Ctx) -> Result {
    show(c, "/app/terms")
}

/// The pages have moved for good to the SPA (`/app/about`, `/app/privacy`, `/app/terms`), which
/// draws them for anyone. A request that doesn't want a page still gets the bare 404.
fn show(c: &mut Ctx, path: &str) -> Result {
    let formats = c.formats()?;
    if !formats
        .first()
        .is_some_and(|value| **value == format::HTML || **value == format::ALL)
    {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    c.respond_to(&[&format::HTML])?;
    crate::concerns::redirect_to_signed_out_spa(c, path, StatusCode::MOVED_PERMANENTLY)
}
