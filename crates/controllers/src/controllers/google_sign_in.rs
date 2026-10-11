//! `sessions/google` and Google sign-in link controllers.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    integrations::google::sign_in::{self, Error as GoogleError},
};
use campfire_db::{
    models::{
        audit_log::{AuditLog, NewAuditLog, Target},
        google_identity,
    },
};
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode, halt};
use serde_json::{Map, Value, json};
use campfire_people::controllers::auth::{self, ResponseMode};

fn configured(c: &mut Ctx) -> Result<()> {
    if c.app().google.sign_in().config.configured() {
        Ok(())
    } else {
        halt(c.head(StatusCode::NOT_FOUND))
    }
}
async fn workspace(c: &mut Ctx) -> Result<()> {
    let ready = c
        .app()
        .db
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts) AND EXISTS(SELECT 1 FROM users)",
                [],
                |r| r.get::<_, bool>(0),
            )?)
        })
        .await
        .map_err(Error::internal)?;
    if !ready {
        return halt(c.redirect_to(&c.url_for("/first_run"))?);
    }
    Ok(())
}
fn redirect(c: &mut Ctx, path: &str, message: &str, notice: bool) -> Result {
    c.redirect_to_with(
        &c.url_for(path),
        Redirect {
            alert: (!notice).then(|| message.into()),
            notice: notice.then(|| message.into()),
            ..Default::default()
        },
    )
}
fn scalar(c: &Ctx, key: &str) -> String {
    c.params
        .get(key)
        .map(|p| campfire_richtext::ruby::json_value_to_s(&p.to_json()))
        .unwrap_or_default()
}
fn canceled(c: &Ctx) -> bool {
    c.params.get("error").is_some_and(|p| p.is_present())
        || c.params.get("code").is_none_or(|p| !p.is_present())
}
pub async fn create(c: &mut Ctx) -> Result {
    create_response(c, ResponseMode::Html).await
}

pub async fn create_json(c: &mut Ctx) -> Result {
    let result = create_response(c, ResponseMode::Json).await;
    auth::complete(c, result)
}

async fn create_response(c: &mut Ctx, mode: ResponseMode) -> Result {
    concerns::before_actions(c, Before::default().require_unauthenticated_access()).await?;
    configured(c)?;
    workspace(c).await?;
    let response = c.app().google.clone().start(c, "sign_in", None)?;
    if mode == ResponseMode::Html {
        return Ok(response);
    }
    // State, nonce, PKCE and the allowed provider host come from the retained start operation.
    let location = response
        .get_header("location")
        .expect("Google authorization redirect")
        .to_owned();
    auth::json(
        c,
        StatusCode::OK,
        &campfire_api_types::AuthResponse::Navigate { location },
    )
}
pub async fn callback(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    concerns::restore_authentication(c).await?;
    configured(c)?;
    workspace(c).await?;
    let flow = c
        .session()
        .remove(sign_in::FLOW_SESSION_KEY)
        .unwrap_or(Value::Null);
    super::sudos::restore_google_response_mode(c, &flow);
    let valid = sign_in::valid_flow(&flow, &scalar(c, "state"), &c.app().secrets, c.now());
    let purpose = flow.get("purpose").and_then(Value::as_str).unwrap_or("");
    if valid && matches!(purpose, "link" | "reauth" | "sudo") {
        return step_up(c, &flow, purpose).await;
    }
    if purpose == "sudo" && let Some(path) = super::sudos::google_return_path(c) {
        return redirect(c, path, "Confirmation expired. Try again.", false);
    }
    if concerns::signed_in(c) {
        return c.redirect_to(&c.url_for("/"));
    }
    if !valid {
        return redirect(
            c,
            "/session/new",
            "Google sign-in expired. Try again or sign in with email and password.",
            false,
        );
    }
    if c.params.get("error").is_some_and(|p| p.is_present()) {
        return redirect(
            c,
            "/session/new",
            "Google sign-in was cancelled. Try again or sign in with email and password.",
            false,
        );
    }
    if canceled(c) {
        return redirect(
            c,
            "/session/new",
            "Google sign-in failed. Try again or sign in with email and password.",
            false,
        );
    }
    match verified(c, &flow, purpose).await {
        Ok(claims) => complete_sign_in(c, claims).await,
        Err(GoogleError::Rejected(reason)) => rejected(c, reason).await,
        Err(GoogleError::Unavailable) => redirect(
            c,
            "/session/new",
            "Google sign-in is unavailable right now. Try again or sign in with email and password.",
            false,
        ),
    }
}
async fn verified(
    c: &Ctx,
    flow: &Value,
    purpose: &str,
) -> std::result::Result<Map<String, Value>, GoogleError> {
    let service = c.app().google.sign_in();
    let token = service
        .exchange_code(
            &scalar(c, "code"),
            &c.url_for("/session/google/callback"),
            flow["verifier"].as_str().unwrap(),
        )
        .await?;
    service
        .verify(
            &token,
            flow["nonce"].as_str().unwrap(),
            purpose,
            c.now().as_second(),
        )
        .await
}
async fn rejected(c: &mut Ctx, reason: &str) -> Result {
    tracing::warn!("Google sign-in rejected: {reason}");
    super::sessions::record_sign_in_failure(c, "google", String::new()).await?;
    let message=match reason {
        "wrong_domain"=>format!("Google sign-in is only available for {}. Other email addresses can sign in with email and password.",domain_list(c)),
        "ambiguous"|"subject_mismatch"=>"Google sign-in could not pick your account. Contact your administrator or sign in with email and password.".into(),
        "admin_link_required"=>"An administrator must link this account to Google before you can sign in with Google. Contact your administrator or sign in with email and password.".into(),
        "deactivated"|"banned"=>"This account is no longer active. Contact your administrator or sign in with email and password.".into(),
        _=>"Google sign-in failed. Try again or sign in with email and password.".into()
    };
    redirect(c, "/session/new", &message, false)
}
pub(super) fn domain_list(c: &Ctx) -> String {
    let domains = c
        .app()
        .google
        .sign_in()
        .config
        .domains
        .iter()
        .map(|d| format!("@{d}"))
        .collect::<Vec<_>>();
    match domains.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [one, two] => format!("{one} and {two}"),
        _ => format!(
            "{}, and {}",
            domains[..domains.len() - 1].join(", "),
            domains.last().unwrap()
        ),
    }
}
async fn complete_sign_in(c: &mut Ctx, claims: Map<String, Value>) -> Result {
    let device_id = concerns::ensure_device_cookie(c)?;
    let remember = c.cookies.signed("two_factor_remember");
    let agent = c.request.user_agent().map(str::to_string);
    let ip = c.request.remote_ip()?.to_string();
    let context = super::two_factor::audit_context(c)?;
    let notify = c.app().mail.config.security_configured();
    let result = c
        .app()
        .db
        .write(move |tx| {
            crate::authentication::begin_google_session(
                tx,
                &claims,
                campfire_db::NewSession {
                    user_agent: agent.as_deref(),
                    ip_address: Some(&ip),
                    device_id: Some(&device_id),
                    two_factor_verified: false,
                },
                remember.as_deref(),
                notify,
                &context,
            )
        })
        .await;
    let (user, session) = match result {
        Ok(result) => result,
        Err(campfire_db::Error::GoogleSignInRejected(reason)) => return rejected(c, reason).await,
        Err(error) => return Err(Error::internal(error)),
    };
    // Rails resolves the identity before consuming the password fallback's destination.
    let host = c.request.host();
    let stored = c.session().remove(concerns::session_keys::RETURN_TO_KEY);
    let return_url = sign_in::safe_return_path(stored.as_ref().and_then(Value::as_str), &host)
        .unwrap_or_else(|| c.url_for("/"));
    if let Some(session) = session {
        concerns::session_keys::clear_confirmations(c.session());
        concerns::authenticated_as(c, session, Some(user), true).await?;
        c.form_authenticity_token();
        let return_url = concerns::post_authentication_destination(c, return_url).await?;
        c.redirect_to(&return_url)
    } else {
        let now = c.now();
        c.session()
            .insert(concerns::session_keys::RETURN_TO_KEY, return_url);
        concerns::session_keys::stash_two_factor_pending(c.session(), user.id, "google", now);
        c.redirect_to(&c.url_for("/two_factor_challenge"))
    }
}
async fn step_up_profile(c: &Ctx, purpose: &str) -> campfire_kit::Result<&'static str> {
    if purpose == "sudo" { return Ok("/users/me/profile"); }
    if let Some(user) = concerns::current_user(c)
        && concerns::next_ui(c, user).await?
    {
        return Ok(if purpose == "reauth" { "/app/settings/security" } else { "/app/settings/integrations" });
    }
    Ok("/users/me/profile")
}
async fn step_up(c: &mut Ctx, flow: &Value, purpose: &str) -> Result {
    let user = concerns::current_user(c).cloned();
    if user
        .as_ref()
        .is_none_or(|u| Some(u.id) != flow["user_id"].as_i64())
    {
        let path = if user.is_some() {
            if purpose == "sudo" { super::sudos::google_return_path(c).unwrap_or("/users/me/profile") } else { step_up_profile(c, purpose).await? }
        } else {
            "/session/new"
        };
        let message = match purpose {
            "link" => "Google linking expired. Try again.",
            "sudo" => "Confirmation expired. Try again.",
            _ => "Google confirmation expired. Try again.",
        };
        return redirect(c, path, message, false);
    }
    let user = user.unwrap();
    let path = if purpose == "sudo" {
        super::sudos::google_return_path(c).unwrap_or("/sudo/new")
    } else {
        step_up_profile(c, purpose).await?
    };
    if canceled(c) {
        return redirect(
            c,
            path,
            if purpose == "link" {
                "Google linking was cancelled."
            } else {
                "Google confirmation was cancelled."
            },
            false,
        );
    }
    let claims = match verified(c, flow, purpose).await {
        Ok(claims) => claims,
        Err(GoogleError::Unavailable) => {
            return redirect(
                c,
                path,
                "Google is unavailable right now. Try again.",
                false,
            );
        }
        Err(GoogleError::Rejected(reason)) => {
            match purpose {
                "link" => tracing::warn!("Google link rejected: {reason}"),
                "sudo" => tracing::warn!("Google sudo confirmation rejected: {reason}"),
                _ => tracing::warn!("Google re-auth rejected: {reason}"),
            }
            return redirect(
                c,
                path,
                &if purpose == "link" {
                    link_alert(c, reason)
                } else {
                    "Google confirmation failed. Try again.".into()
                },
                false,
            );
        }
    };
    let subject =
        campfire_richtext::ruby::json_value_to_s(claims.get("sub").unwrap_or(&Value::Null));
    let auth_time = claims.get("auth_time").and_then(Value::as_f64);
    match purpose {
        "reauth" => {
            super::two_factor::finish_google_reauthentication(c, user.id, &subject, auth_time).await
        }
        "sudo" => super::sudos::finish_google(c, user.id, &subject, auth_time).await,
        _ => {
            let email = campfire_richtext::ruby::json_value_to_s(
                claims.get("email").unwrap_or(&Value::Null),
            );
            let context = super::two_factor::audit_context(c)?;
            let result = c
                .app()
                .db
                .write(move |tx| {
                    google_identity::GoogleIdentity::link_to_user(tx, &claims, user.id)?;
                    AuditLog::record(
                        tx,
                        NewAuditLog {
                            action: "google.sign_in.link".into(),
                            target: Some(Target::from(&user)),
                            changes: Some(json!({"email":claims.get("email")})),
                            ..Default::default()
                        },
                        &context,
                    )?;
                    Ok(())
                })
                .await;
            match result {
                Ok(()) => redirect(c, path, &format!("Google sign-in linked to {email}."), true),
                Err(campfire_db::Error::GoogleSignInRejected(reason)) => {
                    tracing::warn!("Google link rejected: {reason}");
                    redirect(c, path, &link_alert(c, reason), false)
                }
                Err(error) => Err(Error::internal(error)),
            }
        }
    }
}
fn link_alert(c: &Ctx, reason: &str) -> String {
    match reason { "wrong_domain"=>format!("Only {} Google accounts can be linked.",domain_list(c)),"subject_taken"=>"That Google account already signs in as another member.".into(),"already_linked"=>"Your account is already linked to a Google account. Ask an administrator to unlink it first.".into(),_=>"Google linking failed. Try again.".into() }
}
pub async fn link(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    configured(c)?;
    let id = concerns::require_current_user(c)?.id;
    let linked = c
        .app()
        .db
        .read(move |conn| google_identity::GoogleIdentity::for_user(conn, id))
        .await
        .map_err(Error::internal)?
        .is_some();
    if linked {
        let path = step_up_profile(c, "link").await?;
        redirect(
            c,
            path,
            "Google sign-in is already linked.",
            true,
        )
    } else {
        c.app().google.clone().start(c, "link", Some(id))
    }
}
pub async fn admin_allow(c: &mut Ctx) -> Result {
    admin(c, true).await
}
pub async fn admin_unlink(c: &mut Ctx) -> Result {
    admin(c, false).await
}
async fn admin(c: &mut Ctx, allow: bool) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let id = c
        .param_str("user_id")
        .and_then(concerns::cast_integer)
        .ok_or(Error::Status(StatusCode::NOT_FOUND))?;
    let context = super::two_factor::audit_context(c)?;
    let user=c.app().db.write(move |tx| {
        google_identity::GoogleIdentity::admin_set_link(tx,id,allow,&context)
    }).await.map_err(|e|match e {campfire_db::Error::RecordNotFound(_)=>Error::Status(StatusCode::NOT_FOUND),_=>Error::internal(e)})?;
    let message = if allow {
        format!(
            "{} can now link Google sign-in for {}.",
            user.name,
            user.email_address.unwrap_or_default()
        )
    } else {
        format!("Google sign-in unlinked from {}.", user.name)
    };
    redirect(c, "/account/edit", &message, true)
}
