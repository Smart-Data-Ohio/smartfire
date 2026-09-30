//! `app/controllers/two_factor`: domain writes are transactional; page rendering stays separate.
use crate::app::AppCtx;
use crate::concerns::{self, Before, current_session, require_current_user};
use crate::controllers::presenters::page::framed_page;
use campfire_db::models::audit_log::{Actor, AuditLog, Context, NewAuditLog, Target};
use campfire_db::{Session, TwoFactorBackupCode, TwoFactorCredential, TwoFactorSetupSecret};
use campfire_kit::{Ctx, Error, RateLimit, Result, StatusCode, format, halt};
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
    enum Enrollment {
        Enabled,
        Wrong,
        Confirmed {
            codes: Vec<String>,
            signed_out: usize,
            session: Box<Session>,
        },
    }
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            let mut credential = match TwoFactorCredential::for_user(tx.conn(), user.id)? {
                Some(credential) if credential.enabled() => return Ok(Enrollment::Enabled),
                Some(credential) => credential,
                None => TwoFactorCredential::create(
                    tx,
                    &ArEncryption::new(&secrets),
                    user.id,
                    &totp::generate_secret(),
                )?,
            };
            let Some(setup) = TwoFactorSetupSecret::valid_for(tx.conn(), session_id, tx.now())?
            else {
                return Ok(Enrollment::Wrong);
            };
            if !credential.confirm_with_setup_secret(
                tx,
                &ArEncryption::new(&secrets),
                &setup,
                &code,
            )? {
                return Ok(Enrollment::Wrong);
            }
            let codes = TwoFactorBackupCode::regenerate_set(tx, credential.id)?;
            let mut session = Session::find(tx.conn(), session_id)?;
            session.mark_two_factor_verified(tx)?;
            let others = Session::for_user(tx.conn(), user.id)?
                .into_iter()
                .filter(|s| s.id != session_id)
                .collect::<Vec<_>>();
            for other in &others {
                other.destroy(tx)?;
            }
            user.reset_remote_connections(tx);
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "two_factor.enable".into(),
                    target: Some(Target::from(&user)),
                    changes: (!others.is_empty())
                        .then(|| json!({"signed_out_other_devices":others.len()})),
                    ..Default::default()
                },
                &audit,
            )?;
            Ok(Enrollment::Confirmed {
                codes,
                signed_out: others.len(),
                session: Box::new(session),
            })
        })
        .await
        .map_err(Error::internal)?;
    match outcome {
        Enrollment::Enabled => profile(c),
        Enrollment::Wrong => {
            c.flash().now(
                "alert",
                "That code didn't work. Check your authenticator app and try again.",
            );
            render_setup(c, StatusCode::UNPROCESSABLE_ENTITY).await
        }
        Enrollment::Confirmed {
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
            let enc = ArEncryption::new(&secrets);
            let setup = if let Some(mut setup) =
                TwoFactorSetupSecret::valid_for(tx.conn(), session_id, tx.now())?
            {
                setup.extend_expiry(tx)?;
                setup
            } else {
                TwoFactorSetupSecret::issue_for(tx, &enc, session_id)?
            };
            setup.secret(&enc)
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
