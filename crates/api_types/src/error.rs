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
    /// 403: confirm an available credential, then retry the pending write.
    SudoRequired { message: String, reauthentication: crate::SudoState },
    /// 403: two-factor setup or verification is required first.
    TwoFactorRequired { message: String, requirement: crate::TwoFactorRequirement },
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
    /// 422: link a Fizzy account on the viewer's profile first.
    FizzyNotConnected { message: String },
    /// 503: boards or the token verification request could not reach Fizzy.
    FizzyUnreachable { message: String },
    /// 422: Fizzy rejected the token; the linked account has been marked disconnected.
    FizzyTokenRejected { message: String },
    /// 422: identity works but card creation is unauthorized. The account stays connected.
    FizzyReadOnly { message: String },
    /// 422: any other card refusal, including a failed create request, as in classic.
    FizzyRefused { message: String },
    /// 409: the source thread was locked before or during posting.
    FizzyThreadLocked { message: String },
    /// 422: the card exists, but posting its reply failed validation. Do not create it again.
    FizzyReplyFailed {
        message: String,
        number: String,
        url: String,
    },
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
            ApiError::Conflict { .. } | ApiError::FizzyThreadLocked { .. } => 409,
            ApiError::InvalidAuthenticityToken { .. }
            | ApiError::Validation { .. }
            | ApiError::FizzyNotConnected { .. }
            | ApiError::FizzyTokenRejected { .. }
            | ApiError::FizzyReadOnly { .. }
            | ApiError::FizzyRefused { .. }
            | ApiError::FizzyReplyFailed { .. } => 422,
            ApiError::RateLimited { .. } => 429,
            ApiError::Unavailable { .. } | ApiError::FizzyUnreachable { .. } => 503,
        }
    }
}
