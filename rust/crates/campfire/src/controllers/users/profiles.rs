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
    c.respond_to(&[&format::HTML])?;
    let id = concerns::require_current_user(c)?.id;
    let user = c
        .app()
        .db
        .read(move |conn| campfire_db::UserStatusSettings::find(conn, id))
        .await
        .map_err(Error::internal)?;
    render_settings(c, StatusCode::OK, user, campfire_db::Errors::default()).await
}

/// Failed settings writes keep submitted values in the forms and fetch the rolled-back keyword list.
pub(super) async fn render_settings(
    c: &mut Ctx,
    status: StatusCode,
    settings: campfire_db::UserStatusSettings,
    errors: campfire_db::Errors,
) -> Result {
    render_profile(c, status, settings, errors, true).await
}

async fn render_profile(
    c: &mut Ctx,
    status: StatusCode,
    settings: campfire_db::UserStatusSettings,
    errors: campfire_db::Errors,
    settings_error: bool,
) -> Result {
    c.respond_to(&[&format::HTML])?;
    let user = settings.user.clone();
    if settings_error && !errors.is_empty() && user.role != campfire_db::Role::Bot {
        // At the pin, both settings controllers render profiles/show without setting
        // @two_factor_devices. The enabled-credential branch calls nil.any? and returns 500.
        // Preserve this observed failure until the reference/WS9 template contract changes.
        let id = user.id;
        let enabled=c.app().db.read(move |conn|Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM two_factor_credentials WHERE user_id=? AND confirmed_at IS NOT NULL)",[id],|r|r.get::<_,bool>(0))?)).await.map_err(Error::internal)?;
        if enabled {
            return Err(Error::internal(anyhow::anyhow!(
                "profiles/two_factor: undefined method any? for nil remembered devices"
            )));
        }
    }
    let secrets = c.app().secrets.clone();
    let transfer_id = presenters::accounts::transfer_id(&secrets, user.id, c.now());
    let now = c.app().db.env().now();
    let google_configured = presenters::status_settings::google_configured();
    let (avatar_attached, (direct_memberships, shared_memberships), settings) = {
        let user = user.clone();
        c.app()
            .db
            .read(move |conn| {
                let attached =
                    attachments::attached_blob(conn, "User", user.id, "avatar")?.is_some();
                Ok((
                    attached,
                    presenters::accounts::profile_memberships(conn, &user)?,
                    presenters::status_settings::forms(
                        conn,
                        &settings,
                        errors,
                        now,
                        google_configured,
                    )?,
                ))
            })
            .await
            .map_err(Error::internal)?
    };
    let user = presenters::user_summary(&secrets, &user);
    framed_page!(c, status, |ctx| users::ProfileShow {
        ctx,
        user: user.clone(),
        avatar_attached,
        transfer_id: transfer_id.clone(),
        shared_memberships: shared_memberships.clone(),
        direct_memberships: direct_memberships.clone(),
        settings: settings.clone(),
    })
    .await
}

/// `@user.update user_params`, then `redirect_to user_profile_url, notice: update_notice`.
pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let mut user = concerns::require_current_user(c)?.clone();

    // params.require(:user).permit(:name, :avatar, :email_address, :password, :bio).compact
    let params = c.params.require("user")?.permit(&permit_keys(&[
        "name",
        "avatar",
        "email_address",
        "password",
        "bio",
        "theme",
        "text_size",
        "time_zone",
    ]));
    let present = |key: &str| string_attribute(&params, key).flatten();
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
        ..UserChanges::default()
    };
    let id = user.id;
    let mut settings = c
        .app()
        .db
        .read(move |conn| campfire_db::UserStatusSettings::find(conn, id))
        .await
        .map_err(Error::internal)?;
    if let Some(theme) = present("theme") {
        settings.theme = theme;
    }
    if let Some(size) = present("text_size") {
        settings.text_size = size;
    }
    if let Some(zone) = present("time_zone") {
        settings.time_zone = Some(zone);
    }
    if params.get("time_zone").is_some() {
        settings.time_zone_explicit = true;
    }
    let mut submitted = settings.clone();
    if let Some(name) = &changes.name {
        submitted.user.name = name.clone();
    }
    if let Some(email) = &changes.email_address {
        submitted.user.email_address = email.clone();
    }
    if let Some(bio) = &changes.bio {
        submitted.user.bio = bio.clone();
    }
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
            settings.save(tx)?;
            user.update(tx, changes)?;
            attachments::assign(tx, Record::user(user.id), "avatar", avatar)
        })
        .await;
    let pending = match result {
        Ok(pending) => pending,
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            return render_profile(
                c,
                StatusCode::UNPROCESSABLE_ENTITY,
                submitted,
                errors,
                false,
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

#[cfg(test)]
#[path = "profiles/ws17_tests.rs"]
mod ws17_tests;
