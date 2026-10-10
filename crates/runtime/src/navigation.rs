//! Redirect retired HTML pages to their supported SPA destination.
use crate::concerns;
use campfire_kit::{Ctx, Error, Result, format};
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
    c.respond_to(&[&format::HTML, &format::TURBO_STREAM])?;
    let url = concerns::alias_location(c).await?.ok_or(Error::NotFound)?;
    concerns::keep_waiting_flash(c);
    c.redirect_to(&c.url_for(&url))
}
