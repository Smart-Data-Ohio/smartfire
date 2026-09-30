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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    #[test]
    fn ws15e_signed_image_urls_match_rails_in_both_directions() {
        let secrets = Secrets::new(
            include_str!("../../../../parity/.env.reference").lines().find_map(|line| line.strip_prefix("SECRET_KEY_BASE=")).unwrap(),
        );
        let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../vectors/ws15e_embeds.json")).unwrap();
        for entry in vectors["signed"].as_array().unwrap() {
            let signed = embed_image::sign(&secrets, entry["url"].as_str().unwrap());
            // Rails verifies this exact byte string in generate.rb; Rust verifies Rails' output.
            assert_eq!(signed, entry["signed"].as_str().unwrap());
            assert_eq!(verified_url(&secrets, &signed, jiff::Timestamp::now()).as_deref(), entry["verified_url"].as_str());
            assert_eq!(verified_url(&secrets, &format!("{signed}x"), jiff::Timestamp::now()), None);
            assert_eq!(signed_path(&secrets, entry["url"].as_str().unwrap()), entry["path"].as_str().unwrap());
        }
    }

    #[tokio::test]
    async fn ws15e_rendered_embed_html_matches_rails_and_uses_the_proxy() {
        let app = crate::controllers::presenters::test_support::TestApp::boot().await.expect("build parity seed before this test");
        let secrets = app.booted.app.secrets.clone();
        let html = app
            .db()
            .read(move |conn| {
                let resolver = crate::controllers::presenters::DbResolver::new(conn, &secrets, jiff::Timestamp::now());
                let embed = campfire_richtext::attachables::OpengraphEmbed {
                    href: Some("https://example.com/page".into()),
                    url: Some("https://example.com/image.png".into()),
                    filename: Some("Title".into()),
                    description: Some("Description".into()),
                };
                Ok(campfire_richtext::attachables::render_opengraph_embed(&embed, &resolver.render_context(Some("campfire.test".into())))
                    .unwrap())
            })
            .await
            .unwrap();
        let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../vectors/ws15e_embeds.json")).unwrap();
        assert_eq!(html, vectors["embed_html"].as_str().unwrap());
        assert!(!html.contains("https://example.com/image.png"));
    }

    #[tokio::test]
    async fn ws15e_image_proxy_redirect_budget_is_ten_requests() {
        let mut routes: Vec<Route> = (0..10)
            .map(|i| Route::new("GET", "images.example.com", &format!("/loop-{i}"), 302).header("Location", &format!("/loop-{}", i + 1)))
            .collect();
        routes.extend((0..10).map(|i| {
            if i == 9 {
                Route::new("GET", "images.example.com", &format!("/ok-{i}"), 200).header("Content-Type", "image/png").body("image")
            } else {
                Route::new("GET", "images.example.com", &format!("/ok-{i}"), 302).header("Location", &format!("/ok-{}", i + 1))
            }
        }));
        let server = FakeServer::start_ws15e(routes).await;
        let resolver = Arc::new(FakeResolver::new([("images.example.com", vec!["93.184.216.34"])]));
        let dialer = Arc::new(MappingDialer {
            public: HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: server.addr,
            dialed: Mutex::new(Vec::new()),
        });
        let net = network(resolver, dialer);
        assert!(matches!(
            fetch(&net, "http://images.example.com/loop-0").await,
            Err(ProxyError::Fetch(fetch::FetchError::TooManyRedirects))
        ));
        assert_eq!(server.received().len(), 10);
        assert_eq!(fetch(&net, "http://images.example.com/ok-0").await.unwrap().body, b"image");
        assert_eq!(server.received().len(), 20);
    }
}
