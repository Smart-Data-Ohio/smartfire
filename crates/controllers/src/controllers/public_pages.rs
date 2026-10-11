//! Public pages inherit ActionController::Base, bypassing the workspace callback chain.
//!
//! `GET /api/v1/public_pages/{page}` is the SPA's copy of the same pages: the article each
//! template renders, with its links on the SPA's pages, and the `LEGAL_*` settings it read. Like
//! the HTML pages it answers anyone, signed in or not, and runs no before-action.

use crate::app::AppCtx;
use campfire_api_types::{PublicPage, PublicPageName, PublicPagePolicy};
use campfire_app::public_policy::PublicPolicy;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_retained::{
    helpers as h,
    public_pages::{self, Links, Page},
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
    let stylesheet = h::raw(format!("{}\n{}", h::auth_stylesheet_tag(), h::auth_script_tag()));
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
