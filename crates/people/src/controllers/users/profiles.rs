//! `Users::ProfilesController` (reference/app/controllers/users/profiles_controller.rb): the
//! signed-in user's own profile.

use campfire_db::UserChanges;
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode, permit_keys};

use crate::app::AppCtx;
use crate::concerns::{self, Before};
use crate::controllers::presenters::attachments::{self, Assignment, Record};
use crate::controllers::presenters::accounts::string_attribute;

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
        ..Default::default()
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
        let password = current_password.and_then(|p| p.to_s()).unwrap_or_default();
        let existing = user.clone();
        let confirmed =
            tokio::task::spawn_blocking(move || existing.current_password_confirmed(&password))
                .await
                .map_err(Error::internal)?;
        if !confirmed {
            return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
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
        Err(campfire_db::Error::RecordInvalid(_errors)) => {
            return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
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

/// `user_params.compact` drops nil instead of coercing it to an empty string.
fn compact_string(params: &campfire_kit::ParamMap, key: &str) -> Option<String> {
    params.get(key).filter(|p| !p.is_null())?;
    string_attribute(params, key).flatten()
}
