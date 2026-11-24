//! Signed-in management of confirmed email addresses.
use campfire_api_types::{
    ApiError, EmailAddressEntry, EmailAddressList, EmailAddressRequest, EmailAddressRequestResult,
};
use campfire_app::app::AppCtx;
use campfire_db::user::email_addresses::EmailAddress;
use campfire_db::{Error as DbError, User};
use campfire_kit::{Ctx, Error, RateLimit, Result, StatusCode};
use campfire_runtime::authentication;
use jiff::SignedDuration;

use crate::admin::{self, body, require_sudo, viewer};
use crate::error::{fail, not_found, validation};

endpoint!(index => list);
endpoint!(create => request_confirmation);
endpoint!(destroy => remove);
endpoint!(primary => promote);

async fn list(c: &mut Ctx) -> Result {
    let user_id = viewer(c).await?.id;
    let can_add = c.app().mail.config.password_reset_available();
    let list = c
        .app()
        .db
        .read(move |conn| {
            let user = User::find_active(conn, user_id)?;
            let secondary = EmailAddress::for_user(conn, user_id)?
                .into_iter()
                .map(|entry| EmailAddressEntry {
                    id: entry.id,
                    email_address: entry.email_address,
                    created_at: entry.created_at.jiff().to_string(),
                })
                .collect();
            Ok(EmailAddressList {
                primary: user.email_address,
                secondary,
                can_add,
            })
        })
        .await
        .map_err(Error::internal)?;
    c.json(StatusCode::OK, &list)
}

async fn request_confirmation(c: &mut Ctx) -> Result {
    let user_id = viewer(c).await?.id;
    if !c.app().mail.config.password_reset_available() {
        return Err(fail(c, not_found()));
    }
    require_sudo(c).await?;
    let request: EmailAddressRequest = body(c).await?;
    let email_address = request.email_address.trim().to_owned();
    if email_address.len() > 254
        || !campfire_db::models::drive_recipients::eligible_email(&email_address)
        || campfire_mail::outbound::first_address(&email_address).is_err()
    {
        return Err(fail(c, validation("emailAddress", "isn't valid")));
    }
    let limit = RateLimit::new(
        "email_address_confirmation",
        5,
        SignedDuration::from_hours(1),
    );
    if c.rate_limited(&limit, Some(&user_id.to_string()))? {
        return Err(fail(
            c,
            ApiError::RateLimited {
                message: "Too many requests. Try again in an hour.".into(),
                retry_after: 3600,
            },
        ));
    }
    let destination = email_address.clone();
    c.app()
        .db
        .write(move |tx| {
            authentication::request_email_address_confirmation(tx, user_id, destination);
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    c.json(
        StatusCode::OK,
        &EmailAddressRequestResult::ConfirmationSent { email_address },
    )
}

async fn remove(c: &mut Ctx) -> Result {
    let user_id = viewer(c).await?.id;
    require_sudo(c).await?;
    let id = address_id(c)?;
    let context = admin::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let user = User::find_active(tx.conn(), user_id)?;
            let address = EmailAddress::remove(tx, user_id, id)?
                .ok_or(DbError::RecordNotFound("EmailAddress"))?;
            authentication::audit_email_address(
                tx,
                &user,
                "user.email_address.remove",
                &address.email_address,
                &context,
            )
        })
        .await
        .map_err(Error::internal)?;
    Ok(c.head(StatusCode::NO_CONTENT))
}

async fn promote(c: &mut Ctx) -> Result {
    let mut user = viewer(c).await?;
    require_sudo(c).await?;
    let id = address_id(c)?;
    let context = admin::audit_context(c)?;
    let session_id = campfire_runtime::concerns::sudo::session_id(c);
    let saved = c
        .app()
        .db
        .write(move |tx| {
            authentication::promote_email_address(tx, &mut user, id, session_id, &context)
        })
        .await
        .map_err(Error::internal)?;
    if !saved {
        return Ok(c.head(StatusCode::UNAUTHORIZED));
    }
    Ok(c.head(StatusCode::NO_CONTENT))
}

fn address_id(c: &mut Ctx) -> Result<i64> {
    c.param_str("id")
        .and_then(|id| id.parse().ok())
        .ok_or_else(|| fail(c, not_found()))
}
