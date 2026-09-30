//! `Webhook#deliver` (reference/app/models/webhook.rb): a bot's webhook gets the message as
//! JSON, and its answer becomes the bot's reply.
//!
//! Requests resolve through the private-network guard and connect to that pinned address.
//! Legacy timeouts create a root text reply; agent timeouts propagate to the ledger job.

use std::sync::LazyLock;
use std::time::Duration;

use campfire_richtext::uri;
use campfire_storage::filename::Filename;
use campfire_storage::{Staged, Storage};
use regex::Regex;

use crate::integrations::net::Network;
use crate::integrations::net::http::{self, Body, Endpoint, HttpError, Timeouts};

/// `Webhook::ENDPOINT_TIMEOUT`
pub const ENDPOINT_TIMEOUT: Duration = Duration::from_secs(campfire_db::models::webhook::ENDPOINT_TIMEOUT_SECONDS);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookDelivery {
    /// The webhook's status, or `None` when it timed out.
    pub status: Option<u16>,
    pub reply: WebhookReply,
}

/// What the bot says back in the message's room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookReply {
    None,
    /// `room.messages.create!(body: text, creator: bot).broadcast_create`: the text is assigned
    /// as the message's rich text body, as a bot's posted body is.
    Text(String),
    /// `room.messages.create_with_attachment!(attachment: blob, creator: bot).broadcast_create`,
    /// with the blob from [`Attachment::stage_blob`].
    Attachment(Attachment),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub data: Vec<u8>,
    /// `"attachment.#{mime_type.symbol}"`: "attachment." for unregistered types.
    pub filename: String,
    /// `mime_type.to_s`: the registered type, which for a synonym differs from what was sent.
    pub content_type: String,
}

impl Attachment {
    /// The upload half of `ActiveStorage::Blob.create_and_upload!(io:, filename:, content_type:)`,
    /// blocking: the caller saves the staged blob's row.
    pub fn stage_blob(&self, storage: &Storage) -> campfire_storage::Result<Staged> {
        storage.stage_bytes(&self.data, Filename::new(self.filename.clone()), Some(&self.content_type))
    }
}

/// Raised out of `deliver` (the job fails), as in Rails: a bad URL, a connection failure other
/// than a timeout, or a reply content type Rails can't parse.
#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    /// `URI::InvalidURIError`, or `ArgumentError` from `Net::HTTP::Post.new`.
    #[error("{0}")]
    InvalidUrl(String),
    #[error("{0}")]
    Http(#[source] HttpError),
    /// `Mime::Type::InvalidMimeType`
    #[error("{0:?} is not a valid MIME type")]
    InvalidMimeType(String),
    #[error(transparent)]
    Guard(#[from] crate::integrations::net::guard::GuardError),
}

/// An unsigned legacy delivery, with the system clock. App jobs use `deliver_signed`.
#[cfg(test)]
pub async fn deliver(net: &Network, url: &str, payload: String) -> Result<WebhookDelivery, WebhookError> {
    deliver_signed(net, url, payload, None, jiff::Timestamp::now, false).await
}

/// `Webhook#deliver`: the agent flag controls timeout propagation, not signing policy.
pub async fn deliver_signed<F: Fn() -> jiff::Timestamp + Send + Sync>(
    net: &Network, url: &str, payload: String, secret: Option<&str>, now: F, agent: bool,
) -> Result<WebhookDelivery, WebhookError> {
    match post_payload(net, url, payload, secret, now).await {
        Ok(response) => Ok(WebhookDelivery { status: Some(response.status), reply: reply(response.status, response.content_type, response.body)? }),
        Err(WebhookError::Http(HttpError::OpenTimeout | HttpError::ReadTimeout)) if !agent => {
            Ok(WebhookDelivery { status: None, reply: WebhookReply::Text(format!("Failed to respond within {} seconds", ENDPOINT_TIMEOUT.as_secs())) })
        }
        Err(error) => Err(error),
    }
}

#[derive(Debug)]
pub struct Posted {
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
    pub headers: hyper::HeaderMap,
}

/// `post(payload)` over `Net::HTTP.new(uri.host, uri.port)`: the status, content type and body.
pub async fn post_payload<F: Fn() -> jiff::Timestamp + Send + Sync>(net: &Network, url: &str, payload: String, secret: Option<&str>, now: F) -> Result<Posted, WebhookError> {
    let uri = uri::parse(url).map_err(|_| WebhookError::InvalidUrl(format!("bad URI (is not URI?): {url:?}")))?;
    if !uri.is_http() {
        return Err(WebhookError::InvalidUrl("not an HTTP URI".into()));
    }
    let host = uri.host.clone().filter(|h| !h.is_empty()).ok_or_else(|| WebhookError::InvalidUrl("no host component for URI".into()))?;
    let https = uri.scheme.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("https"));
    let port = uri.port.and_then(|p| u16::try_from(p).ok()).ok_or_else(|| WebhookError::InvalidUrl("invalid port".into()))?;
    let address = crate::integrations::net::guard::resolve_webhook(&*net.resolver, &host).await?;
    let endpoint = Endpoint { https, host: host.clone(), port, pinned_ip: Some(address) };

    let hostname = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(&host);
    let uri_host = if port == if https { 443 } else { 80 } { hostname.to_string() } else { format!("{hostname}:{port}") };
    let now = now();
    let headers = rails_compat::webhook::smartfire_headers(secret, payload.as_bytes(), now)
        .into_iter().map(|(name, value)| (name.to_string(), value)).collect();
    let mut request = http::Request::net_http(hyper::Method::POST, http::request_uri(&uri), Some(uri_host), headers).transport(false, &endpoint);
    request.body = payload.into_bytes();

    let timeouts = Timeouts { open: ENDPOINT_TIMEOUT, read: ENDPOINT_TIMEOUT, write: http::NET_HTTP_DEFAULT_TIMEOUT };
    let response = http::exchange(net, &endpoint, request, &timeouts).await.map_err(WebhookError::Http)?;
    let (status, content_type) = (response.status, response.content_type());
    let headers = response.headers.clone();
    let body = match response.read_body(usize::MAX).await.map_err(WebhookError::Http)? {
        Body::Complete(body) => body,
        Body::TooLarge => unreachable!("a Vec cannot exceed usize::MAX"),
    };
    Ok(Posted { status, content_type, body, headers })
}

/// `extract_text_from`, else `extract_attachment_from`.
pub(super) fn reply(status: u16, content_type: Option<String>, body: Vec<u8>) -> Result<WebhookReply, WebhookError> {
    if !(200..300).contains(&status) { return Ok(WebhookReply::None); }
    let Some(content_type) = content_type else { return Ok(WebhookReply::None) };
    if status == 200 && (content_type == "text/html" || content_type == "text/plain") {
        return Ok(WebhookReply::Text(String::from_utf8_lossy(&body).into_owned()));
    }
    let (symbol, registered) = mime_lookup(&content_type)?;
    Ok(WebhookReply::Attachment(Attachment {
        data: body,
        filename: format!("attachment.{}", symbol.unwrap_or("")),
        content_type: registered,
    }))
}

/// `Mime::Type.lookup(string)`: a registered type (by its string or a synonym), else a new
/// unregistered type, which must be a valid MIME type.
fn mime_lookup(string: &str) -> Result<(Option<&'static str>, String), WebhookError> {
    let registered = |s: &str| MIME_LOOKUP.iter().find(|(key, _, _)| *key == s).map(|(_, symbol, to_s)| (Some(*symbol), to_s.to_string()));
    if let Some(found) = registered(string) {
        return Ok(found);
    }
    let string = string.split(';').next().unwrap_or("").trim_end_matches([' ', '\t', '\n', '\x0b', '\x0c', '\r', '\0']);
    if let Some(found) = registered(string) {
        return Ok(found);
    }
    if MIME_REGEXP.is_match(string) {
        Ok((None, string.to_string()))
    } else {
        Err(WebhookError::InvalidMimeType(string.to_string()))
    }
}

/// `Mime::Type::MIME_REGEXP` (in the Ruby source, "\s" inside the double-quoted parameter
/// pattern is a literal space).
static MIME_REGEXP: LazyLock<Regex> = LazyLock::new(|| {
    let name = r"[a-zA-Z0-9][a-zA-Z0-9!#$&\-^_.+]{0,126}";
    let value = format!(r#"(?:{name}|"[^"\r\\]*")"#);
    let parameter = format!(r" *; *{name}(?:={value})?");
    Regex::new(&format!(r"\A(?:\*/\*|{name}/(?:\*|{name})(?:{parameter})*[ \t\n\x0B\x0C\r]*)\z")).unwrap()
});

/// `Mime::LOOKUP` in the reference app: Action Dispatch's registrations plus turbo-rails'.
const MIME_LOOKUP: &[(&str, &str, &str)] = &[
    ("text/html", "html", "text/html"),
    ("application/xhtml+xml", "html", "text/html"),
    ("text/plain", "text", "text/plain"),
    ("text/javascript", "js", "text/javascript"),
    ("application/javascript", "js", "text/javascript"),
    ("application/x-javascript", "js", "text/javascript"),
    ("text/css", "css", "text/css"),
    ("text/calendar", "ics", "text/calendar"),
    ("text/csv", "csv", "text/csv"),
    ("text/vcard", "vcf", "text/vcard"),
    ("text/vtt", "vtt", "text/vtt"),
    ("vtt", "vtt", "text/vtt"),
    ("text/markdown", "md", "text/markdown"),
    ("image/png", "png", "image/png"),
    ("image/jpeg", "jpeg", "image/jpeg"),
    ("image/gif", "gif", "image/gif"),
    ("image/bmp", "bmp", "image/bmp"),
    ("image/tiff", "tiff", "image/tiff"),
    ("image/svg+xml", "svg", "image/svg+xml"),
    ("image/webp", "webp", "image/webp"),
    ("video/mpeg", "mpeg", "video/mpeg"),
    ("audio/mpeg", "mp3", "audio/mpeg"),
    ("audio/ogg", "ogg", "audio/ogg"),
    ("audio/aac", "m4a", "audio/aac"),
    ("audio/mp4", "m4a", "audio/aac"),
    ("video/webm", "webm", "video/webm"),
    ("video/mp4", "mp4", "video/mp4"),
    ("font/otf", "otf", "font/otf"),
    ("font/ttf", "ttf", "font/ttf"),
    ("font/woff", "woff", "font/woff"),
    ("font/woff2", "woff2", "font/woff2"),
    ("application/xml", "xml", "application/xml"),
    ("text/xml", "xml", "application/xml"),
    ("application/x-xml", "xml", "application/xml"),
    ("application/rss+xml", "rss", "application/rss+xml"),
    ("application/atom+xml", "atom", "application/atom+xml"),
    ("application/x-yaml", "yaml", "application/x-yaml"),
    ("text/yaml", "yaml", "application/x-yaml"),
    ("multipart/form-data", "multipart_form", "multipart/form-data"),
    ("application/x-www-form-urlencoded", "url_encoded_form", "application/x-www-form-urlencoded"),
    ("application/json", "json", "application/json"),
    ("text/x-json", "json", "application/json"),
    ("application/jsonrequest", "json", "application/json"),
    ("application/problem+json", "json", "application/json"),
    ("application/pdf", "pdf", "application/pdf"),
    ("application/zip", "zip", "application/zip"),
    ("application/gzip", "gzip", "application/gzip"),
    ("application/x-gzip", "gzip", "application/gzip"),
    ("text/vnd.turbo-stream.html", "turbo_stream", "text/vnd.turbo-stream.html"),
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    use base64::Engine;
    use serde_json::Value;

    use super::*;
    use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};

    fn b64(s: &str) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD.decode(s).unwrap()
    }

    /// Replays testdata/webhook_cases.json and compares with what `Webhook#deliver` did in the
    /// reference (testdata/webhook_expected.json, from testdata/oracle/webhook.rb).
    #[tokio::test(flavor = "multi_thread")]
    async fn delivers_like_the_reference() {
        let cases: Vec<Value> = serde_json::from_str(include_str!("testdata/webhook_cases.json")).unwrap();
        let expected: Vec<Value> = serde_json::from_str(include_str!("testdata/webhook_expected.json")).unwrap();
        let routes = cases
            .iter()
            .map(|c| {
                let mut route = Route::new("POST", "*", &format!("/{}", c["name"].as_str().unwrap()), c["status"].as_u64().unwrap() as u16);
                route.headers = serde_json::from_value(c["headers"].clone()).unwrap();
                route.body = match c["body_b64"].as_str() {
                    Some(body) => b64(body),
                    None => c["body"].as_str().unwrap_or("").as_bytes().to_vec(),
                };
                route.gzip = c["gzip"].as_bool().unwrap_or(false);
                route.delay = Duration::from_secs(c["delay"].as_u64().unwrap_or(0));
                route
            })
            .collect();
        let server = FakeServer::start(routes).await;
        let resolver = Arc::new(FakeResolver::new([("webhook.example", vec!["93.184.216.34"])]));
        let dialer = Arc::new(MappingDialer { public: HashSet::from(["93.184.216.34".parse().unwrap()]), to: server.addr, dialed: Mutex::new(Vec::new()) });
        let net = network(resolver, dialer);

        let runs = cases.iter().map(|c| {
            let net = net.clone();
            let url = c["url"].as_str().map(str::to_string).unwrap_or_else(|| format!("http://webhook.example:{}/{}", server.addr.port(), c["name"].as_str().unwrap()));
            async move { deliver_signed(&net, &url, r#"{"message":"hi"}"#.to_string(), None, || "2026-03-02T16:00:00Z".parse().unwrap(), false).await }
        });
        let outcomes = futures_join_all(runs).await;

        for ((case, expected), outcome) in cases.iter().zip(&expected).zip(outcomes) {
            let name = case["name"].as_str().unwrap();
            let actual = match outcome {
                Ok(delivery) => {
                    let reply = match delivery.reply {
                        WebhookReply::None => Value::Null,
                        WebhookReply::Text(text) => serde_json::json!({ "text": text }),
                        WebhookReply::Attachment(a) => serde_json::json!({ "filename": a.filename, "content_type": a.content_type, "data": a.data }),
                    };
                    serde_json::json!({ "status": delivery.status, "reply": reply })
                }
                Err(WebhookError::Guard(crate::integrations::net::guard::GuardError::Violation(_))) => serde_json::json!({ "error": "RestrictedHTTP::Violation", "reply": null }),
                Err(WebhookError::InvalidMimeType(_)) => serde_json::json!({ "error": "Mime::Type::InvalidMimeType", "reply": null }),
                Err(WebhookError::Http(HttpError::Io(e))) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                    serde_json::json!({ "error": "Errno::ECONNREFUSED", "reply": null })
                }
                Err(error) => serde_json::json!({ "error": error.to_string(), "reply": null }),
            };
            let reply = &expected["reply"];
            let wanted_reply = if reply.is_null() {
                Value::Null
            } else if let Some(attachment) = reply.get("attachment") {
                serde_json::json!({
                    "filename": attachment["filename"],
                    "content_type": attachment["content_type"],
                    "data": b64(attachment["body_b64"].as_str().unwrap()),
                })
            } else {
                serde_json::json!({ "text": String::from_utf8_lossy(&b64(reply["text_b64"].as_str().unwrap())) })
            };
            let wanted = match expected.get("error") {
                Some(error) => serde_json::json!({ "error": error, "reply": wanted_reply }),
                None => serde_json::json!({ "status": expected["status"], "reply": wanted_reply }),
            };
            assert_eq!(actual, wanted, "{name}");
        }

        // The request as Net::HTTP sends it
        let request = server.received().into_iter().find(|r| r.target == "/text").unwrap();
        let wanted: Vec<(String, String)> = serde_json::from_value(expected[0]["requests"][0]["headers"].clone()).unwrap();
        let wanted: Vec<(String, String)> =
            wanted.into_iter().map(|(n, v)| if n == "Host" { (n, format!("webhook.example:{}", server.addr.port())) } else { (n, v) }).collect();
        assert_eq!(request.headers, wanted);
        assert_eq!(request.body, expected[0]["requests"][0]["body"].as_str().unwrap().as_bytes());
    }

    async fn futures_join_all<F: std::future::Future + Send + 'static>(futures: impl Iterator<Item = F>) -> Vec<F::Output>
    where
        F::Output: Send + 'static,
    {
        let handles: Vec<_> = futures.map(tokio::spawn).collect();
        let mut out = Vec::new();
        for handle in handles {
            out.push(handle.await.unwrap());
        }
        out
    }

    #[tokio::test]
    async fn ws11_blocks_private_webhooks_before_connecting() {
        let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 200)]).await;
        let resolver = Arc::new(FakeResolver::new([("bots.internal", vec!["10.0.0.7"])]));
        let dialer = Arc::new(MappingDialer { public: HashSet::from(["10.0.0.7".parse().unwrap()]), to: server.addr, dialed: Mutex::new(Vec::new()) });
        let net = network(resolver.clone(), dialer.clone());
        for host in ["bots.internal", "127.0.0.1", "10.0.0.1", "192.168.1.1", "169.254.169.254", "[::1]", "[::ffff:127.0.0.1]", "2130706433", "0177.1", "0x7f.1"] {
            let result = deliver(&net, &format!("http://{host}:8080/hook"), "{}".into()).await;
            assert!(result.is_err(), "private webhook succeeded: {host}: {result:?}");
        }
        assert!(dialer.dialed.lock().unwrap().is_empty(), "private connections were attempted");
        assert!(server.received().is_empty());
    }

    #[tokio::test]
    async fn ws11_pins_public_dns_answer_and_sends_timestamp() {
        let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 200).header("Content-Type", "text/plain").body("ok")]).await;
        let resolver = Arc::new(FakeResolver::default());
        resolver.set("bots.example", vec![vec!["93.184.216.34".parse().unwrap()], vec!["127.0.0.1".parse().unwrap()]]);
        let dialer = Arc::new(MappingDialer { public: HashSet::from(["93.184.216.34".parse().unwrap()]), to: server.addr, dialed: Mutex::new(Vec::new()) });
        let net = network(resolver.clone(), dialer.clone());
        let delivery = deliver(&net, "http://bots.example:8080/hook", "{}".into()).await.unwrap();
        assert_eq!(delivery.reply, WebhookReply::Text("ok".into()));
        assert_eq!(resolver.lookups(), ["bots.example"]);
        assert_eq!(*dialer.dialed.lock().unwrap(), ["93.184.216.34:8080".parse().unwrap()]);
        let request = &server.received()[0];
        assert_eq!(request.header("Host"), Some("bots.example:8080"));
        assert!(request.header("X-Smartfire-Timestamp").unwrap().parse::<i64>().is_ok());
        assert_eq!(request.header("X-Smartfire-Signature"), None);
    }

    #[test]
    fn ws11_error_responses_never_become_replies() {
        for status in [301, 400, 408, 429, 500, 503] {
            assert_eq!(reply(status, Some("text/html".into()), b"Error".to_vec()).unwrap(), WebhookReply::None);
        }
    }

    #[tokio::test]
    async fn ws11_signed_requests_match_rails_vectors() {
        let vectors: Value = serde_json::from_str(include_str!("../../../../vectors/agents_webhook_contract.json")).unwrap();
        let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 204)]).await;
        let resolver = Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])]));
        let dialer = Arc::new(MappingDialer { public: HashSet::from(["93.184.216.34".parse().unwrap()]), to: server.addr, dialed: Mutex::new(Vec::new()) });
        let net = network(resolver, dialer);
        for case in vectors["signatures"].as_array().unwrap() {
            post_payload(&net, "http://bots.example:8080/hook", case["body"].as_str().unwrap().into(), Some(case["secret"].as_str().unwrap()), || vectors["now"].as_str().unwrap().parse().unwrap()).await.unwrap();
            let request = server.received().pop().unwrap();
            assert_eq!(request.header("X-Smartfire-Timestamp"), case["timestamp"].as_str());
            assert_eq!(request.header("X-Smartfire-Signature"), case["signature"].as_str());
            assert_eq!(request.body, case["body"].as_str().unwrap().as_bytes());
        }
    }

    #[tokio::test]
    async fn ws11_guard_matches_rails_vectors() {
        use crate::integrations::net::guard::{resolve_webhook, GuardError};
        let vectors: Value = serde_json::from_str(include_str!("../../../../vectors/agents_webhook_contract.json")).unwrap();
        let resolver = FakeResolver::default();
        for case in vectors["guards"].as_array().unwrap().iter().chain(vectors["dns"].as_array().unwrap()) {
            let host = case["host"].as_str().unwrap();
            if let Some(answers) = case.get("answers") {
                resolver.set(host, vec![answers.as_array().unwrap().iter().map(|ip| ip.as_str().unwrap().parse().unwrap()).collect()]);
            }
            let actual = match resolve_webhook(&resolver, host).await {
                Ok(address) => serde_json::json!({"address":address.to_string()}),
                Err(GuardError::Violation(_)) => serde_json::json!({"error":"RestrictedHTTP::Violation"}),
                Err(GuardError::Unresolvable) => serde_json::json!({"error":"Surfguard::Unresolvable"}),
            };
            let wanted = if let Some(address) = case.get("address") { serde_json::json!({"address":address}) } else { serde_json::json!({"error":case["error"]}) };
            assert_eq!(actual, wanted, "{host}");
        }
    }

    #[tokio::test]
    async fn ws11_timestamp_is_sampled_after_resolution() {
        use crate::integrations::net::{BoxFuture, Resolver};
        use std::sync::atomic::{AtomicI64, Ordering};
        struct SlowResolver(Arc<AtomicI64>);
        impl Resolver for SlowResolver {
            fn lookup<'a>(&'a self, _: &'a str) -> BoxFuture<'a, std::io::Result<Vec<std::net::IpAddr>>> {
                Box::pin(async move {
                    tokio::time::sleep(Duration::from_millis(1100)).await;
                    self.0.store(jiff::Timestamp::now().as_second(), Ordering::SeqCst);
                    Ok(vec!["93.184.216.34".parse().unwrap()])
                })
            }
        }
        let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 204)]).await;
        let resolved_at = Arc::new(AtomicI64::new(0));
        let net = Network { resolver: Arc::new(SlowResolver(resolved_at.clone())), dialer: Arc::new(MappingDialer { public: HashSet::from(["93.184.216.34".parse().unwrap()]), to: server.addr, dialed: Mutex::new(Vec::new()) }), tls: crate::integrations::net::tls_config(crate::integrations::test_support::test_tls_roots()) };
        deliver(&net, "http://bots.example:8080/hook", "{}".into()).await.unwrap();
        let timestamp: i64 = server.received()[0].header("X-Smartfire-Timestamp").unwrap().parse().unwrap();
        assert!(timestamp >= resolved_at.load(Ordering::SeqCst), "timestamp was sampled before DNS resolution");
    }

    #[tokio::test]
    async fn ws11_http_guard_matches_rails_vectors() {
        let vectors: Value = serde_json::from_str(include_str!("../../../../vectors/agents_webhook_contract.json")).unwrap();
        let cases: Vec<_> = vectors["guards"].as_array().unwrap().iter().chain(vectors["dns"].as_array().unwrap()).collect();
        let server = FakeServer::start(vec![Route::new("POST", "*", "/hook", 204)]).await;
        let resolver = Arc::new(FakeResolver::default());
        let public = cases.iter().filter_map(|case| case["address"].as_str()).map(|ip| ip.parse().unwrap()).collect();
        let dialer = Arc::new(MappingDialer { public, to: server.addr, dialed: Mutex::new(Vec::new()) });
        let net = network(resolver.clone(), dialer);
        for case in cases {
            let host = case["host"].as_str().unwrap();
            if let Some(answers) = case.get("answers") {
                resolver.set(host, vec![answers.as_array().unwrap().iter().map(|ip| ip.as_str().unwrap().parse().unwrap()).collect()]);
            }
            let uri_host = if host.contains(':') && !host.starts_with('[') { format!("[{host}]") } else { host.to_string() };
            let response = deliver(&net, &format!("http://{uri_host}:8080/hook"), "{}".into()).await;
            match case["error"].as_str() {
                Some("RestrictedHTTP::Violation") => assert!(matches!(response, Err(WebhookError::Guard(crate::integrations::net::guard::GuardError::Violation(_)))), "{host}: {response:?}"),
                Some("Surfguard::Unresolvable") => assert!(matches!(response, Err(WebhookError::Guard(crate::integrations::net::guard::GuardError::Unresolvable))), "{host}: {response:?}"),
                None => assert_eq!(response.unwrap().status, Some(204), "{host}"),
                other => panic!("unknown Rails error: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn ws11_agent_timeouts_propagate_while_legacy_gets_a_root_reply() {
        let mut route = Route::new("POST", "*", "/hook", 200);
        route.delay = Duration::from_secs(8);
        let server = FakeServer::start(vec![route]).await;
        let resolver = Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])]));
        let dialer = Arc::new(MappingDialer { public: HashSet::from(["93.184.216.34".parse().unwrap()]), to: server.addr, dialed: Mutex::new(Vec::new()) });
        let net = network(resolver, dialer);
        let now = "2026-03-02T16:00:00Z".parse().unwrap();
        let (legacy, agent) = tokio::join!(deliver_signed(&net, "http://bots.example:8080/hook", "{}".into(), None, || now, false), deliver_signed(&net, "http://bots.example:8080/hook", "{}".into(), Some("test-secret"), || now, true));
        assert_eq!(legacy.unwrap(), WebhookDelivery { status: None, reply: WebhookReply::Text("Failed to respond within 7 seconds".into()) });
        assert!(matches!(agent, Err(WebhookError::Http(HttpError::ReadTimeout))));
    }

    #[test]
    fn looks_up_mime_types_like_rails() {
        assert_eq!(mime_lookup("image/jpeg").unwrap(), (Some("jpeg"), "image/jpeg".into()));
        assert_eq!(mime_lookup("application/x-gzip").unwrap(), (Some("gzip"), "application/gzip".into()));
        assert_eq!(mime_lookup("IMAGE/PNG").unwrap(), (None, "IMAGE/PNG".into()));
        assert_eq!(mime_lookup("text/html; charset=utf-8").unwrap(), (Some("html"), "text/html".into()));
        assert_eq!(mime_lookup("video/quicktime").unwrap(), (None, "video/quicktime".into()));
        for invalid in ["image", "", "text/html, text", "a/b c"] {
            assert!(mime_lookup(invalid).is_err(), "{invalid}");
        }
    }
}
