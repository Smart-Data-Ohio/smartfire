//! `app/controllers/sudos_controller.rb`; Google transport/token verification belongs to WS14.
use super::auth;
use crate::app::AppCtx;
use crate::concerns::{self, Before, require_current_user, session_keys, sudo};
use crate::controllers::presenters::page::retained_page;
use campfire_api_types::{SudoMethod, SudoResponse, SudoSubmission};
use campfire_db::SudoVerifier;
use campfire_kit::{Ctx, Error, Param, RateLimit, Redirect, Result, StatusCode, format, halt};
use campfire_retained::sudos;
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

pub async fn new(c: &mut Ctx) -> Result {
    before_actions(c, "sudos#new").await?;
    render_new(c, StatusCode::OK).await
}

pub async fn new_json(c: &mut Ctx) -> Result {
    c.params.insert("format", Param::Str("json".into()));
    let result = new(c).await;
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
        confirmed_json(c)
    }
    .await;
    complete(c, result)
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
                    },
                );
            }
            if matches!(response.body, campfire_kit::response::Body::Empty) {
                let body = if response.status == StatusCode::UNAUTHORIZED {
                    SudoResponse::Navigate {
                        location: c.url_for(&campfire_routes::new_session()),
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
    auth::json(c, StatusCode::OK, &SudoResponse::Confirmed { retry })
}

#[derive(Clone)]
struct GoogleJson;

/// Keep the consumed OAuth flow's response mode for the shared confirmation operation.
pub fn restore_google_response_mode(c: &mut Ctx, flow: &serde_json::Value) {
    if flow["purpose"] == "sudo" && flow["google_json"] == true {
        c.set_current(GoogleJson);
    }
}

pub fn google_return_path(c: &Ctx) -> Option<&'static str> {
    c.current::<GoogleJson>().map(|_| "/app/sudo/continue")
}

pub async fn create(c: &mut Ctx) -> Result {
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

pub async fn google(c: &mut Ctx) -> Result {
    before_actions(c, "sudos#google").await?;
    rate_limit(c).await?;
    let json_mode = sudo::json_request(c)?;
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
            c.session().insert(key, flow);
        }
        let location = response
            .get_header("location")
            .expect("Google authorization redirect")
            .to_owned();
        return auth::json(c, StatusCode::OK, &SudoResponse::Navigate { location });
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
        &campfire_routes::new_sudo(),
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
            campfire_routes::new_session()
        };
        return redirect_alert(c, &path, "Confirmation expired. Try again.");
    };
    if !auth_time.is_some_and(|time| time > (c.now().as_second() - 330) as f64) {
        return redirect_alert(
            c,
            &campfire_routes::new_sudo(),
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
            c.redirect_to(&c.url_for(path))
        } else {
            continue_after_sudo(c).await
        }
    } else {
        redirect_alert(
            c,
            &campfire_routes::new_sudo(),
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

pub async fn continue_after_sudo(c: &mut Ctx) -> Result {
    match session_keys::continue_after_sudo(c.session(), &campfire_routes::root()) {
        session_keys::SudoContinuation::Redirect(path)
        | session_keys::SudoContinuation::Origin(path) => c.redirect_to(&path),
        session_keys::SudoContinuation::Replay {
            method,
            path,
            params,
        } => {
            c.respond_to(&[&format::HTML])?;
            retained_page!(c, StatusCode::OK, |ctx| sudos::Continue {
                ctx,
                method: method.clone(),
                path: path.clone(),
                params: params.clone()
            })
            .await
        }
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
    c.flash().now("alert", alert);
    render_new(c, status).await
}

async fn render_new(c: &mut Ctx, status: StatusCode) -> Result {
    if sudo::json_request(c)? {
        let reauthentication = sudo::state(c).await?;
        return auth::json(c, status, &SudoResponse::Ready { reauthentication });
    }
    c.respond_to(&[&format::HTML])?;
    let methods = sudo::methods(c).await?;
    let password = methods.contains(&SudoMethod::Password);
    let totp = methods.contains(&SudoMethod::Totp);
    let google = methods.contains(&SudoMethod::Google);
    retained_page!(c, status, |ctx| sudos::New {
        ctx,
        password,
        totp,
        google
    })
    .await
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
