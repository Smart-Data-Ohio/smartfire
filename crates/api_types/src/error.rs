use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The body of every failed `/api/v1` response: `{"error": {"_tag": ..., "message": ...}}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApiErrorResponse {
    pub error: ApiError,
}

/// A typed API error, tagged by `_tag`. Each variant has one HTTP status
/// ([`ApiError::status`]); the front end decodes them into `Schema.TaggedError` classes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "_tag", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum ApiError {
    /// 401: not signed in, or the session ended.
    Unauthorized { message: String },
    /// 403: signed in, but not allowed.
    Forbidden { message: String },
    /// 403: the action needs a fresh password confirmation (`/sudo/new`).
    SudoRequired { message: String },
    /// 403: two-factor setup or verification is required first.
    TwoFactorRequired { message: String },
    /// 404.
    NotFound { message: String },
    /// 409: the record changed underneath the request.
    Conflict { message: String },
    /// 422: the request's `X-CSRF-Token` header was missing or stale. The client fetches a fresh
    /// token (`GET /api/v1/boot`'s `csrfToken`) and retries once.
    InvalidAuthenticityToken { message: String },
    /// 422: the input didn't validate. `fields` maps each attribute to its messages.
    Validation {
        message: String,
        fields: BTreeMap<String, Vec<String>>,
    },
    /// 429: try again after `retryAfter` seconds.
    RateLimited { message: String, retry_after: u32 },
    /// 503: the feature isn't set up on this server ("Huddles are not configured"). Not worth
    /// retrying; the client hides the feature.
    Unavailable { message: String },
}

impl ApiError {
    /// The HTTP status that carries this error.
    pub fn status(&self) -> u16 {
        match self {
            ApiError::Unauthorized { .. } => 401,
            ApiError::Forbidden { .. }
            | ApiError::SudoRequired { .. }
            | ApiError::TwoFactorRequired { .. } => 403,
            ApiError::NotFound { .. } => 404,
            ApiError::Conflict { .. } => 409,
            ApiError::InvalidAuthenticityToken { .. } | ApiError::Validation { .. } => 422,
            ApiError::RateLimited { .. } => 429,
            ApiError::Unavailable { .. } => 503,
        }
    }
}
