//! Every `/api/v1` failure as the typed envelope `{"error":{"_tag":...}}` with its status
//! ([`ApiError::status`]), whatever produced it: a before-action's halt, the kit's errors, or a
//! failed validation.

use std::collections::BTreeMap;

use campfire_api_types::{ApiError, ApiErrorResponse};
use campfire_kit::{Ctx, Error, Param, Response, Result, StatusCode};
use campfire_web::concerns;

/// Runs before an endpoint: every request is a JSON one (Rails' `defaults: { format: :json }`),
/// so the before-actions answer a signed-out caller with a 401 rather than the sign-in redirect,
/// and nothing is cached.
pub(crate) fn prepare(c: &mut Ctx) {
    c.params.insert("format", Param::Str("json".into()));
    c.set_header(axum::http::header::CACHE_CONTROL, "no-store");
}

/// The endpoint's response, or its failure as the envelope. Errors with no envelope (a
/// server error) go on to the kit's error pages.
pub(crate) fn respond(c: &mut Ctx, result: Result) -> Result {
    let error = match result {
        Ok(response) => return Ok(response),
        Err(error) => error,
    };
    match envelope(c, &error) {
        Some(api) => render(c, &api),
        None => Err(error),
    }
}

/// An error that answers with `api` as it is.
pub(crate) fn fail(c: &mut Ctx, api: ApiError) -> Error {
    match render(c, &api) {
        Ok(response) => Error::Halt(Box::new(response)),
        Err(error) => error,
    }
}

pub(crate) fn not_found() -> ApiError {
    ApiError::NotFound {
        message: "Not found".into(),
    }
}

/// A 422 naming the request fields that were wrong.
pub(crate) fn validation(field: &str, message: &str) -> ApiError {
    ApiError::Validation {
        message: format!("{} {message}", humanize(field)),
        fields: BTreeMap::from([(field.to_string(), vec![message.to_string()])]),
    }
}

/// A failed validation: each attribute's messages on its wire field (the one `rename` gives it,
/// else its camelCased name), and the message as Rails' full messages read, by the attribute's
/// own name.
pub(crate) fn record_invalid(errors: &campfire_db::Errors, rename: &[(&str, &str)]) -> ApiError {
    let mut fields: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (attribute, message) in &errors.0 {
        let field = rename
            .iter()
            .find(|(from, _)| from == attribute)
            .map_or_else(|| camel_case(attribute), |(_, to)| (*to).to_string());
        fields.entry(field).or_default().push(message.clone());
    }
    let message = errors
        .0
        .iter()
        .map(|(attribute, message)| format!("{} {message}", humanize(attribute)))
        .collect::<Vec<_>>()
        .join(", ");
    ApiError::Validation { message, fields }
}

fn render(c: &mut Ctx, api: &ApiError) -> Result<Response> {
    let status = StatusCode::from_u16(api.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut response = c.json(status, &ApiErrorResponse { error: api.clone() })?;
    if let ApiError::RateLimited { retry_after, .. } = api
        && *retry_after > 0
        && let Ok(value) = retry_after.to_string().parse()
    {
        response
            .headers
            .insert(axum::http::header::RETRY_AFTER, value);
    }
    Ok(response)
}

fn envelope(c: &Ctx, error: &Error) -> Option<ApiError> {
    Some(match error {
        Error::Halt(response) => return halted(c, response),
        Error::NotFound => not_found(),
        Error::InvalidAuthenticityToken(_) | Error::InvalidCrossOriginRequest => {
            ApiError::InvalidAuthenticityToken {
                message: "Can't verify CSRF token authenticity.".into(),
            }
        }
        Error::BadRequest(message) | Error::ParameterMissing(message) => ApiError::Validation {
            message: message.clone(),
            fields: BTreeMap::new(),
        },
        Error::Internal(error) => match error.downcast_ref::<campfire_db::Error>()? {
            campfire_db::Error::RecordNotFound(_) => not_found(),
            campfire_db::Error::RecordInvalid(errors) => record_invalid(errors, &[]),
            _ => return None,
        },
        _ => return None,
    })
}

/// What a before-action's halt means to the client. A JSON body is already an envelope; a bare
/// `head` (which takes the request's JSON content type, as Rails' does) is not.
fn halted(c: &Ctx, response: &Response) -> Option<ApiError> {
    let json = response
        .headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    let bare = match &response.body {
        campfire_kit::Body::Empty => true,
        campfire_kit::Body::Bytes(bytes) => bytes.is_empty(),
        _ => false,
    };
    if json && !bare {
        return None;
    }
    let status = response.status;
    Some(if status == StatusCode::UNAUTHORIZED {
        ApiError::Unauthorized {
            message: "Sign in to continue".into(),
        }
    } else if status.is_redirection() {
        // Only `request_authentication` redirects a JSON request: to the sign-in page, or to the
        // second step when the first is done.
        let location = response
            .headers
            .get(axum::http::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if location.contains("two_factor") {
            ApiError::TwoFactorRequired {
                message: "Finish two-step sign-in to continue".into(),
            }
        } else {
            ApiError::Unauthorized {
                message: "Sign in to continue".into(),
            }
        }
    } else if status == StatusCode::FORBIDDEN {
        if two_factor_setup_pending(c) {
            ApiError::TwoFactorRequired {
                message: "Set up two-step sign-in to continue".into(),
            }
        } else {
            ApiError::Forbidden {
                message: "Not allowed".into(),
            }
        }
    } else if status == StatusCode::TOO_MANY_REQUESTS {
        let retry_after = response
            .headers
            .get(axum::http::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        ApiError::RateLimited {
            message: "Too many requests".into(),
            retry_after,
        }
    } else if status == StatusCode::NOT_FOUND {
        not_found()
    } else {
        return None;
    })
}

/// `enforce_two_factor_for_restored_session`'s 403: the person must enroll before going on.
fn two_factor_setup_pending(c: &Ctx) -> bool {
    let (Some(user), Some(session)) = (concerns::current_user(c), concerns::current_session(c))
    else {
        return false;
    };
    user.requires_two_factor() && !session.two_factor_verified()
}

/// `client_message_id` → `clientMessageId`, the wire's name for a column.
fn camel_case(attribute: &str) -> String {
    let mut out = String::with_capacity(attribute.len());
    let mut upper = false;
    for character in attribute.chars() {
        if character == '_' {
            upper = true;
        } else if upper {
            out.extend(character.to_uppercase());
            upper = false;
        } else {
            out.push(character);
        }
    }
    out
}

/// `markdown_source` / `markdownSource` → `Markdown source`, as Rails' full messages read.
fn humanize(attribute: &str) -> String {
    let mut words = String::with_capacity(attribute.len() + 4);
    for (index, character) in attribute.chars().enumerate() {
        if character == '_' {
            words.push(' ');
        } else if character.is_uppercase() && index > 0 {
            words.push(' ');
            words.extend(character.to_lowercase());
        } else if index == 0 {
            words.extend(character.to_uppercase());
        } else {
            words.push(character);
        }
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_wire() {
        assert_eq!(camel_case("client_message_id"), "clientMessageId");
        assert_eq!(camel_case("body"), "body");
        assert_eq!(humanize("markdown_source"), "Markdown source");
        assert_eq!(humanize("clientMessageId"), "Client message id");
    }
}
