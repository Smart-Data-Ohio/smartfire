//! `app/controllers/sudos_controller.rb`; Google transport/token verification belongs to WS14.
use crate::app::AppCtx;
use crate::concerns::{self, Before, require_current_user, session_keys, sudo};
use crate::controllers::presenters::page::framed_page;
use campfire_db::SudoVerifier;
use campfire_kit::{Ctx, Error, RateLimit, Redirect, Result, StatusCode, format, halt};
use campfire_views::sudos;
use jiff::SignedDuration;
use rusqlite::OptionalExtension;

pub async fn new(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    render_new(c, StatusCode::OK).await
}

pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    rate_limit(c).await?;
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
        continue_after_sudo(c).await
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
    concerns::before_actions(c, Before::default()).await?;
    rate_limit(c).await?;
    let user_id = require_current_user(c)?.id;
    if let Some(google) = c.app().sudo.google()
        && linked_subject(c, user_id).await?.is_some()
    {
        return google.start(c, user_id);
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
        continue_after_sudo(c).await
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
            framed_page!(c, StatusCode::OK, |ctx| sudos::Continue {
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
    c.flash().now("alert", alert);
    render_new(c, status).await
}

async fn render_new(c: &mut Ctx, status: StatusCode) -> Result {
    c.respond_to(&[&format::HTML])?;
    let user = require_current_user(c)?.clone();
    let password = user
        .password_digest
        .as_ref()
        .is_some_and(|digest| !digest.trim().is_empty());
    let app = c.app().clone();
    let totp = c
        .app()
        .db
        .read(move |conn| app.sudo.verifier_available(conn, "totp", user.id))
        .await
        .map_err(Error::internal)?;
    let google = c.app().sudo.google().is_some()
        && linked_subject(c, require_current_user(c)?.id)
            .await?
            .is_some();
    framed_page!(c, status, |ctx| sudos::New {
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
