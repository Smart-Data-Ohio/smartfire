//! `WorkspaceIconsController#show`: authenticated private bytes, checksum validator and SVG policy.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    controllers::presenters::{attachments, page::db_error},
};
use campfire_db::models::workspace_icon::WorkspaceIcon;
use campfire_kit::{Ctx, Error, ExpiresIn, Result, SendOptions, StatusCode};
pub async fn show(c: &mut Ctx) -> Result {
    // Rails declares restore_authentication after the inherited callback chain; enrollment
    // applies to any session it restores, just like its other public media endpoints.
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    if !concerns::restore_authentication(c).await? {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    let name = campfire_richtext::ruby::strip(c.param_str("name").unwrap_or("")).to_lowercase();
    let blob = c
        .app()
        .db
        .read(move |conn| {
            let Some(icon) = WorkspaceIcon::find_by_name(conn, &name)? else {
                return Ok(None);
            };
            attachments::attached_blob(conn, "WorkspaceIcon", icon.id, "image")
        })
        .await
        .map_err(db_error)?;
    let Some(mut blob) = blob else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    if c.param_str("still") == Some("1") && campfire_storage::workspace_icon::animated(&blob) {
        blob = campfire_runtime::active_storage::processed_branding_variant_with_deadline(
            c.app(),
            blob,
            campfire_storage::branding::Kind::Emoji.still_variation(),
            std::time::Duration::from_secs(10),
            |storage, blob, variation, cancel| {
                campfire_storage::branding::transform_variant(
                    storage,
                    blob,
                    campfire_storage::branding::Kind::Emoji,
                    variation,
                    cancel,
                )
            },
        )
        .await?;
    }
    c.expires_in(
        3600,
        ExpiresIn {
            ..Default::default()
        },
    );
    c.set_header(
        "etag",
        &format!("\"{}\"", blob.checksum.as_deref().unwrap_or("")),
    );
    c.set_header("x-content-type-options", "nosniff");
    if blob.content_type() == "image/svg+xml" {
        c.set_header(
            "content-security-policy",
            "default-src 'none'; style-src 'unsafe-inline'",
        );
    }
    if c.is_fresh() {
        return Ok(c.head(StatusCode::NOT_MODIFIED));
    }
    let path = c.app().storage.service.path_for(&blob.key);
    let data = tokio::fs::read(path).await.map_err(Error::internal)?;
    Ok(c.send_data(
        data,
        SendOptions {
            filename: Some(blob.filename.sanitized()),
            content_type: blob.content_type.clone(),
            disposition: Some("inline".into()),
            ..Default::default()
        },
    ))
}
