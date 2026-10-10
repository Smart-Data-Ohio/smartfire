//! `SessionsController` (reference/app/controllers/sessions_controller.rb): sign in and out.

pub mod transfers;

use campfire_db::PushSubscription;
use campfire_kit::{Ctx, Error, RateLimit, Result, StatusCode, format, halt};
use campfire_retained::sessions;
use jiff::SignedDuration;

use super::auth::{self, ResponseMode};
use super::presenters;
use crate::app::AppCtx;
use crate::concerns::{self, Before, current_user};
use crate::controllers::presenters::page::retained_page;

/// `rate_limit to: 10, within: 3.minutes, only: :create`
const RATE_LIMIT_TO: u64 = 10;
const RATE_LIMIT_WITHIN: SignedDuration = SignedDuration::from_mins(3);

const REJECTION: &str = "Too many requests or unauthorized.";

/// `allow_unauthenticated_access only: %i[ new create ]`, `before_action :ensure_user_exists, only: :new`
pub async fn new(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    ensure_user_exists(c).await?;
    // Our Rails app: background polls redirected to sign in keep their JSON Accept header, and
    // get a 401 (`format.json { head :unauthorized }`) rather than UnknownFormat's 406.
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::JSON {
        return Ok(c.head(StatusCode::UNAUTHORIZED));
    }
    render_new(c, StatusCode::OK).await
}

pub async fn create(c: &mut Ctx) -> Result {
    create_response(c, ResponseMode::Html).await
}

pub async fn create_json(c: &mut Ctx) -> Result {
    let result = create_response(c, ResponseMode::Json).await;
    auth::complete(c, result)
}

async fn create_response(c: &mut Ctx, mode: ResponseMode) -> Result {
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    if mode == ResponseMode::Json {
        let body: campfire_api_types::PasswordSignIn = match auth::body(c).await {
            Ok(body) => body,
            Err(error) => {
                rate_limit(c, mode).await?;
                return Err(error);
            }
        };
        c.params.insert(
            "email_address",
            campfire_kit::Param::Str(body.email_address),
        );
        c.params
            .insert("password", campfire_kit::Param::Str(body.password));
    }
    rate_limit(c, mode).await?;

    let email_address = c.param_str("email_address").map(str::to_string);
    let password = c.param_str("password").map(str::to_string);
    let user = match (email_address, password) {
        (Some(email_address), Some(password)) => {
            concerns::authenticate_by(c, email_address, password).await?
        }
        _ => None,
    };

    match user {
        Some(user) => super::two_factor::begin_session_response(c, user, "password", mode).await,
        None => render_rejection(c, StatusCode::UNAUTHORIZED, mode).await,
    }
}

pub async fn destroy(c: &mut Ctx) -> Result {
    destroy_response(c, ResponseMode::Html).await
}

pub async fn destroy_json(c: &mut Ctx) -> Result {
    let result = destroy_response(c, ResponseMode::Json).await;
    auth::complete(c, result)
}

async fn destroy_response(c: &mut Ctx, mode: ResponseMode) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    if mode == ResponseMode::Json {
        let body: campfire_api_types::SignOut = auth::body(c).await?;
        c.params.remove("push_subscription_endpoint");
        if let Some(endpoint) = body.push_subscription_endpoint {
            c.params.insert(
                "push_subscription_endpoint",
                campfire_kit::Param::Str(endpoint),
            );
        }
    }
    remove_push_subscription(c).await?;
    concerns::terminate_current_session(c).await?;
    let root = c.url_for(&campfire_routes::root());
    mode.navigate(c, &root)
}

/// `redirect_to first_run_url if User.none?`
async fn ensure_user_exists(c: &mut Ctx) -> Result<()> {
    let none = c
        .app()
        .db
        .read(presenters::accounts::no_users)
        .await
        .map_err(Error::internal)?;
    if none {
        let first_run = c.url_for(&campfire_routes::first_run());
        return halt(c.redirect_to(&first_run)?);
    }
    Ok(())
}

/// `flash.now[:alert] = "Too many requests or unauthorized."; render :new, status:`
async fn render_rejection(c: &mut Ctx, status: StatusCode, mode: ResponseMode) -> Result {
    record_sign_in_failure(
        c,
        "password",
        c.params
            .get("email_address")
            .map(|p| {
                p.to_s()
                    .unwrap_or_else(|| campfire_richtext::ruby::json_value_inspect(&p.to_json()))
            })
            .unwrap_or_default(),
    )
    .await?;
    c.flash().now("alert", REJECTION);
    if mode == ResponseMode::Json {
        return auth::field_error(c, status, "base", REJECTION);
    }
    render_new(c, status).await
}

pub async fn record_sign_in_failure(c: &Ctx, method: &'static str, email: String) -> Result<()> {
    let context = super::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            campfire_db::models::audit_log::AuditLog::record_sign_in_failure(
                tx, &email, method, &context,
            )?;
            Ok(())
        })
        .await
        .map_err(Error::internal)
}

async fn render_new(c: &mut Ctx, status: StatusCode) -> Result {
    c.respond_to(&[&format::HTML])?;
    let email_address = c.param_str("email_address").map(str::to_string);
    let help_contact = c
        .app()
        .db
        .read(presenters::accounts::help_contact)
        .await
        .map_err(Error::internal)?;
    // WS14g supplies the configured provider and /session/google start/callback handlers.
    // This read-only display input never authenticates or completes a Google flow.
    let provider = c.app().google.sign_in();
    let google_sign_in_domains = if provider.config.configured() {
        provider.config.domains.clone()
    } else {
        Vec::new()
    };
    retained_page!(c, status, |ctx| sessions::New {
        ctx,
        email_address: email_address.clone(),
        help_contact: help_contact.clone(),
        google_sign_in_domains: google_sign_in_domains.clone(),
    })
    .await
}

/// `Push::Subscription.destroy_by(endpoint: params[:push_subscription_endpoint], user_id: Current.user.id)`
pub(crate) async fn remove_push_subscription(c: &mut Ctx) -> Result<()> {
    let Some(endpoint) = c
        .param_str("push_subscription_endpoint")
        .map(str::to_string)
    else {
        return Ok(());
    };
    let Some(user_id) = current_user(c).map(|user| user.id) else {
        return Ok(());
    };
    c.app()
        .db
        .write(move |tx| PushSubscription::destroy_by_endpoint(tx, user_id, &endpoint))
        .await
        .map_err(Error::internal)
}

// --- Rate limiting ---------------------------------------------------------------------------------

/// `rate_limit to: 10, within: 3.minutes, only: :create, with: -> { render_rejection :too_many_requests }`
pub fn rate_limit_rule() -> RateLimit {
    RateLimit::new("sessions", RATE_LIMIT_TO, RATE_LIMIT_WITHIN)
}

async fn rate_limit(c: &mut Ctx, mode: ResponseMode) -> Result<()> {
    if c.rate_limited(&rate_limit_rule(), None)? {
        return halt(render_rejection(c, StatusCode::TOO_MANY_REQUESTS, mode).await?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_like_the_reference() {
        let vectors: serde_json::Value =
            serde_json::from_str(include_str!("../../../../vectors/kit_security.json")).unwrap();
        assert_eq!(
            rate_limit_rule().cache_key("10.3.0.1"),
            vectors["rate_limit"]["cache_key"].as_str().unwrap()
        );
    }
}
