//! MCP webhook protocol controls over the shared pinned, non-redirecting transport.
use crate::{
    integrations::webhook::{self, Posted, WebhookError},
    net::{Network, http::HttpError},
};
use campfire_db::models::mcp_events::{Category, DeliveryOutcome};
use serde_json::json;
use std::time::Duration;

pub struct Signing<'a> {
    pub secret: &'a str,
    pub previous: Option<&'a str>,
    pub id: &'a str,
    pub subscription: Option<&'a str>,
}

pub async fn post<F: Fn() -> jiff::Timestamp + Send + Sync>(
    net: &Network,
    url: &str,
    signing: Signing<'_>,
    body: String,
    now: F,
) -> Result<Posted, WebhookError> {
    let Signing {
        secret,
        previous,
        id,
        subscription,
    } = signing;
    tokio::time::timeout(
        Duration::from_secs(7),
        webhook::post_payload_limited(
            net,
            url,
            body,
            |body| {
                let mut headers =
                    rails_compat::webhook::standard_headers(secret, previous, id, body, now())
                        .ok_or_else(|| WebhookError::InvalidUrl("invalid signing secret".into()))?;
                if let Some(id) = subscription {
                    headers.push(("X-MCP-Subscription-Id", id.into()));
                }
                Ok(headers)
            },
            256 * 1024,
        ),
    )
    .await
    .unwrap_or(Err(WebhookError::Http(HttpError::ReadTimeout)))
}

pub fn outcome(result: &Result<Posted, WebhookError>, now: jiff::Timestamp) -> DeliveryOutcome {
    use Category::*;
    match result {
        Ok(r) => {
            let hint = r
                .headers
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| {
                    s.parse::<u64>().ok().map(Duration::from_secs).or_else(|| {
                        httpdate::parse_http_date(s).ok().and_then(|at| {
                            at.duration_since(
                                std::time::UNIX_EPOCH
                                    + Duration::from_secs(now.as_second().max(0) as u64),
                            )
                            .ok()
                        })
                    })
                });
            match r.status {
                200..=299 => DeliveryOutcome::Delivered,
                408 | 429 => DeliveryOutcome::Retry(Http4xx, hint),
                500..=599 => DeliveryOutcome::Retry(Http5xx, hint),
                _ => DeliveryOutcome::Permanent(Http4xx),
            }
        }
        Err(WebhookError::Http(
            HttpError::OpenTimeout | HttpError::ReadTimeout | HttpError::WriteTimeout,
        )) => DeliveryOutcome::Retry(Timeout, None),
        Err(WebhookError::Http(HttpError::Tls(_))) => DeliveryOutcome::Retry(TlsError, None),
        Err(
            WebhookError::Guard(crate::net::guard::GuardError::Violation(_))
            | WebhookError::InvalidUrl(_),
        ) => DeliveryOutcome::Permanent(ConnectionRefused),
        Err(_) => DeliveryOutcome::Retry(ConnectionRefused, None),
    }
}
pub async fn verify<F: Fn() -> jiff::Timestamp + Send + Sync>(
    net: &Network,
    url: &str,
    secret: &str,
    now: F,
) -> Result<(), Category> {
    use subtle::ConstantTimeEq;
    let challenge = campfire_db::models::mcp_events::nonce("");
    let body = json!({"type":"verification","challenge":challenge}).to_string();
    let result = post(
        net,
        url,
        Signing {
            secret,
            previous: None,
            id: &campfire_db::models::mcp_events::nonce("msg_verification_"),
            subscription: None,
        },
        body,
        &now,
    )
    .await;
    match outcome(&result, now()) {
        DeliveryOutcome::Delivered => {
            let echoed = result
                .ok()
                .and_then(|r| serde_json::from_slice::<serde_json::Value>(&r.body).ok())
                .and_then(|v| v["challenge"].as_str().map(str::to_owned));
            if echoed.is_some_and(|v| bool::from(v.as_bytes().ct_eq(challenge.as_bytes()))) {
                Ok(())
            } else {
                Err(Category::ChallengeFailed)
            }
        }
        DeliveryOutcome::Retry(c, _) | DeliveryOutcome::Permanent(c) => Err(c),
        DeliveryOutcome::Terminate(_) => Err(Category::ChallengeFailed),
    }
}
