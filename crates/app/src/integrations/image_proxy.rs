//! `Embeds::ImageProxy`: signed URLs, raster-only content, bounded bodies, and pinned fetches.
use campfire_richtext::uri;
use hyper::Method;
use rails_compat::{Secrets, verifiers::embed_image};

use super::net::{Network, guard, http::Body};
use super::opengraph::fetch::{self, FetchOptions};

pub const MAX_BODY_SIZE: usize = 5 * 1024 * 1024;
const MAX_REQUESTS: usize = 10;
const ALLOWED_CONTENT_TYPES: &[&str] =
    &["image/jpeg", "image/png", "image/gif", "image/webp", "image/avif", "image/bmp", "image/x-icon", "image/vnd.microsoft.icon"];

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("Embeds::ImageProxy::Denied")]
    Denied,
    #[error("Embeds::ImageProxy::UnusableResponse")]
    UnusableResponse,
    #[error("{0}")]
    Fetch(#[from] fetch::FetchError),
    #[error("{0}")]
    Guard(#[from] guard::GuardError),
    #[error("{0}")]
    Http(#[from] super::net::http::HttpError),
}

impl ProxyError {
    pub fn denied(&self) -> bool {
        matches!(self, Self::Denied | Self::Guard(_) | Self::Fetch(fetch::FetchError::Guard(_)))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Image {
    pub body: Vec<u8>,
    pub content_type: String,
}

pub fn signed_path(secrets: &Secrets, url: &str) -> String {
    campfire_routes::embed_image(embed_image::sign(secrets, url))
}

pub fn verified_url(secrets: &Secrets, signed: &str, now: jiff::Timestamp) -> Option<String> {
    embed_image::verified_url(secrets, signed, now)
}

pub async fn fetch(net: &Network, url: &str) -> Result<Image, ProxyError> {
    let url = uri::parse(url).map_err(|_| ProxyError::Denied)?;
    if !url.is_http() {
        return Err(ProxyError::Denied);
    }
    let ip = guard::resolve(net.resolver.as_ref(), url.host.as_deref().unwrap_or("")).await?;
    // ImageProxy's MAX_REDIRECTS.times is ten requests, unlike OpenGraph's
    // (max_redirects + 1).times. Share only the transport and redirect guard.
    let options = FetchOptions { max_redirects: MAX_REQUESTS - 1, deadline: None };
    let response = fetch::request(net, url, ip, Method::GET, options).await?;
    let content_type = response.content_type().ok_or(ProxyError::UnusableResponse)?;
    if response.status != 200
        || !ALLOWED_CONTENT_TYPES.contains(&content_type.as_str())
        || response.content_length()?.unwrap_or(0) > MAX_BODY_SIZE as u64
    {
        return Err(ProxyError::UnusableResponse);
    }
    match response.read_body(MAX_BODY_SIZE).await? {
        Body::Complete(body) => Ok(Image { body, content_type }),
        Body::TooLarge => Err(ProxyError::UnusableResponse),
    }
}
