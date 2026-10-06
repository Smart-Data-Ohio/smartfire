//! One-attempt durable `LinkEmbed::FetchJob` and its negative-cache fetcher.
use super::{
    Embed,
    metadata_parser::{self, Metadata},
};
use crate::{
    app::App,
    integrations::{
        net::Network,
        opengraph::{fetch::FetchOptions, location::Location},
    },
};
use campfire_db::Job;
use campfire_jobs::{Execution, JobKind, JobResult, Outcome, RetryPolicy};
use campfire_richtext::uri;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchJob {
    pub embed_id: i64,
}
impl Job for FetchJob {
    const CLASS: &'static str = "LinkEmbed::FetchJob";
}
impl JobKind for FetchJob {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::no_retries()
    }
}

pub async fn perform(app: App, job: FetchJob, _: Execution) -> JobResult {
    fetch(&app, &Network::system(), job.embed_id)
        .await
        .map_err(crate::queue::discard_missing)?;
    Ok(Outcome::Done)
}

const OPTIONS: FetchOptions = FetchOptions {
    max_redirects: 3,
    deadline: Some(Duration::from_secs(10)),
};

pub async fn fetch(app: &App, net: &Network, id: i64) -> campfire_db::Result<()> {
    let embed = app.db.read(move |conn| Embed::find(conn, id)).await?;
    let result = fetch_metadata(net, &embed.normalized_url).await;
    app.db
        .write(move |tx| {
            let embed = Embed::find(tx.conn(), id)?;
            match result {
                Ok(metadata) => embed.save_metadata(tx, &metadata),
                Err(error) => embed.save_negative(tx, error),
            }
        })
        .await
}

async fn fetch_metadata(net: &Network, url: &str) -> Result<Metadata, &'static str> {
    let mut location = Location::new_with_options(net, Some(url), OPTIONS);
    if !location.is_valid().await {
        return Err(if uri::parse(url).ok().is_some_and(|url| url.is_http()) {
            "is not public"
        } else {
            "is invalid"
        });
    }
    let html = location
        .read_html()
        .await
        .filter(|html| !String::from_utf8_lossy(html).chars().all(char::is_whitespace))
        .ok_or("Could not load this link")?;
    let base = url.to_string();
    let mut metadata = tokio::task::spawn_blocking(move || metadata_parser::parse(&html, &base))
        .await
        .map_err(|_| "Could not load this link")?
        .map_err(|_| "Could not load this link")?;
    metadata.image_url = match metadata.image_url.take() {
        Some(image)
            if uri::parse(&image)
                .ok()
                .is_some_and(|url| url.scheme.as_deref().is_some_and(|scheme| scheme.eq_ignore_ascii_case("https")) && url.host.is_some()) =>
        {
            let mut location = Location::new_with_options(net, Some(&image), OPTIONS);
            let content_type = location.fetch_content_type().await;
            content_type
                .filter(|value| {
                    ["image/jpeg", "image/png", "image/gif", "image/webp", "image/avif"].contains(&value.to_lowercase().as_str())
                })
                .map(|_| image)
        }
        _ => None,
    };
    if metadata.title.is_none() && metadata.description.is_none() {
        Err("No preview available for this link")
    } else {
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests;
