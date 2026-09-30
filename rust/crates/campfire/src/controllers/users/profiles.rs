//! `Users::ProfilesController` (reference/app/controllers/users/profiles_controller.rb): the
//! signed-in user's own profile.

use campfire_db::UserChanges;
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode, format, permit_keys};
use campfire_views::users;

use crate::app::AppCtx;
use crate::concerns::{self, Before};
use crate::controllers::presenters::attachments::{self, Assignment, Record};
use crate::controllers::presenters::page::framed_page;
use crate::controllers::presenters::{self, accounts::string_attribute};

/// `set_user` (`Current.user`); memberships partitioned into direct and shared rooms.
pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let user = concerns::require_current_user(c)?.clone();
    render_show(c, StatusCode::OK, user, None, None).await
}

async fn render_show(
    c: &mut Ctx,
    status: StatusCode,
    user: campfire_db::User,
    current_password_error: Option<&'static str>,
    settings_error: Option<(
        campfire_db::models::user::profile_settings::Changes,
        campfire_db::Errors,
    )>,
) -> Result {
    c.respond_to(&[&format::HTML])?;
    let has_password = user
        .password_digest
        .as_deref()
        .is_some_and(|s| !campfire_richtext::ruby::is_blank(s));
    let secrets = c.app().secrets.clone();
    let transfer_id = presenters::accounts::transfer_id(&secrets, user.id, c.now());
    let (avatar_attached, (direct_memberships, shared_memberships)) = {
        let user = user.clone();
        c.app()
            .db
            .read(move |conn| {
                let attached =
                    attachments::attached_blob(conn, "User", user.id, "avatar")?.is_some();
                Ok((
                    attached,
                    presenters::accounts::profile_memberships(conn, &user)?,
                ))
            })
            .await
            .map_err(Error::internal)?
    };
    let id = user.id;
    let now = c.now();
    let google = c.app().two_factor.google().is_some();
    let mut appearance = c
        .app()
        .db
        .read(move |conn| campfire_db::models::user::profile_settings::appearance(conn, id))
        .await
        .map_err(Error::internal)?;
    let preview_settings = settings_error.as_ref().map(|(changes, _)| changes.clone());
    let errors = if let Some((changes, errors)) = settings_error {
        if let Some(theme) = changes.theme {
            appearance.theme = theme;
        }
        if let Some(text_size) = changes.text_size {
            appearance.text_size = text_size;
        }
        if let Some(time_zone) = changes.time_zone {
            appearance.time_zone = Some(time_zone);
        }
        errors
    } else {
        campfire_db::Errors::default()
    };
    let appearance = users::AppearanceData {
        theme: appearance.theme,
        text_size: appearance.text_size,
        zone_identifier: appearance
            .time_zone
            .as_deref()
            .and_then(campfire_db::models::user::profile_settings::zone_identifier),
        theme_errors: errors.on("theme").into_iter().map(str::to_owned).collect(),
        text_size_errors: errors
            .on("text_size")
            .into_iter()
            .map(str::to_owned)
            .collect(),
        time_zone_errors: errors
            .on("time_zone")
            .into_iter()
            .map(str::to_owned)
            .collect(),
    };
    let security = c
        .app()
        .db
        .read(move |conn| {
            let credential = campfire_db::TwoFactorCredential::for_user(conn, id)?;
            let devices = campfire_db::TwoFactorRememberedDevice::for_user(conn, id)?
                .into_iter()
                .filter(|d| d.expires_at.jiff() > now)
                .map(|d| campfire_views::two_factor::Device {
                    id: d.id,
                    user_agent: d.user_agent,
                    ip_address: d.ip_address,
                    last_used_at: d.last_used_at.map(|t| t.jiff()),
                })
                .collect();
            Ok(campfire_views::two_factor::ProfileData {
                confirmed_at: credential.and_then(|c| c.confirmed_at).map(|t| t.jiff()),
                devices,
                google: google
                    && conn.query_row(
                        "SELECT EXISTS(SELECT 1 FROM google_identities WHERE user_id=?)",
                        [id],
                        |r| r.get::<_, bool>(0),
                    )?,
            })
        })
        .await
        .map_err(Error::internal)?;
    let mut sections = c
        .app()
        .db
        .read(move |conn| presenters::profile_sections::load(conn, id, now))
        .await
        .map_err(Error::internal)?;
    // WS9 owns sign-in configuration and identity rows directly; WS14g completes Calendar.
    sections.google.sign_in_configured = google;
    if google {
        sections.google.identity_email = c
            .app()
            .db
            .read(move |conn| {
                use rusqlite::OptionalExtension;
                Ok(conn
                    .query_row(
                        "SELECT email FROM google_identities WHERE user_id=?",
                        [id],
                        |r| r.get(0),
                    )
                    .optional()?)
            })
            .await
            .map_err(Error::internal)?;
    }
    if let Some(changes) = preview_settings {
        presenters::profile_sections::preview(&mut sections, &changes, &errors);
    }
    let user = presenters::user_summary(&secrets, &user);
    framed_page!(c, status, |ctx| users::ProfileShow {
        ctx,
        sections: sections.clone(),
        appearance: appearance.clone(),
        has_password,
        current_password_error,
        security: security.clone(),
        now,
        user: user.clone(),
        avatar_attached,
        transfer_id: transfer_id.clone(),
        shared_memberships: shared_memberships.clone(),
        direct_memberships: direct_memberships.clone(),
    })
    .await
}

/// `@user.update user_params`, then `redirect_to user_profile_url, notice: update_notice`.
pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let mut user = concerns::require_current_user(c)?.clone();

    // Rails computes presence/key checks before strong parameters discard non-scalars.
    let raw = c.params.require("user")?;
    let password_changing = raw.get("password").is_some_and(|p| p.is_present());
    let time_zone_submitted = raw.get("time_zone").is_some();
    // `user_params.compact`: nil scalar values are dropped below.
    let mut permitted = permit_keys(&[
        "name",
        "avatar",
        "email_address",
        "password",
        "bio",
        "time_zone",
        "theme",
        "text_size",
        "voice_mode",
        "push_to_talk_key",
        "github_login",
    ]);
    permitted.push(campfire_kit::Permit::Nested(
        "inbox_preferences".into(),
        permit_keys(campfire_db::models::user::profile_settings::INBOX_KEYS),
    ));
    let params = c.params.require("user")?.permit(&permitted);
    let settings = campfire_db::models::user::profile_settings::Changes {
        theme: compact_string(&params, "theme"),
        text_size: compact_string(&params, "text_size"),
        time_zone: compact_string(&params, "time_zone"),
        voice_mode: compact_string(&params, "voice_mode"),
        push_to_talk_key: compact_string(&params, "push_to_talk_key"),
        github_login: compact_string(&params, "github_login"),
        inbox_preferences: params
            .get("inbox_preferences")
            .filter(|p| !p.is_null())
            .map(|p| p.to_json()),
    };
    // Rails checks the raw request before strong parameters discard non-scalars.
    let email_changing = c
        .params
        .get("user")
        .and_then(|p| p.get("email_address"))
        .is_some_and(|p| {
            user.email_change_requested(
                &p.to_s()
                    .unwrap_or_else(|| campfire_richtext::ruby::json_value_inspect(&p.to_json())),
            )
        });
    if email_changing {
        let current_password = c.params.get("user").and_then(|p| p.get("current_password"));
        let missing = current_password.is_none_or(|p| !p.is_present());
        let password = current_password.and_then(|p| p.to_s()).unwrap_or_default();
        let existing = user.clone();
        let confirmed =
            tokio::task::spawn_blocking(move || existing.current_password_confirmed(&password))
                .await
                .map_err(Error::internal)?;
        if !confirmed {
            // Rails assigns the submitted non-secret attributes only to the error view.
            assign_error_attributes(&mut user, &params);
            c.set_current(concerns::CurrentUser(user.clone()));
            return render_show(
                c,
                StatusCode::UNPROCESSABLE_ENTITY,
                user,
                Some(if missing {
                    "is required to change your email address"
                } else {
                    "is incorrect"
                }),
                Some((settings, campfire_db::Errors::default())),
            )
            .await;
        }
    }
    let audit = crate::controllers::two_factor::audit_context(c)?;
    let present = |key: &str| compact_string(&params, key);
    let changes = UserChanges {
        name: present("name"),
        email_address: present("email_address").map(Some),
        // `password=` ignores a blank password.
        password_digest: concerns::password_digest(
            c,
            present("password").filter(|password| !password.is_empty()),
        )
        .await?,
        bio: present("bio").map(Some),
        time_zone: present("time_zone").map(Some),
        time_zone_explicit: time_zone_submitted.then_some(true),
        ..UserChanges::default()
    };
    let avatar = match Assignment::from_params(&params, "avatar")? {
        // `.compact` drops a nil avatar before it's assigned.
        Assignment::Delete if params.get("avatar").is_none_or(|p| p.is_null()) => {
            Assignment::Unchanged
        }
        assignment => assignment,
    };
    // `params[:user][:avatar] ? ... : "✓"`: any non-nil value counts.
    let notice = match c.params.get("user").and_then(|user| user.get("avatar")) {
        Some(avatar) if !avatar.is_null() => "It may take up to 30 minutes to change everywhere.",
        _ => "✓",
    };

    let avatar = avatar.stage(c.app()).await?;
    let preview_settings = settings.clone();
    let mut error_user = user.clone();
    assign_error_attributes(&mut error_user, &params);
    let pending = c
        .app()
        .db
        .write(move |tx| {
            crate::authentication::update_profile(
                tx,
                &mut user,
                changes,
                email_changing,
                password_changing,
                &audit,
            )?;
            campfire_db::models::user::profile_settings::update(tx, user.id, settings)?;
            attachments::assign(tx, Record::user(user.id), "avatar", avatar)
        })
        .await;
    let pending = match pending {
        Ok(pending) => pending,
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            c.set_current(concerns::CurrentUser(error_user.clone()));
            return render_show(
                c,
                StatusCode::UNPROCESSABLE_ENTITY,
                error_user,
                None,
                Some((preview_settings, errors)),
            )
            .await;
        }
        Err(error) => return Err(Error::internal(error)),
    };
    attachments::analyze_later(c.app(), pending);

    let location = c.url_for(&campfire_routes::user_profile());
    c.redirect_to_with(
        &location,
        Redirect {
            notice: Some(notice.into()),
            ..Redirect::default()
        },
    )
}

/// Submitted public attributes stay visible after a rejected save; passwords stay blank.
fn assign_error_attributes(user: &mut campfire_db::User, params: &campfire_kit::ParamMap) {
    if let Some(value) = compact_string(params, "name") {
        user.name = value;
    }
    if let Some(value) = compact_string(params, "email_address") {
        user.email_address = Some(value);
    }
    if let Some(value) = compact_string(params, "bio") {
        user.bio = Some(value);
    }
}

/// `user_params.compact` drops nil instead of coercing it to an empty string.
fn compact_string(params: &campfire_kit::ParamMap, key: &str) -> Option<String> {
    params.get(key).filter(|p| !p.is_null())?;
    string_attribute(params, key).flatten()
}
