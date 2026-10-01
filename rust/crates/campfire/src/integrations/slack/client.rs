//! `app/models/slack/client.rb`: fixed-host, user-token, read-only Slack Web API client.
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use hyper::Method;
use serde_json::Value;
use tokio::time::Instant;

use crate::integrations::net::{
    Network,
    http::{self, Body, Endpoint, Request, Timeouts},
};

const HOST: &str = "slack.com";
const MAX_ATTEMPTS: usize = 4;
const BACKOFF: [u64; 3] = [1, 2, 4];
const DEFAULT_RETRY_AFTER: u64 = 60;
#[derive(Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Auth,
    Scope,
    RateLimited,
    Request,
    Http,
    Network,
}
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
    pub retry_after: Option<u64>,
    pub needed: Box<Value>,
    pub provided: Box<Value>,
}
impl Error {
    pub(super) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        let retry_after = (kind == ErrorKind::RateLimited).then_some(DEFAULT_RETRY_AFTER);
        Self {
            kind,
            message: message.into(),
            retry_after,
            needed: Box::new(Value::Null),
            provided: Box::new(Value::Null),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Tier {
    Two,
    Three,
}
impl Tier {
    fn interval(self) -> Duration {
        Duration::from_secs_f64(60.0 / if self == Self::Two { 18.0 } else { 45.0 })
    }
}
pub type OnRequest = Arc<dyn Fn(&str) + Send + Sync>;
pub struct Client {
    token: String,
    network: Network,
    pacing: bool,
    on_request: Option<OnRequest>,
    last_call_at: HashMap<Tier, Instant>,
    requests: u64,
}
impl Client {
    pub fn new(token: String, on_request: Option<OnRequest>) -> Self {
        Self::with_network(token, on_request, true, Network::system())
    }
    pub fn with_network(
        token: String,
        on_request: Option<OnRequest>,
        pacing: bool,
        network: Network,
    ) -> Self {
        Self {
            token,
            network,
            pacing,
            on_request,
            last_call_at: HashMap::new(),
            requests: 0,
        }
    }
    pub fn request_count(&self) -> u64 {
        self.requests
    }
    pub async fn auth_test(&mut self) -> Result<Value, Error> {
        self.request("auth.test", &[], Tier::Three).await
    }
    pub async fn team_info(&mut self) -> Result<Value, Error> {
        self.request("team.info", &[], Tier::Three).await
    }
    pub async fn users_list(&mut self, cursor: Option<&str>, limit: usize) -> Result<Value, Error> {
        self.request(
            "users.list",
            &[
                ("cursor", cursor.map(str::to_owned)),
                ("limit", Some(limit.to_string())),
            ],
            Tier::Two,
        )
        .await
    }
    pub async fn conversations_list(
        &mut self,
        types: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, Error> {
        self.request(
            "conversations.list",
            &[
                ("types", Some(types.to_owned())),
                ("exclude_archived", Some("false".into())),
                ("cursor", cursor.map(str::to_owned)),
                ("limit", Some(limit.to_string())),
            ],
            Tier::Two,
        )
        .await
    }
    pub async fn conversations_members(
        &mut self,
        channel: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, Error> {
        self.request(
            "conversations.members",
            &[
                ("channel", Some(channel.to_owned())),
                ("cursor", cursor.map(str::to_owned)),
                ("limit", Some(limit.to_string())),
            ],
            Tier::Three,
        )
        .await
    }
    pub async fn conversations_history(
        &mut self,
        channel: &str,
        bounds: Bounds<'_>,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, Error> {
        self.request(
            "conversations.history",
            &[
                ("channel", Some(channel.to_owned())),
                ("oldest", bounds.oldest.map(str::to_owned)),
                ("latest", bounds.latest.map(str::to_owned)),
                ("cursor", cursor.map(str::to_owned)),
                ("limit", Some(limit.to_string())),
            ],
            Tier::Three,
        )
        .await
    }
    pub async fn conversations_replies(
        &mut self,
        channel: &str,
        ts: &str,
        bounds: Bounds<'_>,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, Error> {
        self.request(
            "conversations.replies",
            &[
                ("channel", Some(channel.to_owned())),
                ("ts", Some(ts.to_owned())),
                ("oldest", bounds.oldest.map(str::to_owned)),
                ("latest", bounds.latest.map(str::to_owned)),
                ("cursor", cursor.map(str::to_owned)),
                ("limit", Some(limit.to_string())),
            ],
            Tier::Three,
        )
        .await
    }
    async fn pace(&mut self, tier: Tier) {
        if !self.pacing {
            return;
        }
        if let Some(last) = self.last_call_at.get(&tier)
            && let Some(wait) = tier.interval().checked_sub(last.elapsed())
        {
            tokio::time::sleep(wait).await;
        }
        self.last_call_at.insert(tier, Instant::now());
    }
    async fn request(
        &mut self,
        method: &str,
        params: &[(&str, Option<String>)],
        tier: Tier,
    ) -> Result<Value, Error> {
        for attempt in 0..MAX_ATTEMPTS {
            self.pace(tier).await;
            self.requests += 1;
            let result = self.get(method, params).await;
            match result {
                Err(error)
                    if matches!(error.kind, ErrorKind::Http | ErrorKind::Network)
                        && attempt + 1 < MAX_ATTEMPTS =>
                {
                    if self.pacing {
                        tokio::time::sleep(Duration::from_secs(
                            BACKOFF.get(attempt).copied().unwrap_or(4),
                        ))
                        .await;
                    }
                }
                Err(mut error) if error.kind == ErrorKind::Network => {
                    error.kind = ErrorKind::Request;
                    return Err(error);
                }
                result => return result,
            }
        }
        unreachable!("final attempt returns")
    }
    async fn get(&self, method: &str, params: &[(&str, Option<String>)]) -> Result<Value, Error> {
        if let Some(on_request) = &self.on_request {
            on_request(method);
        }
        let query = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(
                params
                    .iter()
                    .filter_map(|(key, value)| value.as_deref().map(|value| (*key, value))),
            )
            .finish();
        let endpoint = Endpoint {
            https: true,
            host: HOST.into(),
            port: 443,
            pinned_ip: None,
        };
        let request = Request::net_http(
            Method::GET,
            format!("/api/{method}?{query}"),
            Some(HOST.into()),
            vec![
                ("Accept".into(), "application/json; charset=utf-8".into()),
                ("User-Agent".into(), "Smartfire-Slack-Import".into()),
                ("Authorization".into(), format!("Bearer {}", self.token)),
            ],
        )
        .transport(false, &endpoint);
        let transport_error = |_: http::HttpError| {
            Error::new(
                ErrorKind::Network,
                format!("Slack network error for {method}"),
            )
        };
        let response = http::exchange(
            &self.network,
            &endpoint,
            request,
            &Timeouts {
                open: Duration::from_secs(10),
                read: Duration::from_secs(30),
                write: Duration::from_secs(60),
            },
        )
        .await
        .map_err(transport_error)?;
        let status = response.status;
        if status == 429 {
            let mut error = Error::new(
                ErrorKind::RateLimited,
                format!("Slack rate limited {method}"),
            );
            error.retry_after = Some(retry_after(response.header("Retry-After").as_deref()));
            return Err(error);
        }
        if !((200..300).contains(&status) || matches!(status, 400 | 401 | 403 | 404)) {
            return Err(Error::new(
                ErrorKind::Http,
                format!("Slack HTTP {status} for {method}"),
            ));
        }
        let body = match response
            .read_body(usize::MAX)
            .await
            .map_err(transport_error)?
        {
            Body::Complete(bytes) => bytes,
            Body::TooLarge => unreachable!("fixed-host unbounded body"),
        };
        let payload: Value = serde_json::from_slice(&body).map_err(|_| {
            if (200..300).contains(&status) {
                Error::new(
                    ErrorKind::Http,
                    format!("Slack returned invalid JSON for {method}"),
                )
            } else {
                Error::new(
                    ErrorKind::Request,
                    format!("Slack HTTP {status} for {method}"),
                )
            }
        })?;
        check_ok(payload, method)
    }
}
#[derive(Clone, Copy, Default)]
pub struct Bounds<'a> {
    pub oldest: Option<&'a str>,
    pub latest: Option<&'a str>,
}
fn retry_after(header: Option<&str>) -> u64 {
    let h = header.unwrap_or("").trim_start();
    let h = h.strip_prefix('+').unwrap_or(h);
    let digits: String = h.chars().take_while(char::is_ascii_digit).collect();
    digits
        .parse::<u64>()
        .ok()
        .filter(|s| *s > 0)
        .unwrap_or(DEFAULT_RETRY_AFTER)
}
fn check_ok(payload: Value, method: &str) -> Result<Value, Error> {
    if !payload["ok"].is_null() && payload["ok"] != false {
        return Ok(payload);
    }
    let code = payload["error"].as_str().unwrap_or("");
    let mut error = match code {
        "invalid_auth" | "token_revoked" | "account_inactive" | "not_authed" => Error::new(
            ErrorKind::Auth,
            format!("Slack authentication failed for {method} ({code})"),
        ),
        "missing_scope" => {
            let needed = match &payload["needed"] {
                Value::Array(a) => a
                    .iter()
                    .map(|v| v.as_str().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .join(","),
                Value::String(s) => s.clone(),
                _ => String::new(),
            };
            Error::new(
                ErrorKind::Scope,
                format!(
                    "Slack token is missing a required scope for {method} (needed: {})",
                    if needed.trim().is_empty() {
                        "unknown"
                    } else {
                        &needed
                    }
                ),
            )
        }
        "ratelimited" => Error::new(
            ErrorKind::RateLimited,
            format!("Slack rate limited {method}"),
        ),
        _ => Error::new(
            ErrorKind::Request,
            format!(
                "Slack error for {method}: {}",
                if code.trim().is_empty() {
                    "unknown"
                } else {
                    code
                }
            ),
        ),
    };
    if error.kind == ErrorKind::Scope {
        error.needed = Box::new(payload["needed"].clone());
        error.provided = Box::new(payload["provided"].clone());
    }
    Err(error)
}

#[cfg(test)]
pub(crate) mod tests;
