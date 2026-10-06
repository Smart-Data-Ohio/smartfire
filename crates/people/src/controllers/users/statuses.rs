//! Users::StatusesController: submitted values survive errors; side effects share the save.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
};
use campfire_db::{Errors, UserStatusSettings};
use campfire_kit::{Ctx, Error, Param, Redirect, Result, StatusCode, format, permit_keys};
use super::super::presenters::{self, page};
use campfire_views::users;
use askama::Template;

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
    let params = raw.permit(&permit_keys(&[
        "presence_setting",
        "custom_status_emoji",
        "custom_status_text",
        "custom_status_expires_in",
        "meeting_status_enabled",
        "ooo_calendar_enabled",
    ]));
    let mut user = c
        .app()
        .db
        .read(move |conn| UserStatusSettings::find(conn, id))
        .await
        .map_err(Error::internal)?;
    let original_status = (
        user.presence_setting.clone(),
        user.custom_status_emoji.clone(),
        user.custom_status_text.clone(),
        user.custom_status_expires_at,
    );
    let now = c.app().db.env().now();
    let mut errors = Errors::default();
    let mut nulls = Vec::new();
    for (key, value) in params.iter() {
        match key.as_str() {
            "presence_setting" => user.presence_setting = string(value).unwrap_or_default(),
            "custom_status_emoji" => user.custom_status_emoji = string(value),
            "custom_status_text" => user.custom_status_text = string(value),
            "custom_status_expires_in" => {
                if user
                    .set_custom_status_expires_in(&value.to_s().unwrap_or_default(), now)
                    .is_err()
                {
                    errors.add("custom_status_expires_in", "is not valid");
                    break;
                }
            }
            "meeting_status_enabled" | "ooo_calendar_enabled" => {
                let cast = value
                    .to_s()
                    .map(|v| campfire_db::models::account::cast_boolean(&v))
                    .unwrap_or(Some(true));
                if cast.is_none() {
                    nulls.push(key.clone());
                }
                match key.as_str() {
                    "meeting_status_enabled" => user.meeting_status_enabled = cast.unwrap_or(false),
                    _ => user.ooo_calendar_enabled = cast.unwrap_or(false),
                };
            }
            _ => {}
        }
    }
    if errors.is_empty() {
        if raw
            .get("clear_custom_status")
            .is_some_and(|p| !p.is_blank())
        {
            user.custom_status_emoji = None;
            user.custom_status_text = None;
            user.custom_status_expires_at = None;
        }
        if raw.get("clear_ooo").is_some_and(|p| !p.is_blank()) {
            user.ooo_until = None;
            user.ooo_note = None;
        } else if let Some(preset) = raw.get("ooo_preset").filter(|p| !p.is_blank()) {
            let preset = super::notification_settings::ruby_to_s(preset);
            if !["tomorrow", "monday", "week", "custom"].contains(&preset.as_str()) {
                errors.add("ooo_until", "is not valid");
            } else {
                let custom = raw
                    .get("ooo_until_custom")
                    .map(super::notification_settings::ruby_to_s);
                let end = user
                    .ooo_preset_until(&preset, custom.as_deref(), now)
                    .map_err(Error::internal)?;
                if end.is_none_or(|t| t <= now) {
                    errors.add("ooo_until", "needs a future date and time");
                } else {
                    user.ooo_until = end;
                    user.ooo_note = raw
                        .get("ooo_note")
                        .filter(|p| !p.is_blank())
                        .and_then(string);
                }
            }
        } else if let Some(note) = raw.get("ooo_note") {
            user.ooo_note = (!note.is_blank()).then(|| string(note)).flatten();
        }
    }
    if !errors.is_empty() {
        return render_rejected(c, user, errors).await;
    }
    let submitted = user.clone();
    // Users::StatusesController#broadcast_status_change checks only STATUS_ATTRIBUTES.
    let status_changed = original_status
        != (
            user.presence_setting.clone(),
            user.custom_status_emoji.clone(),
            user.custom_status_text.clone(),
            user.custom_status_expires_at,
        );
    let saved = c
        .app()
        .db
        .write(move |tx| {
            user.save_status(tx)?;
            for key in nulls {
                tx.conn()
                    .execute(&format!("UPDATE users SET {key}=NULL WHERE id=?"), [id])?;
            }
            if status_changed {
                user.announce_badge(tx)?;
            }
            Ok(())
        })
        .await;
    match saved {
        Ok(()) => after_save(c, id),
        Err(campfire_db::Error::RecordInvalid(errors)) => render_rejected(c, submitted, errors).await,
        Err(error) => Err(Error::internal(error)),
    }
}

fn string(value: &Param) -> Option<String> {
    match value {
        Param::Null => None,
        Param::Bool(true) => Some("t".into()),
        Param::Bool(false) => Some("f".into()),
        value => Some(super::notification_settings::ruby_to_s(value)),
    }
}

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

async fn render_rejected(c: &mut Ctx, user: UserStatusSettings, errors: Errors) -> Result {
    if c.is_turbo_frame_request() {
        let fields = presenters::profile_sections::status_fields(&user, &errors, c.app().db.env().now());
        render_edit(c, StatusCode::UNPROCESSABLE_ENTITY, user.user.id, fields).await
    } else {
        super::profiles::render_settings(c, StatusCode::UNPROCESSABLE_ENTITY, user, errors).await
    }
}
