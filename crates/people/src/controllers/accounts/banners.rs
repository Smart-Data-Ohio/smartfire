//! Workspace banner and the SPA's animated/still logo responses.

use campfire_kit::{Ctx, Error, ExpiresIn, Freshness, Result, SendOptions};
use campfire_storage::Variation;

use crate::app::AppCtx;
use crate::concerns::{self, Before};
use crate::controllers::presenters::attachments;

pub async fn show(c: &mut Ctx) -> Result {
    c.use_live_response();
    concerns::before_actions(c, Before::default()).await?;
    show_image(c, "banner").await
}

pub(super) async fn show_image(c: &mut Ctx, name: &'static str) -> Result {
    let account = super::current_account(c).await?;
    let id = account.id;
    let blob = c
        .app()
        .db
        .read(move |conn| attachments::attached_blob(conn, "Account", id, name))
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)?;
    let storage = c.app().storage.clone();
    let source = blob.clone();
    let animated = tokio::task::spawn_blocking(move || storage.is_animated(&source))
        .await
        .map_err(Error::internal)?
        .map_err(Error::internal)?;
    let still = c.param_str("still") == Some("1");
    let small = name == "logo" && c.param_str("size") == Some("small");
    let original = animated && !still;
    if let Some(response) = c.fresh_when(Freshness {
        etag: Some(format!(
            "workspace-{name}-{}-{}-{small}",
            blob.id,
            if original {
                "original"
            } else if still {
                "still"
            } else {
                "variant"
            }
        )),
        ..Freshness::default()
    }) {
        return Ok(response);
    }
    c.expires_in(
        5 * 60,
        ExpiresIn {
            public: name == "logo",
            stale_while_revalidate: Some(7 * 24 * 60 * 60),
            ..ExpiresIn::default()
        },
    );
    let image = if original {
        blob
    } else {
        // Classic forms may have attached a BMP or another non-variable logo.
        if !blob.is_variable() {
            if name != "logo" {
                return Err(Error::NotFound);
            }
            let filename = if small {
                "app-icon-192.png"
            } else {
                "app-icon.png"
            };
            let path =
                crate::controllers::users::avatars::asset_file(&format!("logos/{filename}"))?;
            return c.send_file(path, SendOptions::inline("image/png"));
        }
        let (width, height) = if name == "banner" {
            (1920, 1080)
        } else if small {
            (192, 192)
        } else {
            (512, 512)
        };
        let format = if still || name == "logo" {
            "png".to_string()
        } else {
            blob.default_variant_format()
        };
        campfire_web::active_storage::processed_representation(
            c.app(),
            blob,
            Variation::resize_to_limit(width, height, Some(&format)),
        )
        .await?
    };
    c.send_file(
        c.app().storage.service.path_for(&image.key),
        SendOptions::inline(image.content_type()),
    )
}
