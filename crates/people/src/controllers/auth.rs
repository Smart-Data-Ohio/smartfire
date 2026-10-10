//! Response rendering for the shared retained and JSON sign-in operations.
use campfire_api_types::{AuthResponse, ChallengeMethod, ChallengeState};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use serde::{Serialize, de::DeserializeOwned};

use crate::concerns::{self, Before, MatchedRoute};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResponseMode {
    Html,
    Json,
}

#[derive(Clone)]
struct JsonBody(Vec<u8>);

/// Direct JSON routes and public SPA shells run the retained action's callback policy.
pub async fn before_actions(
    c: &mut Ctx,
    endpoint: &'static str,
    before: Before,
    mode: ResponseMode,
) -> Result<()> {
    c.set_current(MatchedRoute { endpoint });
    if mode == ResponseMode::Json && !c.request.is_get() && !c.request.is_head() {
        let bytes = c.read_body(16 * 1024).await.to_vec();
        // Expose the Rails body token to the ordinary CSRF verifier. Defer DTO errors to the
        // operation so malformed submissions still reach its retained rate-limit callback.
        if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&bytes)
            && let Some(token) = body.get(campfire_kit::csrf::PARAM).and_then(|v| v.as_str())
        {
            c.params.insert(
                campfire_kit::csrf::PARAM,
                campfire_kit::Param::Str(token.into()),
            );
            c.params.merge(&c.query_params);
            c.params.merge(&c.path_params);
        }
        c.set_current(JsonBody(bytes));
    }
    concerns::before_actions(c, before).await
}

pub fn challenge_state() -> ChallengeState {
    ChallengeState {
        methods: vec![ChallengeMethod::Totp, ChallengeMethod::RecoveryCode],
        remember_device: true,
    }
}

pub fn json(c: &mut Ctx, status: StatusCode, value: &impl Serialize) -> Result {
    c.no_store();
    let body = serde_json::to_string(value).map_err(Error::internal)?;
    Ok(c.render_as(status, "application/json; charset=utf-8", body))
}

pub fn field_error(c: &mut Ctx, status: StatusCode, field: &str, message: &str) -> Result {
    json(
        c,
        status,
        &AuthResponse::Error {
            field_errors: [(field.into(), vec![message.into()])].into(),
        },
    )
}

/// Before-actions may halt with a retained redirect or an empty refusal, before an operation.
pub fn complete(c: &mut Ctx, result: Result) -> Result {
    match result {
        Err(Error::Halt(response)) => {
            if let Some(location) = response.get_header("location") {
                return json(
                    c,
                    StatusCode::OK,
                    &AuthResponse::Navigate {
                        location: location.into(),
                    },
                );
            }
            if matches!(response.body, campfire_kit::response::Body::Empty) {
                return json(
                    c,
                    response.status,
                    &AuthResponse::Error {
                        field_errors: Default::default(),
                    },
                );
            }
            Ok(*response)
        }
        result => result,
    }
}

pub async fn body<T: DeserializeOwned>(c: &mut Ctx) -> Result<T> {
    let bytes = match c.take_current::<JsonBody>() {
        Some(body) => body.0,
        None => c.read_body(16 * 1024).await.to_vec(),
    };
    match serde_json::from_slice(&bytes) {
        Ok(body) => Ok(body),
        Err(_) => campfire_kit::halt(field_error(
            c,
            StatusCode::UNPROCESSABLE_ENTITY,
            "base",
            "The request body isn't valid.",
        )?),
    }
}

impl ResponseMode {
    pub fn navigate(self, c: &mut Ctx, location: &str) -> Result {
        self.redirect(c, location, false)
    }

    pub fn signed_in(self, c: &mut Ctx, location: &str) -> Result {
        self.redirect(c, location, true)
    }

    fn redirect(self, c: &mut Ctx, location: &str, signed_in: bool) -> Result {
        // Validate with the retained redirect implementation before exposing a navigation URL.
        let redirect = c.redirect_to(location)?;
        if self == Self::Html {
            return Ok(redirect);
        }
        let location = redirect
            .get_header("location")
            .expect("redirect location")
            .to_owned();
        let response = if signed_in {
            AuthResponse::SignedIn { location }
        } else {
            AuthResponse::Navigate { location }
        };
        json(c, StatusCode::OK, &response)
    }

    pub fn challenge(self, c: &mut Ctx) -> Result {
        if self == Self::Html {
            return c.redirect_to(&c.url_for("/two_factor_challenge"));
        }
        json(
            c,
            StatusCode::OK,
            &AuthResponse::SecondFactorRequired {
                challenge: challenge_state(),
            },
        )
    }
}
