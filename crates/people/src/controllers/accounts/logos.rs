//! `Accounts::LogosController` (reference/app/controllers/accounts/logos_controller.rb): the
//! account logo (or the stock app icon) as a PNG, public so it can be the PWA icon.

use campfire_db::Account;
use campfire_kit::{Ctx, Error, ExpiresIn, Freshness, Result, SendOptions};
use campfire_storage::Variation;

use crate::app::AppCtx;
use crate::concerns::{self, Before};
use crate::controllers::presenters::attachments::{self, Record};
use crate::controllers::presenters::cache_key_with_version;
use crate::controllers::users::avatars::asset_file;

/// `expires_in 5.minutes, public: true, stale_while_revalidate: 1.week`
const MAX_AGE: u64 = 5 * 60;
const STALE_WHILE_REVALIDATE: u64 = 7 * 24 * 60 * 60;

/// `allow_unauthenticated_access only: :show`
pub async fn show(c: &mut Ctx) -> Result {
    c.use_live_response(); // `include ActiveStorage::Streaming`
    concerns::before_actions(c, Before::default().allow_unauthenticated_access().skip_allow_browser()).await?;
    if c.param_str("animated") == Some("1")
        || c.param_str("still") == Some("1")
        || c.param_str("blob").is_some()
    {
        return super::banners::show_image(c, "logo").await;
    }
    let (variation, small) = match c.param_str("size") {
        Some("small") => (Variation::resize_to_limit(192, 192, Some("png")), true),
        Some("touch") => (Variation::resize_and_pad(TOUCH_ICON_SIZE, TOUCH_ICON_SIZE), true),
        _ => (Variation::resize_to_limit(512, 512, Some("png")), false),
    };
    send_logo(c, variation, |c| stock_icon(c, small)).await
}

/// The apple-touch-icon's side: iOS shows a square tile and fills a transparent one with black.
const TOUCH_ICON_SIZE: i64 = 180;

/// The attached logo as the PNG `variation` makes, with `show`'s caching. `fallback` answers when
/// there's no logo or it can't be resized.
pub async fn send_logo(c: &mut Ctx, variation: Variation, fallback: impl FnOnce(&mut Ctx) -> Result) -> Result {
    let account = c.app().db.read(Account::first).await.map_err(Error::internal)?;
    let blob = match &account {
        Some(account) => {
            let id = account.id;
            c.app().db.read(move |conn| attachments::attached_blob(conn, "Account", id, "logo"))
                .await.map_err(Error::internal)?
        }
        None => None,
    };
    let private = match &blob {
        Some(blob) => campfire_runtime::active_storage::authorize_private_media(c, blob.id).await?,
        None => false,
    };

    // `stale?(etag: Current.account)`; there's no accounts/logos/show template to digest.
    let freshness = Freshness {
        etag: account.as_ref().map(|account| cache_key_with_version("accounts", account.id, account.updated_at.jiff())),
        ..Freshness::default()
    };
    if !private && let Some(not_modified) = c.fresh_when(freshness) {
        return Ok(not_modified);
    }
    c.expires_in(MAX_AGE, ExpiresIn { public: true, stale_while_revalidate: Some(STALE_WHILE_REVALIDATE), ..ExpiresIn::default() });

    let variant = match blob.filter(|blob| blob.is_variable()) {
        // `logo.variant(size).processed if logo.variable?`
        Some(blob) => {
            let timeout = campfire_storage::branding::processing_timeout(&blob);
            campfire_runtime::active_storage::processed_branding_variant_with_deadline(
                c.app(),
                blob,
                variation,
                timeout,
                |storage, blob, variation, cancel| {
                    campfire_storage::branding::transform_variant(
                        storage,
                        blob,
                        campfire_storage::branding::Kind::Logo,
                        variation,
                        cancel,
                    )
                },
            )
            .await
            .ok()
        }
        None => None,
    };
    let response = match variant {
        Some(variant) => {
            let path = c.app().storage.service.path_for(&variant.key);
            c.send_file(path, SendOptions::inline("image/png"))
        }
        None => fallback(c),
    }?;
    Ok(if private { response.header("cache-control", "private, no-store") } else { response })
}

pub(super) fn stock_icon(c: &mut Ctx, small: bool) -> Result {
    let filename = if small {
        "app-icon-192.png"
    } else {
        "app-icon.png"
    };
    let path = asset_file(&format!("logos/{filename}"))?;
    c.send_file(path, SendOptions::inline("image/png"))
}

/// `Current.account.logo.destroy`
pub async fn destroy(c: &mut Ctx) -> Result {
    c.use_live_response(); // `include ActiveStorage::Streaming`
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let account = super::current_account(c).await?;
    let secrets = c.app().secrets.clone();
    let audit = crate::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            attachments::destroy(tx, Record::account(account.id, &secrets), "logo")?;
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    // Rails destroys the attachment before its separate audit write.
    c.app().db.write(move |tx| crate::account_security::logo_removed(tx, &account, &audit)).await.map_err(Error::internal)?;
    crate::controllers::presenters::workspace_branding::publish(c.app()).await;
    let location = c.url_for(&campfire_routes::edit_account());
    c.redirect_to(&location)
}
