//! `Github::WebhooksController`: an API action with raw-body HMAC authentication.
use campfire_kit::{Ctx, Error, Result, StatusCode};

use crate::app::AppCtx;
use crate::integrations::github::webhooks::{self, Authentication};

pub async fn create(c: &mut Ctx) -> Result {
    let app = c.app().clone();
    if app.config.github_webhook_secret.is_none() {
        return Ok(c.head(StatusCode::SERVICE_UNAVAILABLE));
    }
    let raw = c.read_body(usize::MAX).await;
    let guid = c
        .request
        .header("X-GitHub-Delivery")
        .unwrap_or("")
        .to_owned();
    match webhooks::authenticate(
        app.config.github_webhook_secret.as_deref(),
        c.request.header("X-Hub-Signature-256"),
        &guid,
        &raw,
    ) {
        Authentication::Unavailable => return Ok(c.head(StatusCode::SERVICE_UNAVAILABLE)),
        Authentication::Unauthorized => return Ok(c.head(StatusCode::UNAUTHORIZED)),
        Authentication::Accepted => {}
    }
    let event = c.request.header("X-GitHub-Event").unwrap_or("").to_owned();
    webhooks::receive(&app.db, guid, event, raw.to_vec())
        .await
        .map_err(Error::internal)?;
    Ok(c.head(StatusCode::OK))
}

/// No Rails route matches this path with another verb.
pub async fn not_found(_c: &mut Ctx) -> Result {
    Err(Error::NotFound)
}

#[cfg(test)]
mod tests;
