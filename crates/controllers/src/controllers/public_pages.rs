//! Public pages inherit ActionController::Base, bypassing the workspace callback chain. Their
//! retained URLs (`/about`, `/privacy`, `/terms`) have moved to the SPA's.
//!
//! `GET /api/v1/public_pages/{page}` is the SPA's copy of the same pages: the article each
//! template renders, with its links on the SPA's pages, and the `LEGAL_*` settings it read. Like
//! the HTML pages it answers anyone, signed in or not, and runs no before-action.

use crate::app::AppCtx;
use campfire_api_types::{PublicPage, PublicPageName, PublicPagePolicy};
use campfire_app::public_policy::PublicPolicy;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_retained::public_pages::{self, Links, Page};

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

/// `GET /api/v1/public_pages/{page}`.
pub async fn show_json(c: &mut Ctx) -> Result {
    let page = match c.param_str("page").unwrap_or_default() {
        "about" => PublicPageName::About,
        "privacy" => PublicPageName::Privacy,
        "terms" => PublicPageName::Terms,
        _ => return Ok(c.head(StatusCode::NOT_FOUND)),
    };
    let body = public_page(page, &c.app().config.public_policy).map_err(Error::internal)?;
    let body = serde_json::to_string(&body).map_err(Error::internal)?;
    Ok(c.render_as(StatusCode::OK, "application/json; charset=utf-8", body))
}

/// The SPA's links: its own copies of these pages and its sign-in page.
pub fn spa_links() -> Links {
    Links {
        privacy: "/app/privacy".into(),
        terms: "/app/terms".into(),
        sign_in: "/app/session/new".into(),
    }
}

pub fn public_page(name: PublicPageName, policy: &PublicPolicy) -> askama::Result<PublicPage> {
    let page = match name {
        PublicPageName::About => Page::About,
        PublicPageName::Privacy => Page::Privacy,
        PublicPageName::Terms => Page::Terms,
    };
    Ok(PublicPage {
        page: name,
        title: page.title().into(),
        description: page.description().into(),
        policy: PublicPagePolicy {
            operator_name: policy.operator_name.clone(),
            contact_email: policy.contact_email.clone(),
            effective_date: policy.effective_date.clone(),
        },
        html: public_pages::render_article(
            page,
            &policy.operator_name,
            &policy.contact_email,
            &policy.effective_date,
            &spa_links(),
        )?,
    })
}
