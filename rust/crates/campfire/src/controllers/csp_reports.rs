//! `ContentSecurityPolicyReportsController` (reference/app/controllers/
//! content_security_policy_reports_controller.rb): `POST /csp_reports` takes violation reports
//! (`report-uri`'s `application/csp-report`, or the Reporting API's `application/reports+json`) and
//! logs one line for each of the first ten.
//!
//! An `ActionController::API`: no session, cookies, forgery protection or `ApplicationController`
//! chain, so nothing authenticated to forge. It's reached without the Rails route table
//! (`app::router` mounts it), and it's rate-limited per IP in a store of its own, so a report
//! flood never touches the shared one.

use std::sync::{Arc, LazyLock};

use campfire_kit::{Ctx, RateLimit, RateLimitStore, Result, StatusCode};
use jiff::SignedDuration;
use serde_json::Value;

use crate::security::describe_csp_violation;

/// `MAX_BODY = 16.kilobytes`
pub const MAX_BODY: usize = 16 * 1024;
const MAX_VIOLATIONS: usize = 10;

/// `RATE_LIMIT_STORE = ActiveSupport::Cache::MemoryStore.new`
static RATE_LIMIT_STORE: LazyLock<Arc<RateLimitStore>> = LazyLock::new(Default::default);

/// `rate_limit to: 20, within: 1.minute, store: RATE_LIMIT_STORE, with: -> { head :too_many_requests }`
fn rate_limit() -> RateLimit {
    RateLimit::new("content_security_policy_reports", 20, SignedDuration::from_mins(1)).store(RATE_LIMIT_STORE.clone())
}

pub async fn create(c: &mut Ctx) -> Result {
    if c.rate_limited(&rate_limit(), None)? {
        return Ok(c.head(StatusCode::TOO_MANY_REQUESTS));
    }
    for violation in violations(c.request.raw_post()).iter().take(MAX_VIOLATIONS) {
        tracing::warn!("CSP violation: {}", describe_csp_violation(violation));
    }
    Ok(c.head(StatusCode::NO_CONTENT))
}

/// The reports in a body of at most [`MAX_BODY`] bytes: one report or a list, each either
/// `{"csp-report": {...}}` or the Reporting API's `{"body": {...}}`. Anything else is nothing.
fn violations(body: &[u8]) -> Vec<serde_json::Map<String, Value>> {
    if body.len() > MAX_BODY {
        return vec![];
    }
    let Ok(parsed) = serde_json::from_slice::<Value>(body) else { return vec![] };
    // `Array.wrap`: a list as is, anything else as a list of one.
    let entries = match parsed {
        Value::Array(entries) => entries,
        other => vec![other],
    };
    entries
        .into_iter()
        .filter_map(|entry| {
            let Value::Object(mut entry) = entry else { return None };
            let report = match entry.remove("csp-report") {
                Some(report) if !report.is_null() && report != Value::Bool(false) => report,
                _ => entry.remove("body")?,
            };
            match report {
                Value::Object(report) => Some(report),
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_report_formats_and_ignores_the_rest() {
        let legacy = br#"{"csp-report":{"violated-directive":"img-src"}}"#;
        assert_eq!(violations(legacy).len(), 1);
        let reporting_api = br#"[{"type":"csp-violation","body":{"effectiveDirective":"script-src"}},{"body":"x"},3,{"csp-report":null,"body":{"a":1}}]"#;
        let reports = violations(reporting_api);
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0]["effectiveDirective"], "script-src");
        assert!(violations(b"not json").is_empty());
        assert!(violations(b"null").is_empty());
    }

    #[test]
    fn bodies_over_16_kilobytes_are_ignored() {
        let report = br#"{"csp-report":{"violated-directive":"img-src"}}"#;
        let mut padded = report.to_vec();
        padded.resize(MAX_BODY, b' ');
        assert_eq!(violations(&padded).len(), 1, "16 KB exactly is read");
        padded.push(b' ');
        assert!(violations(&padded).is_empty());
    }
}
