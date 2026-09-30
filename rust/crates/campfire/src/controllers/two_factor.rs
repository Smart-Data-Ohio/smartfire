//! `app/controllers/two_factor`: domain writes are transactional; page rendering stays separate.
use crate::app::AppCtx;
use crate::concerns::{self, Before, current_session, require_current_user};
use crate::controllers::presenters::page::framed_page;
use campfire_db::models::audit_log::{Actor, AuditLog, Context, NewAuditLog, Target};
use campfire_db::{TwoFactorCredential, TwoFactorRememberedDevice, User};
use campfire_kit::{
    Cookie, Ctx, Error, RateLimit, Redirect, Result, SameSite, StatusCode, format, halt,
};
use campfire_views::two_factor;
use jiff::SignedDuration;
use rails_compat::{ar_encryption::ArEncryption, totp};
use serde_json::json;

pub fn audit_context(c: &Ctx) -> Result<Context> {
    Ok(Context {
        actor: concerns::current_user(c).map(Actor::from),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_string),
    })
}
fn scalar(c: &Ctx, name: &'static str) -> String {
    c.params
        .get(name)
        .and_then(|p| p.to_s())
        .unwrap_or_default()
}
fn profile(c: &mut Ctx) -> Result {
    c.redirect_to(&c.url_for("/users/me/profile"))
}
/// IP and per-user limits share the app's WS4 in-process store, as Rails does.
fn limited(
    c: &mut Ctx,
    scope: &'static str,
    name: &'static str,
    user_id: Option<i64>,
) -> Result<bool> {
    let ip = RateLimit::new(scope, 10, SignedDuration::from_mins(3));
    let user = RateLimit::new(scope, 10, SignedDuration::from_mins(15)).named(name);
    Ok(c.rate_limited(&ip, None)?
        || c.rate_limited(
            &user,
            Some(&user_id.map(|id| id.to_string()).unwrap_or_default()),
        )?)
}
fn no_store(c: &mut Ctx) {
    c.no_store();
    c.headers.insert("pragma", "no-cache".parse().unwrap());
}

pub async fn setup_show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    ensure_human(c)?;
    no_store(c);
    if enabled(c).await? {
        return profile(c);
    }
    render_setup(c, StatusCode::OK).await
}

pub async fn setup_create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let user = require_current_user(c)?.clone();
    if limited(c, "two_factor/setups", "per-user", Some(user.id))? {
        no_store(c);
        if enabled(c).await? {
            return profile(c);
        }
        c.flash()
            .now("alert", "Too many attempts. Try again in a few minutes.");
        return render_setup(c, StatusCode::TOO_MANY_REQUESTS).await;
    }
    ensure_human(c)?;
    no_store(c);
    let session_id = current_session(c)
        .ok_or(Error::Status(StatusCode::UNAUTHORIZED))?
        .id;
    let code = scalar(c, "code");
    let secrets = c.app().secrets.clone();
    let audit = audit_context(c)?;
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            crate::authentication::enroll(
                tx,
                &user,
                session_id,
                &ArEncryption::new(&secrets),
                &code,
                &audit,
            )
        })
        .await
        .map_err(Error::internal)?;
    match outcome {
        crate::authentication::Enrollment::Enabled => profile(c),
        crate::authentication::Enrollment::Wrong => {
            c.flash().now(
                "alert",
                "That code didn't work. Check your authenticator app and try again.",
            );
            render_setup(c, StatusCode::UNPROCESSABLE_ENTITY).await
        }
        crate::authentication::Enrollment::Confirmed {
            codes,
            signed_out,
            session,
        } => {
            c.set_current(concerns::CurrentSession(*session));
            let continue_url = concerns::post_authenticating_url(c);
            render_backups(c, codes, signed_out, continue_url).await
        }
    }
}
fn ensure_human(c: &mut Ctx) -> Result<()> {
    if !require_current_user(c)?.requires_two_factor() {
        return halt(c.redirect_to(&c.url_for("/"))?);
    }
    Ok(())
}
async fn enabled(c: &Ctx) -> Result<bool> {
    let id = require_current_user(c)?.id;
    c.app()
        .db
        .read(move |conn| Ok(TwoFactorCredential::for_user(conn, id)?.is_some_and(|v| v.enabled())))
        .await
        .map_err(Error::internal)
}
async fn render_setup(c: &mut Ctx, status: StatusCode) -> Result {
    c.respond_to(&[&format::HTML])?;
    let session_id = current_session(c)
        .ok_or(Error::Status(StatusCode::UNAUTHORIZED))?
        .id;
    let email = require_current_user(c)?
        .email_address
        .clone()
        .unwrap_or_default();
    let secrets = c.app().secrets.clone();
    let secret = c
        .app()
        .db
        .write(move |tx| {
            crate::authentication::setup_secret(tx, &ArEncryption::new(&secrets), session_id)
        })
        .await
        .map_err(Error::internal)?;
    let uri = totp::provisioning_uri(&secret, &email);
    let qr = super::qr_code::two_factor_svg(&uri)
        .ok_or_else(|| Error::internal(anyhow::anyhow!("provisioning URI too large")))?;
    let key = secret
        .chars()
        .collect::<Vec<_>>()
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ");
    framed_page!(c, status, |ctx| two_factor::Setup {
        ctx,
        key: key.clone(),
        qr: qr.clone()
    })
    .await
}
async fn render_backups(
    c: &mut Ctx,
    codes: Vec<String>,
    signed_out: usize,
    continue_url: String,
) -> Result {
    c.respond_to(&[&format::HTML])?;
    framed_page!(c, StatusCode::OK, |ctx| two_factor::BackupCodes {
        ctx,
        codes: codes.clone(),
        signed_out,
        continue_url: continue_url.clone()
    })
    .await
}

/// Shared password/transfer/WS14 Google first-factor seam. Pending state grants no session.
pub async fn begin_session_for(c: &mut Ctx, user: User, method: &str) -> Result {
    let return_url = concerns::post_authenticating_url(c);
    let user_id = user.id;
    let enabled = c
        .app()
        .db
        .read(move |conn| user_enabled(conn, user_id))
        .await
        .map_err(Error::internal)?;
    if enabled {
        let token = c.cookies.signed("two_factor_remember");
        let remembered = c
            .app()
            .db
            .write(move |tx| {
                TwoFactorRememberedDevice::find_valid(tx, token.as_deref(), Some(user_id))
            })
            .await
            .map_err(Error::internal)?
            .is_some();
        if remembered {
            concerns::start_new_verified_session_for(c, user.clone()).await?;
            sign_in_audit(c, user, method.into(), Some("remembered_device")).await?;
        } else {
            let now = c.now();
            c.session()
                .insert(concerns::session_keys::RETURN_TO_KEY, return_url);
            concerns::session_keys::stash_two_factor_pending(c.session(), user_id, method, now);
            return c.redirect_to(&c.url_for("/two_factor_challenge"));
        }
    } else {
        concerns::start_new_session_for(c, user.clone()).await?;
        sign_in_audit(c, user, method.into(), None).await?;
    }
    c.redirect_to(&return_url)
}
fn user_enabled(conn: &campfire_db::Connection, user_id: i64) -> campfire_db::Result<bool> {
    Ok(TwoFactorCredential::for_user(conn, user_id)?.is_some_and(|v| v.enabled()))
}
async fn sign_in_audit(
    c: &Ctx,
    user: User,
    method: String,
    factor: Option<&'static str>,
) -> Result<()> {
    let context = audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            crate::authentication::record_sign_in(tx, &user, &method, factor, &context)
        })
        .await
        .map_err(Error::internal)
}
async fn pending(c: &mut Ctx) -> Result<User> {
    concerns::restore_authentication(c).await?;
    if concerns::signed_in(c) {
        return halt(c.redirect_to(&c.url_for("/"))?);
    }
    if let Some(user) = concerns::two_factor_pending_user(c).await? {
        let id = user.id;
        if c.app()
            .db
            .read(move |conn| user_enabled(conn, id))
            .await
            .map_err(Error::internal)?
        {
            return Ok(user);
        }
    }
    concerns::session_keys::clear_two_factor_pending(c.session());
    halt(c.redirect_to(&c.url_for("/session/new"))?)
}
pub async fn challenge_show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    pending(c).await?;
    no_store(c);
    render_challenge(c, StatusCode::OK).await
}
pub async fn challenge_create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    let now = c.now();
    let id = concerns::session_keys::two_factor_pending_user_id(c.session(), now);
    if limited(c, "two_factor/challenges", "per-user", id)? {
        no_store(c);
        let user = concerns::two_factor_pending_user(c).await?;
        let method = concerns::session_keys::two_factor_pending_method(c.session());
        let context = audit_context(c)?;
        c.app()
            .db
            .write(move |tx| {
                AuditLog::record(
                    tx,
                    NewAuditLog {
                        action: "sign_in.two_factor.failure".into(),
                        actor: user.as_ref().map(Actor::from),
                        changes: Some(json!({"method":method,"rate_limited":true})),
                        ..Default::default()
                    },
                    &context,
                )?;
                Ok(())
            })
            .await
            .map_err(Error::internal)?;
        c.flash()
            .now("alert", "Too many attempts. Try again in a few minutes.");
        return render_challenge(c, StatusCode::TOO_MANY_REQUESTS).await;
    }
    let user = pending(c).await?;
    no_store(c);
    let user_id = user.id;
    let code = scalar(c, "code");
    let method = concerns::session_keys::two_factor_pending_method(c.session());
    let audit_method = method.clone();
    let mut context = audit_context(c)?;
    context.actor = Some(Actor::from(&user));
    let secrets = c.app().secrets.clone();
    let notify = c.app().mail.config.two_factor_configured();
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            crate::authentication::challenge(
                tx,
                user_id,
                &ArEncryption::new(&secrets),
                &code,
                &audit_method,
                &context,
                notify,
            )
        })
        .await
        .map_err(Error::internal)?;
    match outcome {
        crate::authentication::Challenge::Gone => {
            concerns::session_keys::clear_two_factor_pending(c.session());
            c.redirect_to(&c.url_for("/session/new"))
        }
        crate::authentication::Challenge::Accepted(factor) => {
            concerns::session_keys::clear_two_factor_pending(c.session());
            concerns::start_new_verified_session_for(c, user.clone()).await?;
            if c.param_str("remember_device") == Some("1") {
                remember_device(c, user_id).await?;
            }
            sign_in_audit(c, user, method, Some(factor)).await?;
            let location = concerns::post_authenticating_url(c);
            c.redirect_to(&location)
        }
        crate::authentication::Challenge::Wrong => {
            c.flash().now(
                "alert",
                "That code didn't work. Check your authenticator app or try a backup code.",
            );
            render_challenge(c, StatusCode::UNPROCESSABLE_ENTITY).await
        }
        crate::authentication::Challenge::Locked(minutes) => {
            c.flash().now(
                "alert",
                format!(
                    "Too many wrong codes. Try again in {minutes} {}.",
                    if minutes == 1 { "minute" } else { "minutes" }
                ),
            );
            render_challenge(c, StatusCode::TOO_MANY_REQUESTS).await
        }
    }
}
async fn render_challenge(c: &mut Ctx, status: StatusCode) -> Result {
    c.respond_to(&[&format::HTML])?;
    framed_page!(c, status, |ctx| two_factor::Challenge { ctx }).await
}
async fn remember_device(c: &mut Ctx, user_id: i64) -> Result<()> {
    let agent = c.request.user_agent().map(str::to_string);
    let ip = c.request.remote_ip()?.to_string();
    let (device, token) = c
        .app()
        .db
        .write(move |tx| {
            TwoFactorRememberedDevice::create_for(tx, user_id, agent.as_deref(), Some(&ip))
        })
        .await
        .map_err(Error::internal)?;
    c.cookies.set_signed(
        "two_factor_remember",
        Cookie::new(token)
            .expires(device.expires_at.jiff())
            .httponly()
            .secure()
            .same_site(Some(SameSite::Lax)),
    )
}

/// Self-service needs a verified session even before global enrollment enforcement lands.
async fn require_verified(c: &mut Ctx) -> Result<()> {
    if current_session(c).is_some_and(|s| s.two_factor_verified()) {
        return Ok(());
    }
    if enabled(c).await? {
        concerns::terminate_current_session(c).await?;
        if c.format()?
            .is_some_and(|f| f.symbol == "html" || f.string.contains("html"))
        {
            return halt(c.redirect_to_with(
                &c.url_for("/session/new"),
                Redirect {
                    alert: Some("Sign in again to verify two-step sign-in.".into()),
                    ..Default::default()
                },
            )?);
        }
        halt(concerns::head(StatusCode::UNAUTHORIZED))
    } else {
        if c.format()?
            .is_some_and(|f| f.symbol == "html" || f.string.contains("html"))
        {
            return halt(c.redirect_to(&c.url_for("/two_factor_setup"))?);
        }
        halt(concerns::head(StatusCode::FORBIDDEN))
    }
}
async fn reauthenticated(c: &mut Ctx, user: &User) -> Result<bool> {
    if c.params.get("reauth").is_some_and(|p| p.is_present()) {
        let value = scalar(c, "reauth");
        if value.chars().all(char::is_whitespace) {
            return Ok(false);
        }
        let length = value.chars().count();
        if (6..=10).contains(&length)
            && value
                .chars()
                .all(|ch| ch.is_ascii_digit() || totp::ruby_code_whitespace(ch))
        {
            let id = user.id;
            let code = value.clone();
            let secrets = c.app().secrets.clone();
            if c.app()
                .db
                .write(move |tx| {
                    match TwoFactorCredential::for_user(tx.conn(), id)?.filter(|c| c.enabled()) {
                        Some(mut credential) => {
                            credential.verify_code(tx, &ArEncryption::new(&secrets), &code)
                        }
                        None => Ok(false),
                    }
                })
                .await
                .map_err(Error::internal)?
            {
                return Ok(true);
            }
        }
        let user = user.clone();
        tokio::task::spawn_blocking(move || user.authenticate(&value))
            .await
            .map_err(Error::internal)
    } else {
        let now = c.now();
        Ok(concerns::session_keys::consume_google_reauthentication(
            c.session(),
            now,
        ))
    }
}
fn refuse_reauthentication(c: &mut Ctx, user: &User) -> Result {
    let alert = if user.password_digest.as_ref().is_none_or(|s| s.is_empty()) {
        "Enter your authenticator code or confirm with Google to continue."
    } else {
        "Enter your authenticator code or password to continue."
    };
    c.redirect_to_with(
        &c.url_for("/users/me/profile"),
        Redirect {
            alert: Some(alert.into()),
            ..Default::default()
        },
    )
}
fn rate_rejection(c: &mut Ctx) -> Result {
    c.redirect_to_with(
        &c.url_for("/users/me/profile"),
        Redirect {
            alert: Some("Too many attempts. Try again in a few minutes.".into()),
            ..Default::default()
        },
    )
}
pub async fn backup_create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    require_verified(c).await?;
    let user = require_current_user(c)?.clone();
    if limited(c, "two_factor/backup_codes", "per-user", Some(user.id))? {
        return rate_rejection(c);
    }
    no_store(c);
    if !enabled(c).await? {
        return c.redirect_to(&c.url_for("/two_factor_setup"));
    }
    if !reauthenticated(c, &user).await? {
        return refuse_reauthentication(c, &user);
    }
    let context = audit_context(c)?;
    let codes = c
        .app()
        .db
        .write(move |tx| crate::authentication::regenerate_backups(tx, &user, &context))
        .await
        .map_err(Error::internal)?;
    render_backups(c, codes, 0, c.url_for("/users/me/profile")).await
}
pub async fn setup_destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    require_verified(c).await?;
    let user = require_current_user(c)?.clone();
    if limited(c, "two_factor/setups", "per-user-destroy", Some(user.id))? {
        return rate_rejection(c);
    }
    ensure_human(c)?;
    if !enabled(c).await? {
        return c.redirect_to(&c.url_for("/two_factor_setup"));
    }
    if !reauthenticated(c, &user).await? {
        return refuse_reauthentication(c, &user);
    }
    let context = audit_context(c)?;
    let session = current_session(c)
        .cloned()
        .ok_or(Error::Status(StatusCode::UNAUTHORIZED))?;
    let session = c
        .app()
        .db
        .write(move |tx| crate::authentication::disable(tx, &user, session, &context))
        .await
        .map_err(Error::internal)?;
    c.set_current(concerns::CurrentSession(session));
    c.cookies.delete("two_factor_remember");
    c.redirect_to_with(
        &c.url_for("/two_factor_setup"),
        Redirect {
            notice: Some("Two-step sign-in is off. Set it up again to keep signing in.".into()),
            ..Default::default()
        },
    )
}
pub async fn device_destroy(c: &mut Ctx) -> Result {
    revoke_devices(c, false).await
}
pub async fn device_destroy_all(c: &mut Ctx) -> Result {
    revoke_devices(c, true).await
}
async fn revoke_devices(c: &mut Ctx, all: bool) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    require_verified(c).await?;
    let user = require_current_user(c)?.clone();
    if limited(
        c,
        "two_factor/remembered_devices",
        "per-user",
        Some(user.id),
    )? {
        return rate_rejection(c);
    }
    if !reauthenticated(c, &user).await? {
        return refuse_reauthentication(c, &user);
    }
    let id = c.param_str("id").and_then(concerns::cast_integer);
    let context = audit_context(c)?;
    c.app()
        .db
        .write(move |tx| crate::authentication::revoke_devices(tx, &user, id, all, &context))
        .await
        .map_err(Error::internal)?;
    let notice = if all {
        "All devices forgotten. Every browser will ask for a code at next sign-in."
    } else {
        "Device forgotten. It will ask for a code at next sign-in."
    };
    c.redirect_to_with(
        &c.url_for("/users/me/profile"),
        Redirect {
            notice: Some(notice.into()),
            ..Default::default()
        },
    )
}

pub async fn reauthentication_create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    require_verified(c).await?;
    let id = require_current_user(c)?.id;
    if limited(c, "two_factor/reauthentications", "per-user", Some(id))? {
        return rate_rejection(c);
    }
    if let Some(adapter) = c.app().two_factor.google()
        && linked_subject(c, id).await?.is_some()
    {
        return adapter.start(c, id);
    }
    reauth_alert(c, "Google confirmation needs a linked Google account.")
}
async fn linked_subject(c: &Ctx, id: i64) -> Result<Option<String>> {
    use rusqlite::OptionalExtension;
    c.app()
        .db
        .read(move |conn| {
            Ok(conn
                .query_row(
                    "SELECT subject FROM google_identities WHERE user_id=? LIMIT 1",
                    [id],
                    |r| r.get(0),
                )
                .optional()?)
        })
        .await
        .map_err(Error::internal)
}
fn reauth_alert(c: &mut Ctx, alert: &str) -> Result {
    c.redirect_to_with(
        &c.url_for("/users/me/profile"),
        Redirect {
            alert: Some(alert.into()),
            ..Default::default()
        },
    )
}
/// Trusted callback only: WS14 has consumed the flow and verified the signature, audience,
/// issuer, expiry/domain/nonce. flow_user_id belongs to that flow, never raw request params.
#[allow(dead_code)]
pub async fn finish_google_reauthentication(
    c: &mut Ctx,
    flow_user_id: i64,
    verified_subject: &str,
    auth_time: Option<f64>,
) -> Result {
    let Some(user) = concerns::current_user(c)
        .cloned()
        .filter(|user| user.id == flow_user_id)
    else {
        let path = if concerns::signed_in(c) {
            "/users/me/profile"
        } else {
            "/session/new"
        };
        return c.redirect_to_with(
            &c.url_for(path),
            Redirect {
                alert: Some("Google confirmation expired. Try again.".into()),
                ..Default::default()
            },
        );
    };
    if !auth_time.is_some_and(|at| at > (c.now().as_second() - 330) as f64) {
        return reauth_alert(c, "Google confirmation failed. Try again.");
    }
    if linked_subject(c, user.id).await?.as_deref() != Some(verified_subject) {
        return reauth_alert(
            c,
            "That Google account is not linked here. Confirm with the Google account you sign in with.",
        );
    }
    let now = c.now();
    concerns::session_keys::mark_reauthenticated(c.session(), now);
    let context = audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "two_factor.reauthenticate".into(),
                    target: Some(Target::from(&user)),
                    ..Default::default()
                },
                &context,
            )?;
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    c.redirect_to_with(
        &c.url_for("/users/me/profile"),
        Redirect {
            notice: Some("Confirmed with Google. Continue with what you were doing.".into()),
            ..Default::default()
        },
    )
}
