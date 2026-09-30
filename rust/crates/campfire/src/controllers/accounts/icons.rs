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
    let image = image_facts(c, &assignment).await?;
    let audit = super::super::two_factor::audit_context(c)?;
    let input = icon.clone();
    let brand = crate::rich_text::builtin_icon(icon.name.as_deref().unwrap_or(""));
    let result = c
        .app()
        .db
        .write(move |tx| {
            let icon = input.save(tx, brand, image.as_ref())?;
            let pending =
                attachments::assign(tx, Record::workspace_icon(icon.id), "image", assignment)?;
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "workspace_icon.create".into(),
                    target: Some(Target {
                        record_type: "WorkspaceIcon".into(),
                        id: icon.id,
                        label: Some(icon.name.clone()),
                    }),
                    changes: Some(serde_json::json!({"name":icon.name,"title":icon.title})),
                    ..Default::default()
                },
                &audit,
            )?;
            Ok(pending)
        })
        .await;
    match result {
        Ok(pending) => {
            attachments::analyze_later(c.app(), pending);
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
pub(super) async fn image_facts(
    c: &Ctx,
    assignment: &Assignment<campfire_storage::Staged>,
) -> Result<Option<ImageFacts>> {
    let Assignment::Create(staged) = assignment else {
        return Ok(None);
    };
    let blob = staged.blob();
    let content_type = blob.content_type.clone().unwrap_or_default();
    let path = c.app().storage.service.path_for(&blob.key);
    let kind = content_type.clone();
    let content_error = tokio::task::spawn_blocking(move || {
        campfire_storage::workspace_icon::content_error(&path, &kind)
    })
    .await
    .map_err(Error::internal)?;
    Ok(Some(ImageFacts {
        content_type,
        byte_size: blob.byte_size,
        content_error,
    }))
}
pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    let id = c
        .param_str("id")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or(Error::NotFound)?;
    let audit = super::super::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let icon = WorkspaceIcon::find(tx.conn(), id)?;
            attachments::destroy(tx, Record::workspace_icon(id), "image")?;
            icon.destroy(tx)?;
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "workspace_icon.destroy".into(),
                    target: Some(Target {
                        record_type: "WorkspaceIcon".into(),
                        id,
                        label: Some(icon.name.clone()),
                    }),
                    changes: Some(serde_json::json!({"name":icon.name})),
                    ..Default::default()
                },
                &audit,
            )?;
            Ok(())
        })
        .await
        .map_err(db_error)?;
    let location = c.url_for(&campfire_routes::account_icons());
    c.redirect_to_with(
        &location,
        Redirect {
            notice: Some("Icon deleted".into()),
            ..Default::default()
        },
    )
}
#[cfg(test)]
mod tests;
