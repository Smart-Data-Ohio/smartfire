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
    let user = concerns::require_current_user(c)?.clone();
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
    let user = presenters::user_summary(&secrets, &user);
    framed_page!(c, StatusCode::OK, |ctx| users::ProfileShow {
        ctx,
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

    // params.require(:user).permit(:name, :avatar, :email_address, :password, :bio).compact
    let params = c.params.require("user")?.permit(&permit_keys(&[
        "name",
        "avatar",
        "email_address",
        "password",
        "bio",
    ]));
    let password_changing = params.get("password").is_some_and(|p| p.is_present());
    let audit = crate::controllers::two_factor::audit_context(c)?;
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
    let pending = c
        .app()
        .db
        .write(move |tx| {
            user.update(tx, changes)?;
            if password_changing {
                campfire_db::TwoFactorRememberedDevice::revoke_all(tx, user.id)?;
                use campfire_db::models::audit_log::{Actor, AuditLog, NewAuditLog, Target};
                AuditLog::record(
                    tx,
                    NewAuditLog {
                        action: "user.password.change".into(),
                        actor: Some(Actor::from(&user)),
                        target: Some(Target::from(&user)),
                        ..Default::default()
                    },
                    &audit,
                )?;
            }
            attachments::assign(tx, Record::user(user.id), "avatar", avatar)
        })
        .await
        .map_err(Error::internal)?;
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
