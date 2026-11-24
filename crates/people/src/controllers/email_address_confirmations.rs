//! The link is sufficient to attach its address to its account, including from another device.
use campfire_api_types::EmailAddressConfirmation;
use campfire_db::User;
use campfire_db::user::email_addresses::{Confirmation, EmailAddress};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use rails_compat::email_address_confirmation::{self, Claim};

use super::auth::{self, ResponseMode};
use crate::app::AppCtx;
use crate::authentication;
use crate::concerns::{self, Before};

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
    let Some(Claim {
        user_id,
        email_address,
    }) = email_address_confirmation::verify(
        &c.app().secrets,
        c.param_str("token").unwrap_or_default(),
        c.now(),
    )
    else {
        return invalid_link(c);
    };
    let active = c
        .app()
        .db
        .read(move |conn| {
            Ok(User::find_by_id(conn, user_id)?
                .is_some_and(|user| user.is_active() && !user.is_bot()))
        })
        .await
        .map_err(Error::internal)?;
    if !active {
        return invalid_link(c);
    }
    let confirmation = EmailAddressConfirmation {
        email_address,
        requires_sign_in: concerns::current_user(c).is_none(),
    };
    auth::json(c, StatusCode::OK, &confirmation)
}

async fn update(c: &mut Ctx) -> Result {
    before_actions(c, "email_address_confirmations#update").await?;
    let token = c.param_str("token").unwrap_or_default().to_owned();
    let secrets = c.app().secrets.clone();
    let context = super::two_factor::audit_context(c)?;
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            let Some(claim) = email_address_confirmation::verify(&secrets, &token, tx.now().jiff())
            else {
                return Ok(None);
            };
            let Some(user) = User::find_by_id(tx.conn(), claim.user_id)?
                .filter(|user| user.is_active() && !user.is_bot())
            else {
                return Ok(None);
            };
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
            Ok(Some(result))
        })
        .await
        .map_err(Error::internal)?;
    match outcome {
        None => invalid_link(c),
        Some(Confirmation::Taken) => auth::field_error(
            c,
            StatusCode::UNPROCESSABLE_ENTITY,
            "emailAddress",
            "This email address is already linked to another account.",
        ),
        Some(Confirmation::Added | Confirmation::AlreadyLinked) => {
            c.flash()
                .set_notice("Your email address is linked. You can use it to sign in.");
            let path = if concerns::current_user(c).is_some() {
                "/app/settings"
            } else {
                "/app/session/new"
            };
            ResponseMode::Json.navigate(c, &c.url_for(path))
        }
    }
}

fn invalid_link(c: &mut Ctx) -> Result {
    auth::field_error(
        c,
        StatusCode::BAD_REQUEST,
        "base",
        "This confirmation link is invalid or expired. Send a new link from your profile settings.",
    )
}
