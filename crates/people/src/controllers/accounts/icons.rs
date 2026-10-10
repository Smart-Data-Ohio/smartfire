//! `Accounts::IconsController`: staged uploads, transactional records and read-only forms.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    controllers::presenters::{
        attachments::{self, Assignment, Record},
        page::{db_error, framed_page},
    },
};
use campfire_db::{
    Errors,
    models::audit_log::{AuditLog, NewAuditLog, Target},
    models::workspace_icon::{ImageFacts, NewIcon, WorkspaceIcon},
};
use campfire_kit::{Ctx, Error, Param, Redirect, Result, StatusCode, format, permit_keys};
use campfire_views::accounts::icons as views;

pub async fn index(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    render(c, StatusCode::OK, NewIcon::default(), Errors::default()).await
}
async fn render(c: &mut Ctx, status: StatusCode, icon: NewIcon, errors: Errors) -> Result {
    c.respond_to(&[&format::HTML])?;
    let icons = c
        .app()
        .db
        .read(WorkspaceIcon::ordered)
        .await
        .map_err(Error::internal)?
        .into_iter()
        .map(|i| views::Icon {
            id: i.id,
            name: i.name,
            title: i.title,
            creator_name: i.creator_name,
        })
        .collect::<Vec<_>>();
    let icon = views::Form {
        name: icon.name,
        title: icon.title,
        invalid_fields: ["name", "title", "image"]
            .into_iter()
            .filter(|f| !errors.on(f).is_empty())
            .map(str::to_owned)
            .collect(),
        errors: errors.full_messages(),
    };
    framed_page!(c, status, |ctx| views::Index {
        ctx,
        icons: icons.clone(),
        icon: icon.clone()
    })
    .await
}
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let params = c
        .params
        .require("workspace_icon")?
        .permit(&permit_keys(&["name", "title", "image"]));
    let icon = NewIcon {
        name: params.get("name").and_then(Param::to_s),
        title: params.get("title").and_then(Param::to_s),
        creator_id: concerns::require_current_user(c)?.id,
    }
    .normalized();
    let assignment = Assignment::from_params(&params, "image")?
        .stage(c.app())
        .await?;
    let (image, prepared) = image_facts(c, &assignment).await?;
    let audit = super::super::two_factor::audit_context(c)?;
    let input = icon.clone();
    let brand = crate::rich_text::builtin_icon(icon.name.as_deref().unwrap_or(""));
    let storage = c.app().storage.clone();
    let result = c
        .app()
        .db
        .write(move |tx| {
            let icon = input.save(tx, brand, image.as_ref())?;
            attachments::assign(tx, Record::workspace_icon(icon.id), "image", assignment)?;
            save_image(tx, &storage, icon.id, prepared)?;
            Ok(icon)
        })
        .await;
    match result {
        Ok(icon) => {
            // Rails commits the icon and its attachment callbacks before recording the audit.
            // An audit failure returns 500 without undoing the completed mutation.
            c.app()
                .db
                .write(move |tx| {
                    AuditLog::record(
                        tx,
                        NewAuditLog {
                            action: "workspace_icon.create".into(),
                            target: Some(Target {
                                record_type: "WorkspaceIcon".into(),
                                id: icon.id,
                                label: Some(format!(":{}:", icon.name)),
                            }),
                            changes: Some(serde_json::json!({"name":icon.name,"title":icon.title})),
                            ..Default::default()
                        },
                        &audit,
                    )?;
                    Ok(())
                })
                .await
                .map_err(Error::internal)?;
            let location = c.url_for(&campfire_routes::account_icons());
            c.redirect_to_with(
                &location,
                Redirect {
                    notice: Some("Icon added".into()),
                    ..Default::default()
                },
            )
        }
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            render(c, StatusCode::UNPROCESSABLE_ENTITY, icon, errors).await
        }
        Err(error) => Err(Error::internal(error)),
    }
}
pub async fn image_facts(
    c: &Ctx,
    assignment: &Assignment<campfire_storage::Staged>,
) -> Result<(
    Option<ImageFacts>,
    Option<campfire_storage::branding::PreparedEmoji>,
)> {
    let (content_type, byte_size, key, filename) = match assignment {
        Assignment::Create(staged) => {
            let b = staged.blob();
            (
                b.content_type.clone().unwrap_or_default(),
                b.byte_size,
                b.key.clone(),
                b.filename.clone(),
            )
        }
        Assignment::Existing(b) => (
            b.content_type.clone().unwrap_or_default(),
            b.byte_size,
            b.key.clone(),
            b.filename.clone(),
        ),
        _ => return Ok((None, None)),
    };
    let storage = c.app().storage.clone();
    let prepared = campfire_runtime::active_storage::process_branding_with_deadline(
        std::time::Duration::from_secs(10),
        move |cancel| {
            Ok(campfire_storage::workspace_icon::prepare(
                &storage, &key, &filename, byte_size, &cancel,
            ))
        },
    )
    .await?;
    let (content_type, content_error, animated, prepared) = match prepared {
        Ok(prepared) => (
            prepared.content_type.to_owned(),
            None,
            matches!(
                prepared
                    .metadata
                    .get(campfire_storage::workspace_icon::ANIMATED_KEY),
                Some(campfire_storage::Json::Bool(true))
            ),
            Some(prepared),
        ),
        Err(error) => (content_type, Some(error), false, None),
    };
    Ok((
        Some(ImageFacts {
            content_type,
            byte_size,
            content_error,
            animated,
        }),
        prepared,
    ))
}

/// Persist the decoded metadata and still in the icon's attachment transaction.
pub fn save_image(
    tx: &mut campfire_db::Tx<'_>,
    storage: &campfire_storage::Storage,
    icon_id: i64,
    prepared: Option<campfire_storage::branding::PreparedEmoji>,
) -> campfire_db::Result<()> {
    let Some(prepared) = prepared else {
        return Ok(());
    };
    let mut blob = attachments::attached_blob(tx.conn(), "WorkspaceIcon", icon_id, "image")?
        .ok_or(campfire_db::Error::RecordNotFound("ActiveStorage::Blob"))?;
    if prepared.still.is_some() {
        blob.metadata.merge(&prepared.metadata);
    }
    if let Some((variation, image)) = prepared.still {
        if storage
            .record_variant(tx.conn(), &blob, &variation, &image, tx.now().jiff())
            .map_err(attachments::storage_error)?
            .is_some()
        {
            campfire_runtime::active_storage::keep_after_commit(tx, image);
        } else {
            storage
                .existing_variant_file(tx.conn(), &blob, &variation)
                .map_err(attachments::storage_error)?
                .ok_or(campfire_db::Error::RecordNotFound(
                    "ActiveStorage::VariantRecord",
                ))?;
        }
    }
    blob.metadata
        .set("identified", campfire_storage::Json::Bool(true));
    tx.conn().execute(
        "UPDATE active_storage_blobs SET content_type = ?1, metadata = ?2 WHERE id = ?3",
        rusqlite::params![prepared.content_type, blob.metadata.encode(), blob.id],
    )?;
    Ok(())
}
pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let id = c
        .param_str("id")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or(Error::NotFound)?;
    let audit = super::super::two_factor::audit_context(c)?;
    let icon = c
        .app()
        .db
        .write(move |tx| {
            let icon = WorkspaceIcon::find(tx.conn(), id)?;
            attachments::destroy(tx, Record::workspace_icon(id), "image")?;
            icon.destroy(tx)?;
            Ok(icon)
        })
        .await
        .map_err(db_error)?;
    c.app()
        .db
        .write(move |tx| {
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "workspace_icon.destroy".into(),
                    target: Some(Target {
                        record_type: "WorkspaceIcon".into(),
                        id,
                        label: Some(format!(":{}:", icon.name)),
                    }),
                    changes: Some(serde_json::json!({"name":icon.name})),
                    ..Default::default()
                },
                &audit,
            )?;
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    let location = c.url_for(&campfire_routes::account_icons());
    c.redirect_to_with(
        &location,
        Redirect {
            notice: Some("Icon deleted".into()),
            ..Default::default()
        },
    )
}
