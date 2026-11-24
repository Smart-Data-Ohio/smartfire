//! "Forgot password?", after Rails 8's `PasswordsController`: ask for a link by email address,
//! then choose a new password with it. The link is a signed token
//! (`rails_compat::password_reset`) that names the user and their current password and address,
//! so it works once and only while both stay the same. Setting the password signs the user out
//! everywhere and doesn't sign them in: they sign in again, second step and access gates included.
//!
//! Reset is available only when outbound mail can send a working link
//! (`Config::password_reset_available`); otherwise every endpoint answers 404.

use campfire_api_types::{PasswordResetRequest, PasswordResetState, PasswordResetSubmission};
use campfire_db::models::audit_log::Context;
use campfire_db::user::email_addresses::{EmailAddress, normalize, trim};
use campfire_db::{PasswordDigest, Tx, User};
use campfire_kit::{Ctx, Error, RateLimit, Result, StatusCode, halt};
use jiff::SignedDuration;
use rails_compat::password_reset::{self, Claim};
use sha2::{Digest, Sha256};

use super::auth::{self, ResponseMode};
use crate::app::AppCtx;
use crate::authentication::{self, can_reset_password};
use crate::concerns::{self, Before};

const INVALID_LINK: &str = "This reset link didn't work. It may have expired or already been used.";
const TOO_MANY: &str = "Too many attempts. Try again in a few minutes.";
const BLANK_PASSWORD: &str = "Enter a new password.";
const LONG_PASSWORD: &str = "That password is too long. Choose a shorter one.";
const RESET_NOTICE: &str = "Password updated. Sign in with your new password.";

/// bcrypt reads only the first 72 bytes of a password.
const MAX_PASSWORD_BYTES: usize = 72;
const MAX_EMAIL_BYTES: usize = 254;

/// Requests from one address.
pub fn request_rate_limit() -> RateLimit {
    RateLimit::new("password_resets", 5, SignedDuration::from_mins(15)).named("create")
}

/// Requests for one email address, from anywhere, so one inbox can't be flooded.
pub fn inbox_rate_limit() -> RateLimit {
    RateLimit::new("password_resets", 3, SignedDuration::from_mins(15)).named("inbox")
}

/// New passwords from one address.
pub fn update_rate_limit() -> RateLimit {
    RateLimit::new("password_resets", 10, SignedDuration::from_mins(15)).named("update")
}

/// `POST /api/v1/password_resets`: the same `requested` answer for every address, account or not;
/// the delivery job decides whether one is sent.
pub async fn create_json(c: &mut Ctx) -> Result {
    let result = create_response(c).await;
    auth::complete(c, result)
}

async fn create_response(c: &mut Ctx) -> Result {
    before_actions(c, "password_resets#create").await?;
    rate_limit(c, &request_rate_limit(), None)?;
    let body: PasswordResetRequest = auth::body(c).await?;
    let email_address = body.email_address;
    rate_limit(c, &inbox_rate_limit(), Some(&inbox_key(&email_address)))?;
    if trim(&email_address).len() <= MAX_EMAIL_BYTES {
        c.app()
            .db
            .write(move |tx| {
                authentication::request_password_reset(tx, email_address);
                Ok(())
            })
            .await
            .map_err(Error::internal)?;
    }
    auth::json(c, StatusCode::OK, &PasswordResetState::Requested)
}

/// `GET /api/v1/password_resets/:token`: whether the link still works, before a password is typed.
pub async fn show_json(c: &mut Ctx) -> Result {
    let result = show_response(c).await;
    auth::complete(c, result)
}

async fn show_response(c: &mut Ctx) -> Result {
    before_actions(c, "password_resets#edit").await?;
    match resettable(c).await? {
        Some(_) => auth::json(c, StatusCode::OK, &PasswordResetState::Ready),
        None => invalid_link(c),
    }
}

/// `PUT /api/v1/password_resets/:token`: the new password, then `navigate` to sign in.
pub async fn update_json(c: &mut Ctx) -> Result {
    let result = update_response(c).await;
    auth::complete(c, result)
}

async fn update_response(c: &mut Ctx) -> Result {
    before_actions(c, "password_resets#update").await?;
    rate_limit(c, &update_rate_limit(), None)?;
    let body: PasswordResetSubmission = auth::body(c).await?;
    let Some(claim) = resettable(c).await? else {
        return refuse_link(c).await;
    };
    if let Some(problem) = password_problem(&body.password) {
        return auth::field_error(c, StatusCode::UNPROCESSABLE_ENTITY, "password", problem);
    }
    let digest = concerns::password_digest(c, Some(body.password))
        .await?
        .expect("a password was given");
    let context = super::two_factor::audit_context(c)?;
    let secrets = c.app().secrets.clone();
    let reset = c
        .app()
        .db
        .write(move |tx| reset_with_claim(tx, &secrets, &claim, digest, &context))
        .await
        .map_err(Error::internal)?;
    if !reset {
        return refuse_link(c).await;
    }
    c.flash().set_notice(RESET_NOTICE);
    let sign_in = c.url_for("/app/session/new");
    ResponseMode::Json.navigate(c, &sign_in)
}

async fn before_actions(c: &mut Ctx, endpoint: &'static str) -> Result<()> {
    auth::before_actions(
        c,
        endpoint,
        Before::default().allow_unauthenticated_access(),
        ResponseMode::Json,
    )
    .await?;
    if !c.app().mail.config.password_reset_available() {
        return halt(c.head(StatusCode::NOT_FOUND));
    }
    Ok(())
}

fn rate_limit(c: &mut Ctx, limit: &RateLimit, by: Option<&str>) -> Result<()> {
    if c.rate_limited(limit, by)? {
        return halt(auth::field_error(
            c,
            StatusCode::TOO_MANY_REQUESTS,
            "base",
            TOO_MANY,
        )?);
    }
    Ok(())
}

/// One bucket per address however it's capitalised or spaced, without keeping the address.
fn inbox_key(email_address: &str) -> String {
    let normalized = normalize(email_address);
    hex::encode(Sha256::digest(normalized.as_bytes()))
}

/// The link's user, while the link verifies and still matches them.
async fn resettable(c: &Ctx) -> Result<Option<Claim>> {
    let token = c.param_str("token").unwrap_or_default();
    let secrets = c.app().secrets.clone();
    let Some(claim) = password_reset::verify(&secrets, token, c.now()) else {
        return Ok(None);
    };
    c.app()
        .db
        .read(move |conn| Ok(resettable_user(conn, &secrets, &claim)?.map(|_| claim)))
        .await
        .map_err(Error::internal)
}

/// Sets the new password if the link still matches its user and is unexpired, checked again on the
/// writer: of two uses of one link racing each other, the second finds the first's digest and
/// changes nothing, and a link that expired while the password was hashed changes nothing either.
pub fn reset_with_claim(
    tx: &mut Tx<'_>,
    secrets: &rails_compat::Secrets,
    claim: &Claim,
    digest: PasswordDigest,
    context: &Context,
) -> campfire_db::Result<bool> {
    if !claim.unexpired(secrets, tx.now().jiff()) {
        return Ok(false);
    }
    let current = resettable_user(tx.conn(), secrets, claim)?;
    let Some(mut user) = current else {
        return Ok(false);
    };
    authentication::reset_password(tx, &mut user, digest, context)?;
    Ok(true)
}

fn resettable_user(
    conn: &campfire_db::Connection,
    secrets: &rails_compat::Secrets,
    claim: &Claim,
) -> campfire_db::Result<Option<User>> {
    let Some(user) = User::find_by_id(conn, claim.user_id)?.filter(can_reset_password) else {
        return Ok(None);
    };
    let digest = user.password_digest.as_deref().unwrap_or_default();
    let matches = if let Some(id) = claim.secondary_id {
        EmailAddress::find_owned(conn, user.id, id)?.is_some_and(|address| {
            claim.matches_secondary(secrets, digest, id, &address.email_address)
        })
    } else {
        user.email_address
            .as_deref()
            .filter(|address| !address.is_empty())
            .is_some_and(|address| claim.matches(secrets, digest, address))
    };
    Ok(matches.then_some(user))
}

fn password_problem(password: &str) -> Option<&'static str> {
    if campfire_richtext::ruby::is_blank(password) {
        Some(BLANK_PASSWORD)
    } else if password.len() > MAX_PASSWORD_BYTES {
        Some(LONG_PASSWORD)
    } else {
        None
    }
}

/// A `token` error, so the page can tell a dead link (ask for another) from a refusal it can
/// retry, such as a rate limit's `base` error.
fn invalid_link(c: &mut Ctx) -> Result {
    auth::field_error(c, StatusCode::BAD_REQUEST, "token", INVALID_LINK)
}

/// A used, expired or forged link, recorded as a failed sign-in as a bad transfer link is.
async fn refuse_link(c: &mut Ctx) -> Result {
    super::sessions::record_sign_in_failure(c, "password_reset", String::new()).await?;
    invalid_link(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwords_must_be_present_and_fit_bcrypt() {
        assert_eq!(password_problem(""), Some(BLANK_PASSWORD));
        assert_eq!(password_problem("   "), Some(BLANK_PASSWORD));
        assert_eq!(password_problem(&"a".repeat(72)), None);
        assert_eq!(password_problem(&"a".repeat(73)), Some(LONG_PASSWORD));
        // 24 three-byte characters fill bcrypt's 72 bytes; one more doesn't fit.
        assert_eq!(password_problem(&"€".repeat(24)), None);
        assert_eq!(password_problem(&"€".repeat(25)), Some(LONG_PASSWORD));
    }

    #[test]
    fn inbox_buckets_ignore_case_and_surrounding_space() {
        assert_eq!(
            inbox_key("\u{a0}Ada@Example.com\t"),
            inbox_key("ada@example.com")
        );
        assert_ne!(inbox_key("ada@example.com"), inbox_key("bob@example.com"));
        assert!(!inbox_key("ada@example.com").contains("ada"));
    }
}
