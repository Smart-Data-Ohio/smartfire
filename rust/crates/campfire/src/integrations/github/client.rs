//! `app/models/github/{app,write_client,pull_request_fetcher}.rb`.
//! Destinations are fixed; DNS/dialing/TLS can be supplied by the local fake-server tests.
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use hyper::Method;
use serde_json::{Value, json};

use super::blank;
use crate::integrations::net::{
    Network,
    http::{self, Body, Endpoint, HttpError, Request, Timeouts},
};

pub const API_VERSION: &str = "2022-11-28";
pub const TIMEOUT: Duration = Duration::from_secs(10);
const API_HOST: &str = "api.github.com";
const TOKEN_HOST: &str = "github.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Unauthorized,
    Refused,
    Other,
    /// PullRequestFetcher::FetchError: HTTP refusal, rescued by review/check/file fetches.
    Fetch,
    // `authenticated_login` calls `fetch` outside WriteClient's request rescue.
    KeyError,
    TypeError,
    NoMethodError,
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

impl Error {
    pub(super) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    fn transport(error: HttpError) -> Self {
        // No endpoint, body, credential, or raw error string is logged or returned.
        let class = match error {
            HttpError::OpenTimeout => "Open Timeout",
            HttpError::ReadTimeout => "Read Timeout",
            HttpError::Unresolvable(_) => "Socket Error",
            HttpError::Tls(_) => "Ssl Error",
            HttpError::Io(ref e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                "Econnrefused"
            }
            HttpError::Io(_) => "Io Error",
            HttpError::Http(_) => "Http Error",
            HttpError::Inflate(_) => "Error",
        };
        Self::new(
            ErrorKind::Other,
            format!("Could not reach GitHub ({class})"),
        )
    }
    fn json() -> Self {
        Self::new(ErrorKind::Other, "Could not reach GitHub (Parser Error)")
    }
}

#[derive(Clone)]
struct Http {
    network: Network,
}
impl Http {
    async fn request(
        &self,
        host: &str,
        method: Method,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<(u16, http::Response), Error> {
        let endpoint = Endpoint {
            https: true,
            host: host.into(),
            port: 443,
            pinned_ip: None,
        };
        let mut request =
            Request::net_http(method, path, Some(host.into()), headers).transport(false, &endpoint);
        request.body = body;
        let response = http::exchange(
            &self.network,
            &endpoint,
            request,
            &Timeouts {
                open: TIMEOUT,
                read: TIMEOUT,
            },
        )
        .await
        .map_err(Error::transport)?;
        Ok((response.status, response))
    }
    async fn body(response: http::Response) -> Result<Vec<u8>, Error> {
        match response
            .read_body(usize::MAX)
            .await
            .map_err(Error::transport)?
        {
            Body::Complete(body) => Ok(body),
            Body::TooLarge => unreachable!("unbounded fixed-host response"),
        }
    }
}

/// `Github::App`, credentials read at construction (never printed through `Debug`).
#[derive(Clone)]
pub struct AppClient {
    client_id: Option<String>,
    client_secret: Option<String>,
    http: Http,
}
impl AppClient {
    pub fn from_env() -> Self {
        Self::new(
            std::env::var("GITHUB_APP_CLIENT_ID").ok(),
            std::env::var("GITHUB_APP_CLIENT_SECRET").ok(),
        )
    }
    pub fn new(client_id: Option<String>, client_secret: Option<String>) -> Self {
        Self::with_network(client_id, client_secret, Network::system())
    }
    pub fn with_network(
        client_id: Option<String>,
        client_secret: Option<String>,
        network: Network,
    ) -> Self {
        Self {
            client_id: client_id.filter(|s| !blank(s)),
            client_secret: client_secret.filter(|s| !blank(s)),
            http: Http { network },
        }
    }
    pub fn configured(&self) -> bool {
        self.client_id.is_some() && self.client_secret.is_some()
    }
    pub fn authorize_url(&self, redirect_uri: &str, state: &str) -> String {
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("client_id", self.client_id.as_deref().unwrap_or(""))
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("state", state)
            .append_pair("scope", "")
            .finish();
        format!("https://github.com/login/oauth/authorize?{query}")
    }
    pub async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<Value, Error> {
        self.post_oauth(&[("code", code), ("redirect_uri", redirect_uri)])
            .await
    }
    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<Value, Error> {
        self.post_oauth(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .await
    }
    async fn post_oauth(&self, fields: &[(&str, &str)]) -> Result<Value, Error> {
        let body = {
            let mut form = url::form_urlencoded::Serializer::new(String::new());
            form.append_pair("client_id", self.client_id.as_deref().unwrap_or(""));
            form.append_pair("client_secret", self.client_secret.as_deref().unwrap_or(""));
            for (key, value) in fields {
                form.append_pair(key, value);
            }
            form.finish().into_bytes()
        };
        let headers = vec![
            (
                "Content-Type".into(),
                "application/x-www-form-urlencoded".into(),
            ),
            ("Accept".into(), "application/json".into()),
        ];
        let (status, response) = self
            .http
            .request(
                TOKEN_HOST,
                Method::POST,
                "/login/oauth/access_token".into(),
                headers,
                body,
            )
            .await?;
        let body: Value =
            serde_json::from_slice(&Http::body(response).await?).map_err(|_| Error::json())?;
        // Rails tests the access_token before the error field, including when both are present.
        if (200..300).contains(&status) && present(ruby_field(&body, "access_token")?.as_ref()) {
            Ok(body)
        } else if status == 401 || present(ruby_field(&body, "error")?.as_ref()) {
            Err(Error::new(
                ErrorKind::Unauthorized,
                "GitHub rejected the GitHub App grant (401)",
            ))
        } else {
            Err(Error::new(
                ErrorKind::Other,
                format!("GitHub App token request failed ({status})"),
            ))
        }
    }
    pub async fn revoke_token(&self, token: &str) -> bool {
        self.revoke("token", token, true).await
    }
    pub async fn revoke_grant(&self, token: &str) -> bool {
        self.revoke("grant", token, false).await
    }
    async fn revoke(&self, segment: &str, token: &str, unknown_means_gone: bool) -> bool {
        let basic = STANDARD.encode(format!(
            "{}:{}",
            self.client_id.as_deref().unwrap_or(""),
            self.client_secret.as_deref().unwrap_or("")
        ));
        let headers = vec![
            ("Content-Type".into(), "application/json".into()),
            ("Accept".into(), "application/vnd.github+json".into()),
            ("Authorization".into(), format!("Basic {basic}")),
        ];
        let result = self
            .http
            .request(
                API_HOST,
                Method::DELETE,
                format!(
                    "/applications/{}/{segment}",
                    self.client_id.as_deref().unwrap_or("")
                ),
                headers,
                json!({"access_token": token}).to_string().into_bytes(),
            )
            .await;
        match result {
            Ok((status, response)) => {
                Http::body(response).await.is_ok()
                    && (status == 204 || (unknown_means_gone && status == 404))
            }
            Err(_) => false,
        }
    }
}

/// The already-validated names/numbers on `Github::PullRequest`.
#[derive(Clone, Debug)]
pub struct PullRequestKey {
    pub owner: String,
    pub repo: String,
    pub number: i64,
}
impl PullRequestKey {
    fn path(&self, suffix: &str) -> String {
        format!("/repos/{}/{}/{suffix}", self.owner, self.repo)
    }
}

/// `Github::WriteClient`: a member's token, never the workspace token.
#[derive(Clone)]
pub struct WriteClient {
    token: String,
    http: Http,
}
impl WriteClient {
    pub fn new(token: String) -> Self {
        Self::with_network(token, Network::system())
    }
    pub fn with_network(token: String, network: Network) -> Self {
        Self {
            token,
            http: Http { network },
        }
    }
    pub async fn get_user(&self) -> Result<Value, Error> {
        self.request(Method::GET, "/user".into(), None).await
    }
    pub async fn authenticated_login(&self) -> Result<String, Error> {
        let body = self.get_user().await?;
        let login = match &body {
            Value::Object(object) => object
                .get("login")
                .ok_or_else(|| Error::new(ErrorKind::KeyError, "key not found: \"login\""))?,
            Value::Array(_) => {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    "no implicit conversion of String into Integer",
                ));
            }
            Value::Null => {
                return Err(Error::new(
                    ErrorKind::NoMethodError,
                    "undefined method 'fetch' for nil",
                ));
            }
            Value::String(_) => {
                return Err(Error::new(
                    ErrorKind::NoMethodError,
                    "undefined method 'fetch' for an instance of String",
                ));
            }
            Value::Bool(value) => {
                return Err(Error::new(
                    ErrorKind::NoMethodError,
                    format!("undefined method 'fetch' for {value}"),
                ));
            }
            Value::Number(value) => {
                return Err(Error::new(
                    ErrorKind::NoMethodError,
                    format!(
                        "undefined method 'fetch' for an instance of {}",
                        if value.is_f64() { "Float" } else { "Integer" }
                    ),
                ));
            }
        };
        let login = ruby_string(login);
        if blank(&login) {
            Err(Error::new(
                ErrorKind::Other,
                "GitHub did not return a login",
            ))
        } else {
            Ok(login)
        }
    }
    pub async fn repository_readable(&self, owner: &str, repo: &str) -> Result<bool, Error> {
        match self
            .request(Method::GET, format!("/repos/{owner}/{repo}"), None)
            .await
        {
            Ok(_) => Ok(true),
            Err(e) if e.kind == ErrorKind::Refused => Ok(false),
            Err(e) => Err(e),
        }
    }
    pub async fn create_issue_comment(
        &self,
        pr: &PullRequestKey,
        body: &str,
    ) -> Result<Value, Error> {
        self.request(
            Method::POST,
            pr.path(&format!("issues/{}/comments", pr.number)),
            Some(json!({"body": body})),
        )
        .await
    }
    pub async fn create_review(
        &self,
        pr: &PullRequestKey,
        event: &str,
        body: Option<&str>,
    ) -> Result<Value, Error> {
        let mut payload = json!({"event": event});
        if let Some(body) = body.filter(|b| !blank(b)) {
            payload["body"] = body.into();
        }
        self.request(
            Method::POST,
            pr.path(&format!("pulls/{}/reviews", pr.number)),
            Some(payload),
        )
        .await
    }
    pub async fn request_reviewers(
        &self,
        pr: &PullRequestKey,
        logins: &[String],
    ) -> Result<Value, Error> {
        self.request(
            Method::POST,
            pr.path(&format!("pulls/{}/requested_reviewers", pr.number)),
            Some(json!({"reviewers": logins})),
        )
        .await
    }
    async fn request(
        &self,
        method: Method,
        path: String,
        payload: Option<Value>,
    ) -> Result<Value, Error> {
        let headers = vec![
            ("Accept".into(), "application/vnd.github+json".into()),
            ("X-GitHub-Api-Version".into(), API_VERSION.into()),
            ("User-Agent".into(), "Smartfire-GitHub-Writes".into()),
            ("Content-Type".into(), "application/json".into()),
            ("Authorization".into(), format!("Bearer {}", self.token)),
        ];
        let body = payload
            .map(|p| p.to_string().into_bytes())
            .unwrap_or_default();
        let (status, response) = self
            .http
            .request(API_HOST, method, path, headers, body)
            .await?;
        let raw = Http::body(response).await?;
        let parsed: Value = serde_json::from_slice(&raw).unwrap_or_else(|_| json!({}));
        match status {
            200..=299 => Ok(parsed),
            401 => Err(Error::new(
                ErrorKind::Unauthorized,
                "GitHub rejected the linked token",
            )),
            403 | 404 | 422 => {
                let raw = ruby_string(&ruby_field(&parsed, "message")?.unwrap_or(Value::Null))
                    .replace("@[", "@\u{200b}[");
                let message = raw
                    .split(['\r', '\n'])
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                let message = ruby_strip(&message);
                let message = if message.is_empty() {
                    "request was not allowed"
                } else {
                    message
                };
                Err(Error::new(
                    ErrorKind::Refused,
                    format!("GitHub refused: {message}"),
                ))
            }
            _ => Err(Error::new(
                ErrorKind::Other,
                format!("GitHub returned {status}"),
            )),
        }
    }
}

/// REST reads for the workspace card fetcher, with its distinct headers and error messages.
#[derive(Clone)]
pub struct ReadClient {
    token: Option<String>,
    http: Http,
}
impl ReadClient {
    pub fn from_env() -> Self {
        Self::new(std::env::var("GITHUB_TOKEN").ok())
    }
    pub fn new(token: Option<String>) -> Self {
        Self::with_network(token, Network::system())
    }
    pub fn with_network(token: Option<String>, network: Network) -> Self {
        Self {
            token: token.filter(|s| !blank(s)),
            http: Http { network },
        }
    }
    pub async fn pull_request(&self, pr: &PullRequestKey) -> Result<Value, Error> {
        self.get(pr, &format!("pulls/{}", pr.number)).await
    }
    pub async fn reviews(&self, pr: &PullRequestKey) -> Result<Value, Error> {
        self.get(pr, &format!("pulls/{}/reviews?per_page=100", pr.number))
            .await
    }
    pub async fn files(&self, pr: &PullRequestKey) -> Result<Value, Error> {
        self.get(pr, &format!("pulls/{}/files?per_page=100", pr.number))
            .await
    }
    pub async fn check_runs(&self, pr: &PullRequestKey, sha: &str) -> Result<Value, Error> {
        self.get(pr, &format!("commits/{sha}/check-runs?per_page=100"))
            .await
    }
    pub async fn status(&self, pr: &PullRequestKey, sha: &str) -> Result<Value, Error> {
        self.get(pr, &format!("commits/{sha}/status")).await
    }
    async fn get(&self, pr: &PullRequestKey, path: &str) -> Result<Value, Error> {
        let mut headers = vec![
            ("Accept".into(), "application/vnd.github+json".into()),
            ("X-GitHub-Api-Version".into(), API_VERSION.into()),
            ("User-Agent".into(), "Smartfire-GitHub-Cards".into()),
        ];
        if let Some(token) = &self.token {
            headers.push(("Authorization".into(), format!("Bearer {token}")));
        }
        let (status, response) = self
            .http
            .request(API_HOST, Method::GET, pr.path(path), headers, Vec::new())
            .await?;
        let message = match status {
            200..=299 => None,
            404 => Some("Pull request not found on GitHub".to_string()),
            401 => Some("GitHub authentication failed".to_string()),
            403 | 429 if response.header("X-RateLimit-Remaining").as_deref() == Some("0") => {
                let reset = ruby_to_i(
                    response
                        .header("X-RateLimit-Reset")
                        .as_deref()
                        .unwrap_or(""),
                );
                let suffix = jiff::Timestamp::from_second(reset)
                    .map(|t| t.strftime(", resets %d %b %H:%M").to_string())
                    .unwrap_or_default();
                Some(format!("GitHub rate limit exceeded{suffix}"))
            }
            403 | 429 => Some("GitHub request forbidden".to_string()),
            _ => Some(format!("GitHub returned {status}")),
        };
        let body = Http::body(response).await?;
        if let Some(message) = message {
            return Err(Error::new(ErrorKind::Fetch, message));
        }
        serde_json::from_slice(&body).map_err(|_| Error::json())
    }
}

// Ruby `value["key"]`: JSON strings use substring lookup; non-indexable values raise.
pub(super) fn ruby_field(value: &Value, key: &str) -> Result<Option<Value>, Error> {
    match value {
        Value::Object(object) => Ok(object.get(key).cloned()),
        Value::String(text) => Ok(text.contains(key).then(|| Value::String(key.into()))),
        Value::Null | Value::Bool(_) => Err(Error::new(
            ErrorKind::Other,
            "Could not reach GitHub (No Method Error)",
        )),
        Value::Array(_) | Value::Number(_) => Err(Error::new(
            ErrorKind::Other,
            "Could not reach GitHub (Type Error)",
        )),
    }
}

pub(super) fn present(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null | Value::Bool(false)) => false,
        Some(Value::String(s)) => !blank(s),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        _ => true,
    }
}
pub(super) fn ruby_strip(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c', '\0'])
}
pub(crate) fn ruby_to_i(s: &str) -> i64 {
    let s = s.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let (negative, s) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let mut value = 0i64;
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        match b {
            b'0'..=b'9' => value = value.saturating_mul(10).saturating_add((b - b'0') as i64),
            b'_' if i > 0
                && bytes[i - 1].is_ascii_digit()
                && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) => {}
            _ => break,
        }
    }
    if negative { -value } else { value }
}
pub(crate) fn ruby_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Array(a) => format!(
            "[{}]",
            a.iter().map(ruby_inspect).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!("{} => {}", json!(k), ruby_inspect(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => value.to_string(),
    }
}
fn ruby_inspect(value: &Value) -> String {
    match value {
        Value::Null => "nil".into(),
        Value::String(s) => json!(s).to_string(),
        _ => ruby_string(value),
    }
}
