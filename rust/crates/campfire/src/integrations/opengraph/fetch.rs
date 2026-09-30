//! `Opengraph::Fetch` (reference/app/models/opengraph/fetch.rb): GET or HEAD against a pinned
//! address, following up to 10 responses (any 3xx is a redirect), each redirect target parsed,
//! required to be http(s), and resolved through the private network guard again. A document
//! must be a 200 `text/html` of at most 5MB, by `Content-Length` and by what's actually read.

use std::net::IpAddr;
use std::time::Duration;

use campfire_richtext::uri::Uri;
use hyper::Method;

use crate::integrations::net::Network;
use crate::integrations::net::guard::{self, GuardError};
use crate::integrations::net::http::{self, Body, Endpoint, HttpError, Timeouts};

pub const ALLOWED_DOCUMENT_CONTENT_TYPE: &str = "text/html";
pub const MAX_BODY_SIZE: usize = 5 * 1024 * 1024;
pub const MAX_REDIRECTS: usize = 10;

#[derive(Debug, Clone, Copy)]
pub struct FetchOptions {
    pub max_redirects: usize,
    pub deadline: Option<Duration>,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self { max_redirects: MAX_REDIRECTS, deadline: None }
    }
}

/// Explicit operation timeouts in our fork (app/models/opengraph/fetch.rb).
const TIMEOUTS: Timeouts = Timeouts { open: Duration::from_secs(5), read: Duration::from_secs(5), write: Duration::from_secs(5) };

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("Opengraph::Fetch::TooManyRedirectsError")]
    TooManyRedirects,
    #[error("Opengraph::Fetch::RedirectDeniedError")]
    RedirectDenied,
    /// `URI::InvalidURIError` (a missing or unparsable `Location`), or a URI Net::HTTP refuses.
    #[error("fetch deadline exceeded")]
    Deadline,
    #[error("bad URI")]
    InvalidUri,
    #[error("{0}")]
    Guard(#[from] GuardError),
    #[error("{0}")]
    Http(#[from] HttpError),
}

/// `fetch_document(url, ip:)`: the body, or `None` when the response isn't acceptable.
pub async fn fetch_document(net: &Network, url: &Uri, ip: IpAddr) -> Result<Option<Vec<u8>>, FetchError> {
    fetch_document_with(net, url, ip, FetchOptions::default()).await
}

pub async fn fetch_document_with(net: &Network, url: &Uri, ip: IpAddr, options: FetchOptions) -> Result<Option<Vec<u8>>, FetchError> {
    within_deadline(options.deadline, async {
        let response = request(net, url.clone(), ip, Method::GET, options).await?;
        if response.status != 200 || response.content_type().as_deref() != Some(ALLOWED_DOCUMENT_CONTENT_TYPE) {
            return Ok(None);
        }
        if response.content_length()?.unwrap_or(0) > MAX_BODY_SIZE as u64 {
            return Ok(None);
        }
        Ok(match response.read_body(MAX_BODY_SIZE).await? {
            Body::Complete(body) => Some(body),
            Body::TooLarge => None,
        })
    })
    .await
}

/// `fetch_content_type(url, ip:)`: the final response's header, whatever its status.
pub async fn fetch_content_type(net: &Network, url: &Uri, ip: IpAddr) -> Result<Option<String>, FetchError> {
    fetch_content_type_with(net, url, ip, FetchOptions::default()).await
}

pub async fn fetch_content_type_with(net: &Network, url: &Uri, ip: IpAddr, options: FetchOptions) -> Result<Option<String>, FetchError> {
    within_deadline(options.deadline, async { Ok(request(net, url.clone(), ip, Method::HEAD, options).await?.header("content-type")) })
        .await
}

async fn within_deadline<T>(
    deadline: Option<Duration>,
    fetch: impl std::future::Future<Output = Result<T, FetchError>>,
) -> Result<T, FetchError> {
    match deadline {
        Some(duration) if !duration.is_zero() => tokio::time::timeout(duration, fetch).await.map_err(|_| FetchError::Deadline)?,
        _ => fetch.await,
    }
}

pub(crate) async fn request(
    net: &Network,
    mut url: Uri,
    mut ip: IpAddr,
    method: Method,
    options: FetchOptions,
) -> Result<http::Response, FetchError> {
    let max_redirects = options.max_redirects;
    // Rails performs the original request and at most max_redirects subsequent requests.
    for _ in 0..=max_redirects {
        let response = match send(net, &url, ip, method.clone()).await {
            Err(FetchError::Http(error)) if options.deadline.is_none() && retryable(&error) => {
                // Net::HTTP retries idempotent requests once before yielding the response.
                // The Ruby block prevents retries during body reads. Armed deadlines set max_retries=0.
                send(net, &url, ip, method.clone()).await?
            }
            result => result?,
        };
        if (300..400).contains(&response.status) {
            (url, ip) = resolve_redirect(net, response.header("location"), &url).await?;
        } else {
            return Ok(response);
        }
    }
    Err(FetchError::TooManyRedirects)
}

fn retryable(error: &HttpError) -> bool {
    match error {
        HttpError::ReadTimeout | HttpError::WriteTimeout | HttpError::ConnectionClosed => true,
        HttpError::Io(error) => matches!(
            error.kind(),
            std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe
                | std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::UnexpectedEof
        ),
        _ => false,
    }
}

async fn resolve_redirect(net: &Network, location: Option<String>, base: &Uri) -> Result<(Uri, IpAddr), FetchError> {
    let url = crate::integrations::net::redirect::resolve(base, location.as_deref()).ok_or(FetchError::RedirectDenied)?;
    let ip = guard::resolve(net.resolver.as_ref(), url.host.as_deref().unwrap_or("")).await?;
    Ok((url, ip))
}

/// `Net::HTTP.start(url.host, url.port, ipaddr: ip, use_ssl: url.scheme == "https")` and
/// `http.request(request_class.new(url))`.
async fn send(net: &Network, url: &Uri, ip: IpAddr, method: Method) -> Result<http::Response, FetchError> {
    let host = url.host.clone().filter(|h| !h.is_empty()).ok_or(FetchError::InvalidUri)?;
    let https = url.scheme.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("https"));
    let port = url.port.and_then(|p| u16::try_from(p).ok()).ok_or(FetchError::InvalidUri)?;
    let endpoint = Endpoint { https, host: host.clone(), port, pinned_ip: Some(ip) };
    let request = http::Request::net_http(method, http::request_uri(url), Some(host_header(&host, port, https)), Vec::new())
        .transport(false, &endpoint);
    Ok(http::exchange(net, &endpoint, request, &TIMEOUTS).await?)
}

/// `Net::HTTPGenericRequest#initialize`: `uri.hostname`, plus the port unless it's the scheme's
/// default.
fn host_header(host: &str, port: u16, https: bool) -> String {
    let hostname = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(host);
    let default_port = if https { 443 } else { 80 };
    if port == default_port { hostname.to_string() } else { format!("{hostname}:{port}") }
}

#[cfg(test)]
#[path = "rails_fetch_tests.rs"]
mod rails_fetch_tests;
