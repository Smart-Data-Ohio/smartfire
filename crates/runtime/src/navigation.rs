//! Redirect retired HTML pages to their supported SPA destination.
use crate::concerns;
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode, format};
use regex::Regex;
use std::sync::LazyLock;

/// Journey's route path: squeeze slashes, drop trailing slashes and uppercase percent escapes.
pub fn normalize_path(path: &str) -> String {
    let mut normalized = String::with_capacity(path.len() + 1);
    for c in format!("/{path}").chars() {
        if c == '/' && normalized.ends_with('/') {
            continue;
        }
        normalized.push(c);
    }
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }
    static ESCAPE: LazyLock<Regex> = LazyLock::new(|| Regex::new("%[a-fA-F0-9]{2}").unwrap());
    ESCAPE
        .replace_all(&normalized, |m: &regex::Captures| m[0].to_uppercase())
        .into_owned()
}

pub async fn redirect(c: &mut Ctx) -> Result {
    c.respond_to(&[&format::HTML])?;
    let url = concerns::alias_location(c).await?.ok_or(Error::NotFound)?;
    concerns::keep_waiting_flash(c);
    redirect_alias(c, &url)
}

/// Permanent for browser navigation; fetches and frames retain their temporary redirect.
pub fn redirect_alias(c: &mut Ctx, path: &str) -> Result {
    let status = if alias_navigation(c)? {
        StatusCode::MOVED_PERMANENTLY
    } else {
        StatusCode::FOUND
    };
    c.redirect_to_with(&c.url_for(path), Redirect { status: Some(status), ..Default::default() })
}

pub fn alias_navigation(c: &mut Ctx) -> Result<bool> {
    Ok(concerns::authenticated_by(c) == concerns::AuthenticatedBy::Session
        && c.request.header("turbo-frame").is_none()
        && concerns::coexistence_navigation(c)?)
}

/// Typed room bookmarks use the generic room's translation and record-access checks.
pub async fn room_alias(c: &Ctx, room_id: i64) -> Result<String> {
    let path = campfire_routes::room(room_id);
    let query = Some(c.request.query_string()).filter(|query| !query.is_empty());
    let confirmed = concerns::confirmed_room_query(c, &path, query).await?;
    campfire_spa::screens::spa_url_confirmed("rooms#show", &path, query, confirmed)
        .ok_or(Error::NotFound)
}
