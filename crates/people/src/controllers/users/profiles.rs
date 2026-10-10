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
/// `/users/me/profile` and `/users/:own_id/profile` are that page. Another person's id goes to
/// their page (`/users/:id`, or the SPA people screen when they use the new UI).
pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    if let Some(id) = other_person(c, viewer.id).await? {
        return redirect_to_person(c, &viewer, id).await;
    }
    render_show(c, StatusCode::OK, viewer, None, None, None).await
}

/// The SPA's profile edit alias. Classic has no edit action and keeps its 404.
pub async fn edit(c: &mut Ctx) -> Result {
    if !concerns::coexistence_navigation(c)? {
        return Err(Error::NotFound);
    }
    let Some(session) = concerns::find_session_by_cookie(c).await? else {
        return Err(Error::NotFound);
    };
    let user_id = session.user_id;
    let user = c.app().db.read(move |conn| campfire_db::User::find_by_id(conn, user_id))
        .await.map_err(Error::internal)?;
    let Some(user) = user else { return Err(Error::NotFound) };
    if user.is_bot() || concerns::session_expired(
        &session,
        &user,
        c.app().config.admin_session_idle_timeout,
        campfire_db::Timestamp::from_jiff(c.now()),
    ) || !concerns::next_ui(c, &user).await? {
        return Err(Error::NotFound);
    }
    concerns::before_actions(c, Before::default()).await?;
    Err(Error::NotFound)
}

/// `Some(id)` when `:user_id` names a different person. `me` and the viewer's own id are `None`.
async fn other_person(c: &Ctx, viewer_id: i64) -> Result<Option<i64>> {
    let Some(param) = c.param_str("user_id") else {
        return Ok(None);
    };
    if param == "me" {
        return Ok(None);
    }
    let id = concerns::cast_integer(param).ok_or(Error::NotFound)?;
    if id == viewer_id {
        return Ok(None);
    }
    let found = c
        .app()
        .db
        .read(move |conn| campfire_db::User::find_by_id(conn, id))
        .await
        .map_err(Error::internal)?;
    if found.is_none() {
        return Err(Error::NotFound);
    }
    Ok(Some(id))
}

/// Another person's profile alias. The hop is their page, query included (`classic=1` and the
/// rest). That page's coexistence redirect applies the navigation and pending-flash guards; this
/// action doesn't choose the UI itself.
async fn redirect_to_person(c: &mut Ctx, _viewer: &campfire_db::User, id: i64) -> Result {
    let mut path = format!("/users/{id}");
    let query = c.request.query_string();
    if !query.is_empty() {
        path.push('?');
        path.push_str(query);
    }
    c.redirect_to(&c.url_for(&path))
}

async fn render_show(
    c: &mut Ctx,
    status: StatusCode,
    user: campfire_db::User,
    current_password_error: Option<&'static str>,
    status_preview: Option<users::StatusFields>,
    settings_error: Option<(
        campfire_db::models::user::profile_settings::Changes,
        campfire_db::Errors,
    )>,
) -> Result {
    c.respond_to(&[&format::HTML])?;
    if let Some((changes, _)) = &settings_error {
        let id = user.id;
        let mut rendered = c
            .app()
            .db
            .read(move |conn| campfire_db::UserStatusSettings::find(conn, id))
            .await
            .map_err(Error::internal)?;
        if let Some(value) = &changes.theme {
            rendered.theme = value.clone();
        }
        if let Some(value) = &changes.text_size {
            rendered.text_size = value.clone();
        }
        if let Some(value) = &changes.time_zone {
            rendered.time_zone = Some(value.clone());
        }
        c.set_current(presenters::view_context::RenderedSettings(rendered));
    }
    let has_password = presenters::accounts::profile_has_password(&user);
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
    let rendered = c
        .current::<presenters::view_context::RenderedSettings>()
        .cloned();
    let owned = match rendered {
        Some(data) => data.0,
        None => c
            .app()
            .db
            .read(move |conn| campfire_db::UserStatusSettings::find(conn, id))
            .await
            .map_err(Error::internal)?,
    };
    let owned_errors = c
        .current::<RenderedErrors>()
        .map(|data| data.0.clone())
        .unwrap_or_default();
    let source = owned.clone();
    let failures = owned_errors.clone();
    let configured = c.app().google.api().config.configured();
    let settings_form = c
        .app()
        .db
        .read(move |conn| {
            presenters::status_settings::forms(
                conn,
                &source,
                failures,
                campfire_db::Timestamp::from_jiff(now),
                configured,
            )
        })
        .await
        .map_err(Error::internal)?;

    let google = c.app().google.sign_in().config.configured();
    let google_reauthentication = c.app().two_factor.google().is_some();
    let mut appearance = c
        .app()
        .db
        .read(move |conn| campfire_db::models::user::profile_settings::appearance(conn, id))
        .await
        .map_err(Error::internal)?;
    if c.current::<presenters::view_context::RenderedSettings>()
        .is_some()
    {
        appearance.theme = owned.theme.clone();
        appearance.text_size = owned.text_size.clone();
        appearance.time_zone = owned.time_zone.clone();
    }
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
        owned_errors.clone()
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
        // The SPA is everyone's UI: no switch between the two.
        next_ui: None,
    };
    let security = c
        .app()
        .db
        .read(move |conn| {
            presenters::accounts::profile_two_factor(conn, id, now, google_reauthentication)
        })
        .await
        .map_err(Error::internal)?;
    let mut sections = c
        .app()
        .db
        .read(move |conn| presenters::profile_sections::load(conn, id, now))
        .await
        .map_err(Error::internal)?;
    // Google profile controls use the same injected providers as their endpoints.
    sections.google.sign_in_configured = google;
    sections.google.calendar_configured = c.app().google.api().config.configured();
    sections.status = presenters::profile_sections::status_fields(
        &owned,
        &owned_errors,
        campfire_db::Timestamp::from_jiff(now),
    );
    sections.status.fetch_error = settings_form
        .fetch_error
        .clone()
        .filter(|s| !campfire_richtext::ruby::is_blank(s));
    if let Some(fields) = status_preview {
        sections.status = fields;
    }
    if let Some(changes) = preview_settings {
        presenters::profile_sections::preview(&mut sections, &changes, &errors);
    }
    sections.fizzy = presenters::fizzy_profile::connection(c.app(), user.id)
        .await
        .map_err(Error::internal)?;
    let github = presenters::github::connection(c.app(), user.id)
        .await
        .map_err(Error::internal)?;
    let zone = presenters::view_context::time_zone(c).await?;
    let user = presenters::user_summary_in_zone(&secrets, &user, &zone);
    framed_page!(c, status, |ctx| users::ProfileShow {
        ctx,
        github: github.clone(),
        settings: settings_form.clone(),
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
    let github_verified = c
        .app()
        .db
        .read(move |conn| {
            Ok(
                crate::integrations::github::accounts::Account::for_user(conn, user.id)?
                    .is_some_and(|account| account.connected()),
            )
        })
        .await
        .map_err(Error::internal)?;
    let settings = campfire_db::models::user::profile_settings::Changes {
        theme: compact_string(&params, "theme"),
        text_size: compact_string(&params, "text_size"),
        time_zone: compact_string(&params, "time_zone"),
        voice_mode: compact_string(&params, "voice_mode"),
        push_to_talk_key: compact_string(&params, "push_to_talk_key"),
        github_login: if github_verified {
            None
        } else {
            compact_string(&params, "github_login")
        },
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
                None,
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
    let result = c
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
    match result {
        Ok(()) => {}
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            c.set_current(concerns::CurrentUser(error_user.clone()));
            return render_show(
                c,
                StatusCode::UNPROCESSABLE_ENTITY,
                error_user,
                None,
                None,
                Some((preview_settings, errors)),
            )
            .await;
        }
        Err(error) => return Err(Error::internal(error)),
    };

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

#[derive(Clone)]
struct RenderedErrors(campfire_db::Errors);
pub(super) async fn render_settings(
    c: &mut Ctx,
    status: StatusCode,
    settings: campfire_db::UserStatusSettings,
    errors: campfire_db::Errors,
) -> Result {
    if !errors.is_empty() && settings.user.role != campfire_db::Role::Bot {
        let id = settings.user.id;
        let enabled = c.app().db.read(move |conn| Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM two_factor_credentials WHERE user_id=? AND confirmed_at IS NOT NULL)", [id], |r| r.get::<_, bool>(0))?)).await.map_err(Error::internal)?;
        if enabled {
            return Err(Error::internal(anyhow::anyhow!(
                "profiles/two_factor: undefined method any? for nil remembered devices"
            )));
        }
    }
    let user = settings.user.clone();
    c.set_current(presenters::view_context::RenderedSettings(settings));
    c.set_current(RenderedErrors(errors));
    render_show(c, status, user, None, None, None).await
}
