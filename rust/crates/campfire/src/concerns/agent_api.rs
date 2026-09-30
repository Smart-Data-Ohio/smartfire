//! AgentApiThrottle's minute buckets, shared by REST actions and MCP tools.
use campfire_kit::{Ctx, Error, Result, StatusCode, format, halt};
use jiff::SignedDuration;
use sha2::{Digest, Sha256};

use super::{AuthenticatedBy, Before, CurrentAgent, authenticated_by, before_actions};

pub async fn require_token(c: &mut Ctx, rescue_forgery: bool) -> Result<CurrentAgent> {
    match before_actions(c, Before::default().allow_agent_access()).await {
        Err(Error::InvalidAuthenticityToken(_)) if rescue_forgery => return reject_session(c),
        result => result?,
    }
    if authenticated_by(c) == AuthenticatedBy::AgentToken
        && let Some(identity) = c.current::<CurrentAgent>()
    {
        return Ok(*identity);
    }
    reject_session(c)
}

fn reject_session<T>(c: &mut Ctx) -> Result<T> {
    halt(c.render(
        StatusCode::FORBIDDEN,
        &format::JSON,
        serde_json::json!({"error":"Forbidden: Bearer agent token required"}).to_string(),
    ))
}

pub fn no_store(c: &mut Ctx) {
    c.no_store();
    c.set_header("pragma", "no-cache");
}

/// Count every call, including failed calls, under the credential digest. A session
/// with no Bearer header skips the counter, as Rails' agent_throttle_key does.
pub fn retry_after(c: &Ctx, limit: u64, controller: &str, action: &str) -> Option<i64> {
    retry_after_for(
        c.kit().rate_limits(),
        c.request.header("authorization"),
        c.now(),
        limit,
        controller,
        action,
    )
}

fn retry_after_for(
    store: &campfire_kit::RateLimitStore,
    authorization: Option<&str>,
    now: jiff::Timestamp,
    limit: u64,
    controller: &str,
    action: &str,
) -> Option<i64> {
    let authorization = authorization?;
    let prefix = authorization.get(..7)?;
    if !prefix.eq_ignore_ascii_case("Bearer ") {
        return None;
    }
    let token = authorization[7..].trim();
    if token.is_empty() {
        return None;
    }
    let seconds = now.as_second();
    let bucket = seconds.div_euclid(60);
    let digest = format!("{:x}", Sha256::digest(token));
    let key = format!("agent_api_throttle:{controller}:{action}:{bucket}:{digest}");
    let count = store.increment(&key, SignedDuration::from_secs(65), now);
    (count > limit).then_some(((bucket + 1) * 60 - seconds).max(1))
}

pub fn throttle(c: &mut Ctx, limit: u64, controller: &str, action: &str) -> Result<()> {
    if let Some(seconds) = retry_after(c, limit, controller, action) {
        c.set_header("retry-after", &seconds.to_string());
        return halt(c.render(
            StatusCode::TOO_MANY_REQUESTS,
            &format::JSON,
            serde_json::json!({"error":"rate_limited"}).to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn buckets_reset_on_minute_boundaries_and_isolate_credentials_and_actions() {
        let store = campfire_kit::RateLimitStore::new();
        let now = "2026-03-02T16:00:59Z".parse().unwrap();
        let authorization = ["Bearer", "one"].join(" ");
        let other = ["Bearer", "two"].join(" ");
        assert_eq!(
            retry_after_for(
                &store,
                Some(&authorization),
                now,
                1,
                "agents/events",
                "index"
            ),
            None
        );
        assert_eq!(
            retry_after_for(
                &store,
                Some(&authorization),
                now,
                1,
                "agents/events",
                "index"
            ),
            Some(1)
        );
        assert_eq!(
            retry_after_for(&store, Some(&other), now, 1, "agents/events", "index"),
            None
        );
        assert_eq!(
            retry_after_for(&store, Some(&authorization), now, 1, "agents/events", "ack"),
            None
        );
        assert_eq!(
            retry_after_for(&store, None, now, 1, "agents/events", "index"),
            None
        );
        let next = now + SignedDuration::from_secs(1);
        assert_eq!(
            retry_after_for(
                &store,
                Some(&authorization),
                next,
                1,
                "agents/events",
                "index"
            ),
            None
        );
    }
}
