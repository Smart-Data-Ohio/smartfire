//! `UnfurlLinksController` (reference/app/controllers/unfurl_links_controller.rb): the composer
//! asks for a pasted URL's OpenGraph metadata.

use campfire_kit::{Ctx, Error, Result, StatusCode};

use crate::concerns::{Before, before_actions};
use crate::net::Network;
use crate::integrations::opengraph::{self, Unfurl};

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    // A hash or array passes `require`, but `URI.parse` can't take it (`InvalidURIError`,
    // rescued), so the metadata has no title and isn't valid.
    let Some(url) = url_param(c)? else { return Ok(c.head(StatusCode::NO_CONTENT)) };
    match opengraph_json(c, &url).await? {
        // `render json: opengraph`
        Some(json) => Ok(c.render_as(StatusCode::OK, campfire_kit::response::JSON_UTF8, json)),
        None => Ok(c.head(StatusCode::NO_CONTENT)),
    }
}

/// `params.require(:url)`
fn url_param(c: &Ctx) -> Result<Option<String>> {
    Ok(c.params.require("url")?.as_str().map(str::to_string))
}

/// `Opengraph::Metadata.from_url(url)`, as JSON when `valid?` (`crate::integrations::opengraph`).
/// The network is the system's unless the request carries another one (tests).
async fn opengraph_json(c: &Ctx, url: &str) -> Result<Option<String>> {
    let net = c.current::<Network>().cloned().unwrap_or_else(Network::system);
    match opengraph::unfurl(&net, url).await.map_err(Error::internal)? {
        Unfurl::Json(json) => Ok(Some(json)),
        Unfurl::NoContent => Ok(None),
    }
}
