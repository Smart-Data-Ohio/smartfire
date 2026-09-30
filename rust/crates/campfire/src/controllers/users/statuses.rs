//! #163 popup HTTP behavior; domain status/Calendar integration remains a flagged WS17 seam.
use super::super::presenters::{self, page};
use crate::{
    app::AppCtx,
    concerns::{self, Before},
};
use askama::Template;
use campfire_db::{Errors, models::user::status_form::StatusForm};
use campfire_kit::{Ctx, Error, Param, Redirect, Result, StatusCode, format, permit_keys};
use campfire_views::users;
pub async fn edit(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let id = concerns::require_current_user(c)?.id;
    let now = c.now();
    let fields = c
        .app()
        .db
        .read(move |conn| Ok(presenters::profile_sections::load(conn, id, now)?.status))
        .await
        .map_err(Error::internal)?;
    render_edit(c, StatusCode::OK, id, fields).await
}
async fn render_edit(
    c: &mut Ctx,
    status: StatusCode,
    id: i64,
    fields: users::StatusFields,
) -> Result {
    c.respond_to(&[&format::HTML])?;
    page::bare(c, status, &format::HTML, |_| {
        users::StatusEdit {
            user_id: id,
            fields: fields.clone(),
        }
        .render()
    })
    .await
}
pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let id = concerns::require_current_user(c)?.id;
    let raw = c.params.require("user")?.as_hash().ok_or_else(|| {
        Error::internal(anyhow::anyhow!(
            "undefined method permit for user parameter"
        ))
    })?;
    // WS17 owns these pre-existing profile-only mutations and their Calendar side effects.
    // Its controller must call render_invalid/after_save below after save_status succeeds.
    if [
        "meeting_status_enabled",
        "ooo_calendar_enabled",
        "ooo_preset",
        "ooo_until_custom",
        "ooo_note",
        "clear_ooo",
    ]
    .iter()
    .any(|key| raw.get(key).is_some())
    {
        c.set_header("x-campfire-unported", "users/statuses#WS17-calendar");
        return Ok(c.head(StatusCode::NOT_IMPLEMENTED));
    }
    let clear = raw
        .get("clear_custom_status")
        .is_some_and(|p| p.is_present());
    let params = raw.permit(&permit_keys(&[
        "presence_setting",
        "custom_status_emoji",
        "custom_status_text",
        "custom_status_expires_in",
    ]));
    let mut form = c
        .app()
        .db
        .read(move |conn| StatusForm::load(conn, id))
        .await
        .map_err(Error::internal)?;
    let mut errors = Errors::default();
    for (key, value) in params.iter() {
        let string = match value {
            Param::Null => None,
            Param::Bool(v) => Some(if *v { "t" } else { "f" }.into()),
            _ => value.to_s(),
        };
        match key.as_str() {
            "presence_setting" => form.presence = string,
            "custom_status_emoji" => form.emoji = string,
            "custom_status_text" => form.text = string,
            "custom_status_expires_in" => {
                if let Err(campfire_db::Error::RecordInvalid(found)) = form.set_expiry(
                    &value.to_s().unwrap_or_default(),
                    campfire_db::Timestamp::from_jiff(c.now()),
                ) {
                    errors = found;
                    break;
                }
            }
            _ => {}
        }
    }
    if errors.is_empty() && clear {
        form.emoji = None;
        form.text = None;
        form.expiry = None;
    }
    if errors.is_empty() {
        let submitted = form.clone();
        match c.app().db.write(move |tx| submitted.save(tx, id)).await {
            Ok(()) => return after_save(c, id),
            Err(campfire_db::Error::RecordInvalid(found)) => errors = found,
            Err(error) => return Err(Error::internal(error)),
        }
    }
    let now = c.now();
    let mut fields = c
        .app()
        .db
        .read(move |conn| Ok(presenters::profile_sections::load(conn, id, now)?.status))
        .await
        .map_err(Error::internal)?;
    fields.presence = form.presence.unwrap_or_default();
    fields.emoji = form.emoji;
    fields.text = form.text;
    for (key, message) in errors.0 {
        fields.errors.entry(key.into()).or_default().push(message);
    }
    render_invalid(c, fields).await
}
/// WS17 call site after its full status/Calendar writer succeeds. Frame saves have no flash.
pub fn after_save(c: &mut Ctx, id: i64) -> Result {
    if c.is_turbo_frame_request() {
        c.redirect_to_with(
            &c.url_for(&campfire_routes::user_card(id)),
            Redirect {
                status: Some(StatusCode::SEE_OTHER),
                ..Default::default()
            },
        )
    } else {
        c.redirect_to_with(
            &c.url_for(&campfire_routes::user_profile()),
            Redirect {
                notice: Some("✓".into()),
                ..Default::default()
            },
        )
    }
}
/// WS17 supplies the rejected, unsaved display values and ordered error messages.
pub async fn render_invalid(c: &mut Ctx, fields: users::StatusFields) -> Result {
    if c.is_turbo_frame_request() {
        render_edit(
            c,
            StatusCode::UNPROCESSABLE_ENTITY,
            concerns::require_current_user(c)?.id,
            fields,
        )
        .await
    } else {
        super::profiles::render_status_error(c, fields).await
    }
}
