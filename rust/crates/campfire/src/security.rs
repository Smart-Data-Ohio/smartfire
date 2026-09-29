//! The reference's security configuration: the Content Security Policy and its nonce
//! (`config/initializers/content_security_policy.rb`), the default headers
//! (`config/initializers/security_headers.rb`), parameter filtering
//! (`config/initializers/filter_parameter_logging.rb`, plus the attributes Active Record
//! Encryption adds) and the request log's bot-key scrubbing (`lib/rails_ext/log_scrubbing_formatter.rb`).

use std::borrow::Cow;
use std::io::Write;
use std::sync::{Arc, LazyLock};

use axum::http::{HeaderName, HeaderValue};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use campfire_kit::csp::{ContentSecurityPolicy, Source};
use campfire_kit::param_filter::ParameterFilter;
use regex::Regex;
use sha2::{Digest, Sha256};

// --- Headers -------------------------------------------------------------------------------------

/// Huddles need the camera, microphone and screen sharing (`notifications` isn't a directive;
/// browsers ignore it).
pub const PERMISSIONS_POLICY: &str = "camera=(self), display-capture=(self), microphone=(self), notifications=(self)";

/// `action_dispatch.default_headers` with `security_headers.rb`'s `merge!`: the Rails 7.1 set, and
/// `Permissions-Policy` last.
pub fn default_headers() -> Vec<(HeaderName, HeaderValue)> {
    let mut headers = campfire_kit::app::rails_default_headers();
    headers.push((HeaderName::from_static("permissions-policy"), HeaderValue::from_static(PERMISSIONS_POLICY)));
    headers
}

// --- Content Security Policy ---------------------------------------------------------------------

const GOOGLE_SCRIPTS: &[&str] = &["https://accounts.google.com/gsi/", "https://apis.google.com"];
const GOOGLE_CONNECT: &[&str] = &["https://accounts.google.com", "https://www.googleapis.com", "https://content.googleapis.com"];
const GOOGLE_FRAMES: &[&str] = &["https://docs.google.com", "https://drive.google.com", "https://accounts.google.com"];
const LINKEDIN_FRAMES: &[&str] = &["https://www.linkedin.com"];
const GOOGLE_STYLES: &[&str] = &["https://accounts.google.com/gsi/style"];
const GOOGLE_FORMS: &[&str] = &["https://accounts.google.com"];
const GITHUB_FORMS: &[&str] = &["https://github.com"];
const GOOGLE_IMAGES: &[&str] = &["https://*.googleusercontent.com"];
const GOOGLE_FONTS: &[&str] = &["https://fonts.gstatic.com"];

/// `config.content_security_policy`, enforced (`report_only = false`), with the per-session nonce
/// on `script-src`. `livekit_url` is `LIVEKIT_URL`, which Rails reads for each response.
pub fn content_security_policy(livekit_url: Option<String>) -> ContentSecurityPolicy {
    let sources = |keywords: &[&str], hosts: &[&[&str]]| -> Vec<Source> {
        keywords.iter().chain(hosts.iter().flat_map(|hosts| hosts.iter())).map(|source| Source::from(*source)).collect()
    };
    let mut connect = sources(&["'self'"], &[GOOGLE_CONNECT]);
    connect.push(Source::Dynamic(Arc::new(move || livekit_sources(livekit_url.as_deref()))));
    ContentSecurityPolicy::new()
        .directive("default-src", sources(&["'self'"], &[]))
        .directive("base-uri", sources(&["'self'"], &[]))
        .directive("object-src", sources(&["'none'"], &[]))
        .directive("script-src", sources(&["'self'", "'wasm-unsafe-eval'"], &[GOOGLE_SCRIPTS]))
        .directive("style-src", sources(&["'self'", "'unsafe-inline'"], &[GOOGLE_STYLES]))
        .directive("img-src", sources(&["'self'", "data:", "blob:", "https:"], &[GOOGLE_IMAGES]))
        .directive("font-src", sources(&["'self'", "data:"], &[GOOGLE_FONTS]))
        .directive("media-src", sources(&["'self'", "data:", "blob:"], &[]))
        .directive("connect-src", connect)
        .directive("frame-src", sources(&["'self'"], &[GOOGLE_FRAMES, LINKEDIN_FRAMES]))
        .directive("worker-src", sources(&["'self'", "blob:"], &[]))
        .directive("manifest-src", sources(&["'self'"], &[]))
        .directive("form-action", sources(&["'self'"], &[GOOGLE_FORMS, GITHUB_FORMS]))
        .directive("report-uri", sources(&["/csp_reports"], &[]))
        .nonce(Arc::new(csp_nonce), &["script-src"])
        .report_only(false)
}

/// `content_security_policy_nonce_generator`: one nonce per session (Turbo Drive keeps the first
/// page's policy in force), the session id hashed so it never shows in the page, and a random one
/// while the session has no id yet.
pub fn csp_nonce(session_id: Option<&str>) -> String {
    match session_id.filter(|id| !id.is_empty()) {
        Some(id) => STANDARD.encode(Sha256::digest(format!("csp-nonce:{id}"))),
        None => STANDARD.encode(rand::random::<[u8; 16]>()),
    }
}

/// `ContentSecurityPolicySources.livekit`: the LiveKit origin in its WebSocket and HTTP forms, or
/// nothing when `LIVEKIT_URL` is unset or not an absolute ws/wss/http/https URL.
pub fn livekit_sources(url: Option<&str>) -> Vec<String> {
    let Some(uri) = ruby_uri::parse(url.unwrap_or("")) else { return vec![] };
    let (Some(scheme), Some(host)) = (uri.scheme.as_deref(), uri.host.as_deref().filter(|host| !host.is_empty())) else {
        return vec![];
    };
    let pair = match scheme {
        "wss" => "https",
        "ws" => "http",
        "https" => "wss",
        "http" => "ws",
        _ => return vec![],
    };
    let port = uri.non_default_port().map(|port| format!(":{port}")).unwrap_or_default();
    [scheme, pair].iter().map(|scheme| format!("{scheme}://{host}{port}")).collect()
}

// --- CSP reports -----------------------------------------------------------------------------------

/// `ContentSecurityPolicyReportsController#describe`: the directive, the blocked and source
/// origins (or keywords), the document's path and the line, never a query string.
pub fn describe_csp_violation(report: &serde_json::Map<String, serde_json::Value>) -> String {
    let field = |names: &[&str]| names.iter().find_map(|name| report.get(*name).filter(|value| !value.is_null()));
    let directive = field(&["effective-directive", "effectiveDirective", "violated-directive"]);
    let blocked = field(&["blocked-uri", "blockedURL"]);
    let document = field(&["document-uri", "documentURL"]);
    let source = field(&["source-file", "sourceFile"]);
    let line = field(&["line-number", "lineNumber"]);

    let line = line.map(ruby_to_s).filter(|line| (1..=7).contains(&line.len()) && line.bytes().all(|b| b.is_ascii_digit()));
    [
        ("directive", directive.and_then(|value| clean(&ruby_to_s(value), 60))),
        ("blocked", blocked.and_then(|value| origin_or_keyword(&ruby_to_s(value)))),
        ("document", document.and_then(|value| path_only(&ruby_to_s(value)))),
        ("source", source.and_then(|value| origin_or_keyword(&ruby_to_s(value)))),
        ("line", line),
    ]
    .into_iter()
    .filter_map(|(key, value)| value.map(|value| format!("{key}={value}")))
    .collect::<Vec<_>>()
    .join(" ")
}

/// `value.to_s` for a JSON value.
fn ruby_to_s(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// URLs shrink to their origin; keywords (`inline`, `eval`) stay, cleaned.
fn origin_or_keyword(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        return None;
    }
    match ruby_uri::parse(value) {
        Some(uri) if uri.host.is_some() => {
            let port = uri.non_default_port().map(|port| format!(":{port}")).unwrap_or_default();
            Some(format!("{}://{}{port}", uri.scheme.unwrap_or_default(), uri.host.unwrap_or_default()))
        }
        _ => clean(value, 40),
    }
}

fn path_only(value: &str) -> Option<String> {
    let uri = ruby_uri::parse(value)?;
    clean(&uri.path, 200).filter(|_| !uri.path.is_empty())
}

/// `value.gsub(/[^\w\-.:\/@' ]/, "").truncate(limit).presence`
fn clean(value: &str, limit: usize) -> Option<String> {
    let kept: String = value.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '/' | '@' | '\'' | ' ')).collect();
    let truncated = if kept.chars().count() > limit {
        let mut out: String = kept.chars().take(limit - 3).collect();
        out.push_str("...");
        out
    } else {
        kept
    };
    Some(truncated).filter(|value| !value.trim().is_empty())
}

// --- Parameter filtering -------------------------------------------------------------------------

/// `config.filter_parameters` (`filter_parameter_logging.rb`).
const FILTER_PARAMETERS: &[&str] = &[
    "passw", "email", "secret", "token", "_key", "crypt", "salt", "certificate", "otp", "ssn", "cvv", "cvc", "endpoint",
    "message.body", "code", "reauth",
];

/// Each model's `encrypts` attributes, in load order: Active Record Encryption adds
/// `model.attribute` for each, and with the first the app's filters under the model's name.
const ENCRYPTED_ATTRIBUTES: &[(&str, &[&str])] = &[
    ("agent", &["webhook_signing_secret"]),
    ("fizzy_connected_account", &["access_token"]),
    ("github_connected_account", &["access_token", "refresh_token"]),
    ("google_account", &["refresh_token", "access_token"]),
    ("slack_connection", &["access_token"]),
    ("slack_workspace", &["client_secret"]),
    ("two_factor_credential", &["secret"]),
    ("two_factor_setup_secret", &["secret"]),
    ("webhook", &["signing_secret"]),
    ("encrypted_rich_text", &["body"]),
];

/// `Rails.application.config.filter_parameters` as the app has it after boot.
pub fn filter_parameters() -> Vec<String> {
    let mut filters: Vec<String> = FILTER_PARAMETERS.iter().map(|filter| filter.to_string()).collect();
    for (model, attributes) in ENCRYPTED_ATTRIBUTES {
        let (first, rest) = attributes.split_first().expect("each model encrypts something");
        let names = std::iter::once(*first).chain(FILTER_PARAMETERS.iter().copied()).chain(rest.iter().copied());
        for name in names {
            let filter = format!("{model}.{name}");
            if !filters.contains(&filter) {
                filters.push(filter);
            }
        }
    }
    filters
}

pub fn parameter_filter() -> ParameterFilter {
    ParameterFilter::new(&filter_parameters())
}

// --- Log scrubbing -------------------------------------------------------------------------------

/// `LogScrubbingFormatter::BOT_KEY_IN_PATH`: a bot key (`<id>-<token>`) or a signed reply token
/// (`...--<hex>`) as the segment after `/rooms/:id/`.
static BOT_KEY_IN_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(/rooms/\d+/)(?:\d+-[A-Za-z0-9]+|[^/\s"<>]*--[0-9a-f]+)"#).unwrap());

/// `LogScrubbingFormatter#scrub`
pub fn scrub_log_line(line: &str) -> Cow<'_, str> {
    BOT_KEY_IN_PATH.replace_all(line, "${1}[FILTERED]")
}

/// `LogScrubbingFormatter` for the tracing subscriber: every formatted event goes through
/// [`scrub_log_line`] on its way out. (`tracing_subscriber::fmt` writes each event with one
/// `write_all`, so an event never arrives split.)
#[derive(Clone, Copy, Default)]
pub struct ScrubbingStdout;

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for ScrubbingStdout {
    type Writer = ScrubbingWriter<std::io::Stdout>;

    fn make_writer(&'a self) -> Self::Writer {
        ScrubbingWriter(std::io::stdout())
    }
}

pub struct ScrubbingWriter<W: Write>(pub W);

impl<W: Write> Write for ScrubbingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(text) => self.0.write_all(scrub_log_line(text).as_bytes())?,
            Err(_) => self.0.write_all(buf)?,
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

// --- Ruby's URI.parse ----------------------------------------------------------------------------

/// Just enough of Ruby's `URI.parse` (RFC 3986) for the two callers above: the scheme
/// (lowercased), the host as written (brackets kept for IPv6), the port and the path. `None` is
/// `URI::InvalidURIError`.
pub(crate) mod ruby_uri {
    pub struct Uri {
        pub scheme: Option<String>,
        pub host: Option<String>,
        pub port: Option<u16>,
        pub path: String,
    }

    impl Uri {
        /// `uri.port if uri.port != uri.default_port`
        pub fn non_default_port(&self) -> Option<u16> {
            let default = match self.scheme.as_deref() {
                Some("http" | "ws") => Some(80),
                Some("https" | "wss") => Some(443),
                Some("ftp") => Some(21),
                Some("ldap") => Some(389),
                Some("ldaps") => Some(636),
                _ => None,
            };
            self.port.filter(|port| Some(*port) != default)
        }
    }

    fn allowed(c: char) -> bool {
        c.is_ascii_alphanumeric() || "-._~:/?#[]@!$&'()*+,;=%".contains(c)
    }

    pub fn parse(value: &str) -> Option<Uri> {
        if !value.chars().all(allowed) {
            return None;
        }
        let (scheme, rest) = match value.find(':') {
            Some(i) if i > 0 && is_scheme(&value[..i]) && !value[..i].contains(['/', '?', '#']) => {
                (Some(value[..i].to_ascii_lowercase()), &value[i + 1..])
            }
            _ => (None, value),
        };
        let rest = rest.split(['?', '#']).next().unwrap_or("");
        let Some(hierarchical) = rest.strip_prefix("//") else {
            return Some(Uri { scheme, host: None, port: None, path: rest.to_string() });
        };
        let (authority, path) = match hierarchical.find('/') {
            Some(i) => (&hierarchical[..i], &hierarchical[i..]),
            None => (hierarchical, ""),
        };
        let host_port = authority.rsplit_once('@').map_or(authority, |(_, host_port)| host_port);
        let (host, port) = if host_port.starts_with('[') {
            let end = host_port.find(']')?;
            let port = &host_port[end + 1..];
            (&host_port[..=end], port.strip_prefix(':'))
        } else {
            match host_port.split_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (host_port, None),
            }
        };
        let port = match port {
            Some("") | None => None,
            Some(port) if port.bytes().all(|b| b.is_ascii_digit()) => Some(port.parse().ok()?),
            Some(_) => return None,
        };
        Some(Uri { scheme, host: Some(host.to_string()), port, path: path.to_string() })
    }

    fn is_scheme(s: &str) -> bool {
        let mut chars = s.chars();
        chars.next().is_some_and(|c| c.is_ascii_alphabetic()) && chars.all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn vectors() -> Value {
        serde_json::from_str(include_str!("../../../vectors/kit_security.json")).unwrap()
    }

    #[test]
    fn policy_matches_the_reference_for_each_livekit_url() {
        let vectors = vectors();
        for case in vectors["csp"]["policies"].as_array().unwrap() {
            let url = case["livekit_url"].as_str().map(str::to_string);
            let sources: Vec<String> = serde_json::from_value(case["sources"].clone()).unwrap();
            assert_eq!(livekit_sources(url.as_deref()), sources, "{url:?}");
            let policy = content_security_policy(url.clone());
            assert_eq!(policy.build(Some("NONCE")), case["header"].as_str().unwrap(), "{url:?}");
            assert_eq!(policy.build(None), case["without_nonce"].as_str().unwrap(), "{url:?}");
            assert_eq!(policy.header_name(), "content-security-policy");
        }
        assert_eq!(vectors["csp"]["nonce_directives"], json!(["script-src"]));
        assert_eq!(vectors["csp"]["report_only"], json!(false));
    }

    #[test]
    fn nonces_hash_the_session_id_or_are_random() {
        let vectors = vectors();
        for case in vectors["csp"]["nonces"].as_array().unwrap() {
            assert_eq!(csp_nonce(case["session_id"].as_str()), case["nonce"].as_str().unwrap());
        }
        let example = vectors["csp"]["random_nonce_example"].as_str().unwrap();
        let (a, b) = (csp_nonce(None), csp_nonce(Some("")));
        assert_eq!((a.len(), b.len()), (example.len(), example.len()));
        assert_ne!(a, b);
    }

    #[test]
    fn default_headers_match_the_reference() {
        let vectors = vectors();
        let expected: Vec<(String, String)> = vectors["csp"]["default_headers"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, value)| (name.to_ascii_lowercase(), value.as_str().unwrap().to_string()))
            .collect();
        let actual: Vec<(String, String)> =
            default_headers().iter().map(|(name, value)| (name.to_string(), value.to_str().unwrap().to_string())).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn filter_parameters_match_the_reference() {
        let vectors = vectors();
        let filter = parameter_filter();
        let sources: Vec<String> = vectors["parameter_filter"]["filter_parameters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["regexp"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(filter.sources(), sources);
        for case in vectors["parameter_filter"]["params"].as_array().unwrap() {
            assert_eq!(filter.filter(&case["params"]), case["filtered"], "{}", case["params"]);
        }
        for case in vectors["parameter_filter"]["paths"].as_array().unwrap() {
            let path = case["path"].as_str().unwrap();
            let (path_only, query) = match path.split_once('?') {
                Some((path, query)) => (path, Some(query)),
                None => (path, None),
            };
            assert_eq!(filter.filtered_path(path_only, query), case["filtered"].as_str().unwrap(), "{path}");
        }
    }

    #[test]
    fn log_lines_are_scrubbed_like_the_reference() {
        let vectors = vectors();
        assert_eq!(BOT_KEY_IN_PATH.as_str(), vectors["log_scrubbing"]["pattern"].as_str().unwrap());
        for case in vectors["log_scrubbing"]["lines"].as_array().unwrap() {
            assert_eq!(scrub_log_line(case["line"].as_str().unwrap()), case["scrubbed"].as_str().unwrap());
        }
        let mut out = Vec::new();
        ScrubbingWriter(&mut out).write_all(b"GET /rooms/1/12-AbC/messages\n").unwrap();
        assert_eq!(out, b"GET /rooms/1/[FILTERED]/messages\n");
    }

    #[test]
    fn csp_violations_are_described_like_the_reference() {
        let report = json!({
            "document-uri": "https://campfire.test/rooms/1?secret=1",
            "violated-directive": "script-src-elem",
            "effective-directive": "script-src-elem",
            "blocked-uri": "https://evil.test:8443/x.js?q=1",
            "source-file": "https://campfire.test/assets/application-abc.js",
            "line-number": 12
        });
        // The line the reference logged for this report (reference-tools/kit/security_vectors.rb).
        assert_eq!(
            describe_csp_violation(report.as_object().unwrap()),
            "directive=script-src-elem blocked=https://evil.test:8443 document=/rooms/1 source=https://campfire.test line=12"
        );
        let keywords = json!({ "effectiveDirective": "script-src", "blockedURL": "inline", "documentURL": "not a url", "lineNumber": "12345678" });
        assert_eq!(describe_csp_violation(keywords.as_object().unwrap()), "directive=script-src blocked=inline");
        let long = json!({ "violated-directive": "x".repeat(70), "blocked-uri": "data", "source-file": "https://a.test:443/x" });
        assert_eq!(
            describe_csp_violation(long.as_object().unwrap()),
            format!("directive={}... blocked=data source=https://a.test", "x".repeat(57))
        );
    }
}
