//! The link is sufficient to attach its address to its account, including from another device.
use campfire_api_types::EmailAddressConfirmation;
use campfire_db::user::email_addresses::{Confirmation, EmailAddress};
use campfire_db::{Connection, Timestamp, User};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use rails_compat::email_address_confirmation::{self, Claim};

use super::auth::{self, ResponseMode};
use crate::app::AppCtx;
use crate::authentication;
use crate::concerns::{self, Before};

const ACCOUNT_MISMATCH: &str =
    "You're signed in to a different account. Sign out, then open this link again.";

enum Applied {
    Invalid,
    AccountMismatch,
    Taken,
    Linked { viewer_is_owner: bool },
}

pub async fn show_json(c: &mut Ctx) -> Result {
    let result = show(c).await;
    auth::complete(c, result)
}

pub async fn update_json(c: &mut Ctx) -> Result {
    let result = update(c).await;
    auth::complete(c, result)
}

async fn before_actions(c: &mut Ctx, endpoint: &'static str) -> Result<()> {
    c.set_header("referrer-policy", "no-referrer");
    auth::before_actions(
        c,
        endpoint,
        Before::default().allow_unauthenticated_access(),
        ResponseMode::Json,
    )
    .await?;
    concerns::restore_authentication(c).await?;
    Ok(())
}

async fn show(c: &mut Ctx) -> Result {
    before_actions(c, "email_address_confirmations#show").await?;
    let secrets = c.app().secrets.clone();
    let Some(claim) = email_address_confirmation::verify(
        &secrets,
        c.param_str("token").unwrap_or_default(),
        c.now(),
    ) else {
        return invalid_link(c);
    };
    let Some((user, claim)) = c
        .app()
        .db
        .read(move |conn| Ok(confirmable_user(conn, &secrets, &claim)?.map(|user| (user, claim))))
        .await
        .map_err(Error::internal)?
    else {
        return invalid_link(c);
    };
    let viewer = concerns::current_user(c).map(|viewer| viewer.id);
    let confirmation = EmailAddressConfirmation {
        email_address: claim.email_address,
        account_name: user.display_name().to_owned(),
        requires_sign_in: viewer != Some(claim.user_id),
        account_mismatch: viewer.is_some_and(|id| id != claim.user_id),
    };
    auth::json(c, StatusCode::OK, &confirmation)
}

async fn update(c: &mut Ctx) -> Result {
    before_actions(c, "email_address_confirmations#update").await?;
    let token = c.param_str("token").unwrap_or_default().to_owned();
    let secrets = c.app().secrets.clone();
    let context = super::two_factor::audit_context(c)?;
    let viewer = concerns::current_user(c).map(|viewer| viewer.id);
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            let Some(claim) = email_address_confirmation::verify(&secrets, &token, tx.now().jiff())
            else {
                return Ok(Applied::Invalid);
            };
            let Some(user) = confirmable_user(tx.conn(), &secrets, &claim)? else {
                return Ok(Applied::Invalid);
            };
            if viewer.is_some_and(|id| id != claim.user_id) {
                return Ok(Applied::AccountMismatch);
            }
            let result = EmailAddress::confirm(tx, &user, &claim.email_address)?;
            if result == Confirmation::Added {
                authentication::audit_email_address(
                    tx,
                    &user,
                    "user.email_address.add",
                    &claim.email_address,
                    &context,
                )?;
            }
            Ok(match result {
                Confirmation::Taken => Applied::Taken,
                Confirmation::Added | Confirmation::AlreadyLinked => Applied::Linked {
                    viewer_is_owner: viewer == Some(claim.user_id),
                },
            })
        })
        .await
        .map_err(Error::internal)?;
    match outcome {
        Applied::Invalid => invalid_link(c),
        Applied::AccountMismatch => {
            auth::field_error(c, StatusCode::BAD_REQUEST, "base", ACCOUNT_MISMATCH)
        }
        Applied::Taken => auth::field_error(
            c,
            StatusCode::UNPROCESSABLE_ENTITY,
            "emailAddress",
            "This email address is already linked to another account.",
        ),
        Applied::Linked { viewer_is_owner } => {
            c.flash()
                .set_notice("Your email address is linked. You can use it to sign in.");
            let path = if viewer_is_owner {
                "/app/settings"
            } else {
                "/app/session/new"
            };
            ResponseMode::Json.navigate(c, &c.url_for(path))
        }
    }
}

fn confirmable_user(
    conn: &Connection,
    secrets: &rails_compat::Secrets,
    claim: &Claim,
) -> campfire_db::Result<Option<User>> {
    let Some(user) =
        User::find_by_id(conn, claim.user_id)?.filter(|user| user.is_active() && !user.is_bot())
    else {
        return Ok(None);
    };
    if !claim.matches_password(secrets, user.password_digest.as_deref().unwrap_or_default())
        || EmailAddress::removed_since(
            conn,
            user.id,
            &claim.email_address,
            Timestamp::from_jiff(claim.issued_at),
        )?
    {
        return Ok(None);
    }
    Ok(Some(user))
}

fn invalid_link(c: &mut Ctx) -> Result {
    auth::field_error(
        c,
        StatusCode::BAD_REQUEST,
        "base",
        "This confirmation link is invalid or expired. Send a new link from your profile settings.",
    )
}
