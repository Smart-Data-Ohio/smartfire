//! `app/controllers/sudos_controller.rb`; Google transport/token verification belongs to WS14.
use super::auth;
use crate::app::AppCtx;
use crate::concerns::{self, Before, require_current_user, session_keys, sudo};
use campfire_api_types::{SudoGoogleStart, SudoResponse, SudoSubmission};
use campfire_db::SudoVerifier;
use campfire_kit::{Ctx, Error, Param, RateLimit, Redirect, Result, StatusCode, format, halt};
use jiff::SignedDuration;
use rusqlite::OptionalExtension;

async fn before_actions(c: &mut Ctx, endpoint: &'static str) -> Result<()> {
    let mode = if sudo::json_request(c)? {
        auth::ResponseMode::Json
    } else {
        auth::ResponseMode::Html
    };
    auth::before_actions(c, endpoint, Before::default(), mode).await
}

/// The SPA's confirmation page (`/app/sudo/new`), which reads `GET /api/v1/sudo`.
const SPA_SUDO: &str = "/app/sudo/new";

/// `GET /sudo/new`: a page navigation goes to the SPA's confirmation page; a JSON request gets
/// the confirmation methods.
pub async fn new(c: &mut Ctx) -> Result {
    before_actions(c, "sudos#new").await?;
    render_new(c, StatusCode::OK).await
}

/// `GET /api/v1/sudo`: `confirmed` while this session's confirmation is fresh, else `ready` with
/// the methods. Either names the request the gate holds for this session (`retry`) as a peek:
/// reading it never consumes the held request, the Google attempt record or the flow. Only the
/// continuation acts on (and so consumes) what is held.
pub async fn new_json(c: &mut Ctx) -> Result {
    c.params.insert("format", Param::Str("json".into()));
    let result = async {
        before_actions(c, "sudos#new").await?;
        let now = c.now();
        if session_keys::sudo_verified(c.session(), now) {
            let retry = sudo::retry(c);
            let session = sudo::session_id(c);
            return auth::json(
                c,
                StatusCode::OK,
                &SudoResponse::Confirmed {
                    retry,
                    attempt: None,
                    session,
                },
            );
        }
        render_new(c, StatusCode::OK).await
    }
    .await;
    complete(c, result)
}

pub async fn create_json(c: &mut Ctx) -> Result {
    c.params.insert("format", Param::Str("json".into()));
    let result = create(c).await;
    complete(c, result)
}

pub async fn google_json(c: &mut Ctx) -> Result {
    c.params.insert("format", Param::Str("json".into()));
    let result = google(c).await;
    complete(c, result)
}

pub async fn continue_json(c: &mut Ctx) -> Result {
    c.params.insert("format", Param::Str("json".into()));
    let result = async {
        before_actions(c, "sudos#continue").await?;
        let now = c.now();
        if !session_keys::sudo_verified(c.session(), now) {
            return render_new(c, StatusCode::FORBIDDEN).await;
        }
        // The SPA asks by the attempt id it started Google with: only this session's finished
        // attempt is named, and it answers the same way until acknowledged or expired.
        if let Some(attempt) = attempt_param(c)
            && let Some(retry) = sudo::attempt_retry(c, &attempt)?
        {
            c.no_store();
            return auth::json(
                c,
                StatusCode::OK,
                &SudoResponse::Confirmed {
                    retry,
                    attempt: Some(attempt),
                    session: sudo::session_id(c),
                },
            );
        }
        confirmed_json(c)
    }
    .await;
    complete(c, result)
}

/// `DELETE /api/v1/sudo/continue?attempt=…`: the SPA has replayed (or given up on) what the
/// attempt confirmed, so its record goes. Another attempt's record stays.
pub async fn acknowledge_json(c: &mut Ctx) -> Result {
    c.params.insert("format", Param::Str("json".into()));
    let result = async {
        before_actions(c, "sudos#continue").await?;
        if let Some(attempt) = attempt_param(c) {
            sudo::acknowledge_attempt(c, &attempt);
        }
        c.no_store();
        Ok(c.head(StatusCode::NO_CONTENT))
    }
    .await;
    complete(c, result)
}

fn attempt_param(c: &Ctx) -> Option<String> {
    c.query_params
        .get("attempt")
        .and_then(|param| param.to_s())
        .filter(|attempt| sudo::valid_attempt(attempt))
}

fn complete(c: &mut Ctx, result: Result) -> Result {
    match result {
        Err(Error::Halt(response)) => {
            if let Some(location) = response.get_header("location") {
                return auth::json(
                    c,
                    StatusCode::OK,
                    &SudoResponse::Navigate {
                        location: location.into(),
                        session: None,
                    },
                );
            }
            if matches!(response.body, campfire_kit::response::Body::Empty) {
                let body = if response.status == StatusCode::UNAUTHORIZED {
                    SudoResponse::Navigate {
                        location: c.url_for("/app/session/new"),
                        session: None,
                    }
                } else {
                    SudoResponse::Error {
                        message: "This request is not allowed.".into(),
                    }
                };
                return auth::json(c, response.status, &body);
            }
            Ok(*response)
        }
        result => result,
    }
}

fn confirmed_json(c: &mut Ctx) -> Result {
    let retry = sudo::retry(c);
    session_keys::continue_after_sudo(c.session(), &campfire_routes::root());
    auth::json(
        c,
        StatusCode::OK,
        &SudoResponse::Confirmed {
            retry,
            attempt: None,
            session: sudo::session_id(c),
        },
    )
}

/// A Google confirmation the SPA started: the attempt id it named, if any, and the signed-in
/// session that started it.
#[derive(Clone)]
struct GoogleJson {
    attempt: Option<String>,
    session_id: Option<i64>,
}

/// Keep the consumed OAuth flow's response mode for the shared confirmation operation.
pub fn restore_google_response_mode(c: &mut Ctx, flow: &serde_json::Value) {
    if flow["purpose"] == "sudo" && flow["google_json"] == true {
        let attempt = flow["attempt"]
            .as_str()
            .filter(|attempt| sudo::valid_attempt(attempt))
            .map(str::to_owned);
        let session_id = flow["session_id"].as_i64();
        c.set_current(GoogleJson {
            attempt,
            session_id,
        });
    }
}

pub fn google_return_path(c: &Ctx) -> Option<&'static str> {
    c.current::<GoogleJson>().map(|_| "/app/sudo/continue")
}

/// `POST /sudo`. A form posted from the retained confirmation page goes, unread, to the SPA's
/// (`/app/sudo/new`): nothing is checked or counted. A JSON request confirms.
pub async fn create(c: &mut Ctx) -> Result {
    if !sudo::json_request(c)? {
        return concerns::redirect_to_signed_out_spa(c, SPA_SUDO, StatusCode::SEE_OTHER);
    }
    before_actions(c, "sudos#create").await?;
    rate_limit(c).await?;
    if sudo::json_request(c)? {
        let input = match auth::body::<SudoSubmission>(c).await {
            Ok(input) => input,
            Err(Error::Halt(_)) => {
                return reject(
                    c,
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "The request body isn't valid.",
                )
                .await;
            }
            Err(error) => return Err(error),
        };
        let (verifier, key, value) = match input {
            SudoSubmission::Password { password } => ("password", "password", password),
            SudoSubmission::Totp { code } => ("totp", "totp_code", code),
        };
        c.params.insert("verifier", Param::Str(verifier.into()));
        c.params.insert(key, Param::Str(value));
    }
    let verifier = c
        .params
        .get("verifier")
        .map(|param| param.to_s().unwrap_or_else(|| "unsupported".into()))
        .filter(|name| !name.chars().all(char::is_whitespace))
        .unwrap_or_else(|| "password".into());
    let (verified, verifier) = match verifier.as_str() {
        "password" => {
            let user = require_current_user(c)?.clone();
            let password = c
                .params
                .get("password")
                .and_then(|p| p.to_s())
                .unwrap_or_default();
            (
                Some(
                    tokio::task::spawn_blocking(move || user.authenticate(&password))
                        .await
                        .map_err(Error::internal)?,
                ),
                SudoVerifier::Password,
            )
        }
        "totp" => {
            let user_id = require_current_user(c)?.id;
            let code = c
                .params
                .get("totp_code")
                .and_then(|p| p.to_s())
                .unwrap_or_default();
            let secrets = c.app().secrets.clone();
            let app = c.app().clone();
            let verified = c
                .app()
                .db
                .write(move |tx| app.sudo.verify_totp(tx, user_id, &secrets, &code))
                .await
                .map_err(Error::internal)?;
            (verified, SudoVerifier::Totp)
        }
        _ => {
            return reject(
                c,
                StatusCode::UNPROCESSABLE_ENTITY,
                "That confirmation method is not available.",
            )
            .await;
        }
    };
    let Some(verified) = verified else {
        return reject(
            c,
            StatusCode::UNPROCESSABLE_ENTITY,
            "That confirmation method is not available.",
        )
        .await;
    };
    if verified {
        let now = c.now();
        session_keys::mark_sudo_verified(c.session(), now);
    }
    let audit = sudo::audit(c)?;
    c.app()
        .db
        .write(move |tx| audit.sudo_confirmation(tx, verifier, verified, false))
        .await
        .map_err(Error::internal)?;
    if verified {
        if sudo::json_request(c)? {
            confirmed_json(c)
        } else {
            continue_after_sudo(c).await
        }
    } else {
        reject(
            c,
            StatusCode::UNAUTHORIZED,
            "Confirmation failed. Try again.",
        )
        .await
    }
}

/// `POST /sudo/google`. As [`create`]: only a JSON request starts a Google confirmation.
pub async fn google(c: &mut Ctx) -> Result {
    if !sudo::json_request(c)? {
        return concerns::redirect_to_signed_out_spa(c, SPA_SUDO, StatusCode::SEE_OTHER);
    }
    before_actions(c, "sudos#google").await?;
    rate_limit(c).await?;
    let json_mode = sudo::json_request(c)?;
    let attempt = if json_mode {
        let start = auth::body_or_default::<SudoGoogleStart>(c).await?;
        if start
            .attempt
            .as_deref()
            .is_some_and(|attempt| !sudo::valid_attempt(attempt))
        {
            return reject(
                c,
                StatusCode::UNPROCESSABLE_ENTITY,
                "The request body isn't valid.",
            )
            .await;
        }
        start.attempt
    } else {
        None
    };
    let user_id = require_current_user(c)?.id;
    if let Some(google) = c.app().sudo.google()
        && linked_subject(c, user_id).await?.is_some()
    {
        let response = google.start(c, user_id)?;
        if !json_mode {
            return Ok(response);
        }
        let key = crate::integrations::google::sign_in::FLOW_SESSION_KEY;
        if let Some(mut flow) = c.session().get(key).cloned() {
            flow["google_json"] = serde_json::Value::Bool(true);
            flow["session_id"] = serde_json::json!(sudo::session_id(c));
            if let Some(attempt) = attempt {
                flow["attempt"] = serde_json::Value::String(attempt);
            }
            c.session().insert(key, flow);
        }
        let location = response
            .get_header("location")
            .expect("Google authorization redirect")
            .to_owned();
        let session = sudo::session_id(c);
        return auth::json(c, StatusCode::OK, &SudoResponse::Navigate { location, session });
    }
    if json_mode {
        return reject(
            c,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Google confirmation is not available for your account.",
        )
        .await;
    }
    redirect_alert(
        c,
        SPA_SUDO,
        "Google confirmation is not available for your account.",
    )
}

/// WS14 calls this only AFTER consuming/verifying the one-use browser flow and checking the
/// ID token's signature, audience, issuer, expiry, domain and nonce. `flow_user_id` comes from
/// that verified server-side flow, never request params. The shared-session user and current
/// linked subject are checked here, and even a verified token needs a fresh numeric auth_time.
#[allow(dead_code)]
pub async fn finish_google(
    c: &mut Ctx,
    flow_user_id: i64,
    verified_subject: &str,
    auth_time: Option<f64>,
) -> Result {
    let Some(user_id) = concerns::current_user(c)
        .map(|user| user.id)
        .filter(|id| *id == flow_user_id)
    else {
        let path = if concerns::signed_in(c) {
            campfire_routes::user_profile()
        } else {
            "/app/session/new".to_owned()
        };
        return redirect_alert(c, &path, "Confirmation expired. Try again.");
    };
    // The SPA's flow belongs to the session that started it: a sign-in since ends it.
    if c
        .current::<GoogleJson>()
        .is_some_and(|json| json.session_id.is_none() || json.session_id != sudo::session_id(c))
    {
        return redirect_alert(
            c,
            SPA_SUDO,
            "Confirmation expired. Try again.",
        );
    }
    if !auth_time.is_some_and(|time| time > (c.now().as_second() - 330) as f64) {
        return redirect_alert(
            c,
            SPA_SUDO,
            "Google confirmation failed. Try again.",
        );
    }
    let matches = !verified_subject.trim().is_empty()
        && linked_subject(c, user_id).await?.as_deref() == Some(verified_subject);
    if matches {
        let now = c.now();
        session_keys::mark_sudo_verified(c.session(), now);
    }
    let audit = sudo::audit(c)?;
    c.app()
        .db
        .write(move |tx| audit.sudo_confirmation(tx, SudoVerifier::Google, matches, !matches))
        .await
        .map_err(Error::internal)?;
    if matches {
        if let Some(path) = google_return_path(c) {
            if let Some(attempt) = c.current::<GoogleJson>().and_then(|json| json.attempt.clone()) {
                sudo::record_attempt(c, &attempt)?;
            }
            c.redirect_to(&c.url_for(path))
        } else {
            continue_after_sudo(c).await
        }
    } else {
        redirect_alert(
            c,
            SPA_SUDO,
            "That Google account is not the one linked to your account.",
        )
    }
}

async fn linked_subject(c: &Ctx, user_id: i64) -> Result<Option<String>> {
    c.app()
        .db
        .read(move |conn| {
            Ok(conn
                .query_row(
                    "SELECT subject FROM google_identities WHERE user_id = ? LIMIT 1",
                    [user_id],
                    |row| row.get(0),
                )
                .optional()?)
        })
        .await
        .map_err(Error::internal)
}

fn redirect_alert(c: &mut Ctx, path: &str, alert: &str) -> Result {
    let path = google_return_path(c).unwrap_or(path);
    let location = c.url_for(path);
    c.redirect_to_with(
        &location,
        Redirect {
            alert: Some(alert.into()),
            ..Redirect::default()
        },
    )
}

/// Where a confirmation goes next: the page that asked for it, or, for a held form submission,
/// home with a prompt to submit it again. Nothing is replayed.
pub async fn continue_after_sudo(c: &mut Ctx) -> Result {
    match session_keys::continue_after_sudo(c.session(), &campfire_routes::root()) {
        session_keys::SudoContinuation::Redirect(path)
        | session_keys::SudoContinuation::Origin(path) => c.redirect_to(&path),
        session_keys::SudoContinuation::Replay { .. } => c.redirect_to_with(
            &c.url_for(&campfire_routes::root()),
            Redirect {
                alert: Some("Confirmed. Try that again.".into()),
                ..Redirect::default()
            },
        ),
    }
}

async fn reject(c: &mut Ctx, status: StatusCode, alert: &str) -> Result {
    if sudo::json_request(c)? {
        return auth::json(
            c,
            status,
            &SudoResponse::Error {
                message: alert.into(),
            },
        );
    }
    let location = c.url_for(SPA_SUDO);
    c.redirect_to_with(
        &location,
        Redirect {
            status: Some(StatusCode::SEE_OTHER),
            alert: Some(alert.into()),
            ..Redirect::default()
        },
    )
}

async fn render_new(c: &mut Ctx, status: StatusCode) -> Result {
    if sudo::json_request(c)? {
        let reauthentication = sudo::state(c).await?;
        let session = sudo::session_id(c);
        return auth::json(
            c,
            status,
            &SudoResponse::Ready {
                reauthentication,
                session,
            },
        );
    }
    c.respond_to(&[&format::HTML])?;
    concerns::redirect_to_signed_out_spa(c, SPA_SUDO, StatusCode::FOUND)
}

async fn rate_limit(c: &mut Ctx) -> Result<()> {
    if c.rate_limited(
        &RateLimit::new("sudos", 10, SignedDuration::from_mins(3)),
        None,
    )? {
        return halt(
            reject(
                c,
                StatusCode::TOO_MANY_REQUESTS,
                "Too many confirmation attempts. Try again in a few minutes.",
            )
            .await?,
        );
    }
    Ok(())
}
