//! `/api/v1/admin` (S7): workspace administration, the SPA's twin of the classic account pages.
//! Every write runs the classic controller's save path (`accounts#update`, `accounts/logos`,
//! `accounts/join_codes`, `accounts/users`, `accounts/users/two_factor_resets`,
//! `accounts/users/google_links`, `accounts/custom_styles` and `accounts/icons`) in the same
//! order, so the rows, audit entries and frames are the classic ones. The before-actions run in
//! the classic order too: administrators only, then the record, then the password confirmation.
//!
//! The password confirmation differs only in where it comes back to. Classic stashes the form
//! post and replays it after `/sudo`; a JSON write can't be replayed as a form, so the API
//! returns available confirmation methods and retry metadata as `SudoRequired`.
//! The client retains the body and retries once confirmation succeeds.

use axum::Router;
use axum::routing::{delete, get, patch, post, put};
use campfire_api_types as api;
use campfire_app::account_security;
use campfire_app::app::AppCtx;
use campfire_db::models::account::{validate_description, validate_vanity_slug_for};
use campfire_db::models::audit_log::{self, AuditLog, Context, NewAuditLog, Target};
use campfire_db::models::google_identity::GoogleIdentity;
use campfire_db::models::workspace_icon::{NewIcon, WorkspaceIcon};
use campfire_db::{Account, Role, User};
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode, action, unparsed_action};
use campfire_people::controllers::accounts::icons::{image_facts, save_image};
use campfire_presentation::time::Zone;
use campfire_runtime::concerns::{self, Authentication, Before};
use campfire_runtime::context::db_error;
use campfire_runtime::presenters::accounts::audit_logs;
use campfire_runtime::presenters::attachments::{self, Assignment, Record};
use campfire_runtime::presenters::pagination::Page;
use campfire_runtime::presenters::{self, accounts as account_presenters};
use campfire_storage::branding::{self, Kind, Prepared};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::dto::time;
use crate::error::{fail, prepare, respond, validation};

/// The largest admin body read: custom CSS.
const BODY_LIMIT: usize = 1024 * 1024;

/// People a page, as `accounts#edit` and `accounts/users#index`.
const PEOPLE_PER_PAGE: &[i64] = &[500];

/// The admin routes, merged into [`crate::routes`].
pub fn routes() -> Router<Kit> {
    Router::new()
        .route(
            "/api/v1/admin/workspace",
            get(action(workspace)).patch(unparsed_action(update_workspace)),
        )
        .route(
            "/api/v1/admin/workspace/logo",
            put(unparsed_action(update_logo)).delete(action(remove_logo)),
        )
        .route(
            "/api/v1/admin/workspace/banner",
            put(unparsed_action(update_banner)).delete(action(remove_banner)),
        )
        .route(
            "/api/v1/admin/workspace/join_code",
            post(action(reset_join_code)),
        )
        .route("/api/v1/admin/people", get(action(people)))
        .route(
            "/api/v1/admin/people/{id}",
            patch(unparsed_action(update_person)).delete(action(remove_person)),
        )
        .route(
            "/api/v1/admin/people/{id}/two_factor_reset",
            post(action(reset_two_factor)),
        )
        .route(
            "/api/v1/admin/people/{id}/google_link",
            post(action(allow_google_link)).delete(action(unlink_google)),
        )
        .route(
            "/api/v1/admin/custom_styles",
            get(action(custom_styles)).patch(unparsed_action(update_custom_styles)),
        )
        .route(
            "/api/v1/admin/icons",
            get(action(icons)).post(unparsed_action(create_icon)),
        )
        .route("/api/v1/admin/icons/{id}", delete(action(destroy_icon)))
        .route("/api/v1/admin/audit_log", get(action(audit_log)))
        .route(
            "/api/v1/admin/integrations_health",
            get(action(integrations_health)),
        )
}

macro_rules! endpoint {
    ($(#[$doc:meta])* $name:ident => $body:ident) => {
        $(#[$doc])*
        pub async fn $name(c: &mut Ctx) -> Result {
            prepare(c);
            let result = $body(c).await;
            respond(c, result)
        }
    };
}

endpoint!(
    /// `GET /api/v1/admin/workspace`
    workspace => show_workspace
);
endpoint!(
    /// `PATCH /api/v1/admin/workspace`
    update_workspace => save_workspace
);
endpoint!(
    /// `PUT /api/v1/admin/workspace/logo`
    update_logo => attach_logo
);
endpoint!(
    /// `DELETE /api/v1/admin/workspace/logo`
    remove_logo => detach_logo
);
endpoint!(
    /// `PUT /api/v1/admin/workspace/banner`
    update_banner => attach_banner
);
endpoint!(
    /// `DELETE /api/v1/admin/workspace/banner`
    remove_banner => detach_banner
);
endpoint!(
    /// `POST /api/v1/admin/workspace/join_code`
    reset_join_code => new_join_code
);
endpoint!(
    /// `GET /api/v1/admin/people`
    people => index_people
);
endpoint!(
    /// `PATCH /api/v1/admin/people/:id`
    update_person => change_role
);
endpoint!(
    /// `DELETE /api/v1/admin/people/:id`
    remove_person => deactivate_person
);
endpoint!(
    /// `POST /api/v1/admin/people/:id/two_factor_reset`
    reset_two_factor => create_two_factor_reset
);
endpoint!(
    /// `POST /api/v1/admin/people/:id/google_link`
    allow_google_link => create_google_link
);
endpoint!(
    /// `DELETE /api/v1/admin/people/:id/google_link`
    unlink_google => destroy_google_link
);
endpoint!(
    /// `GET /api/v1/admin/custom_styles`
    custom_styles => show_custom_styles
);
endpoint!(
    /// `PATCH /api/v1/admin/custom_styles`
    update_custom_styles => save_custom_styles
);
endpoint!(
    /// `GET /api/v1/admin/icons`
    icons => index_icons
);
endpoint!(
    /// `POST /api/v1/admin/icons`
    create_icon => save_icon
);
endpoint!(
    /// `DELETE /api/v1/admin/icons/:id`
    destroy_icon => delete_icon
);
endpoint!(
    /// `GET /api/v1/admin/audit_log`
    audit_log => show_audit_log
);
endpoint!(
    /// `GET /api/v1/admin/integrations_health`
    integrations_health => show_integrations_health
);

pub(crate) async fn before_actions(c: &mut Ctx) -> Result<()> {
    concerns::before_actions(
        c,
        Before {
            authentication: Authentication::JsonUnauthorized,
            ..Before::default()
        },
    )
    .await
}

/// The signed-in person, as the before-actions loaded them.
pub(crate) async fn viewer(c: &mut Ctx) -> Result<User> {
    before_actions(c).await?;
    Ok(concerns::require_current_user(c)?.clone())
}

/// `ensure_can_administer` after the before-actions: `Forbidden` for everyone else.
pub(crate) async fn administrator(c: &mut Ctx) -> Result<User> {
    let user = viewer(c).await?;
    concerns::ensure_can_administer(c)?;
    Ok(user)
}

pub(crate) async fn require_sudo(c: &mut Ctx) -> Result<()> {
    concerns::sudo::require_sudo_mode(c).await
}

/// The JSON body as `T`; anything else is a 422.
pub(crate) async fn body<T: DeserializeOwned>(c: &mut Ctx) -> Result<T> {
    let bytes = c.read_body(BODY_LIMIT).await;
    serde_json::from_slice(&bytes).map_err(|error| {
        fail(
            c,
            api::ApiError::Validation {
                message: format!("The request body isn't valid: {error}"),
                fields: Default::default(),
            },
        )
    })
}

/// `two_factor#audit_context`: who did it, from where.
fn audit_context(c: &Ctx) -> Result<Context> {
    campfire_people::controllers::two_factor::audit_context(c)
}

/// The `:id` path parameter as a record id; anything else is not found.
fn record_id(c: &Ctx) -> Result<i64> {
    c.param_str("id")
        .and_then(concerns::cast_integer)
        .ok_or(Error::NotFound)
}

/// `Current.account`.
async fn account(c: &Ctx) -> Result<Account> {
    c.app()
        .db
        .read(Account::first)
        .await
        .map_err(Error::internal)?
        .ok_or_else(|| Error::internal(std::io::Error::other("no account")))
}

// --- Workspace ---------------------------------------------------------------------------------

async fn show_workspace(c: &mut Ctx) -> Result {
    viewer(c).await?;
    reply_workspace(c).await
}

/// The answer to a read and to every workspace write.
async fn reply_workspace(c: &mut Ctx) -> Result {
    let account = account(c).await?;
    let branding = presenters::workspace_branding::for_account(c.app(), &account).await?;
    let logo_attached = branding.logo_url.is_some();
    let can_administer =
        concerns::current_user(c).is_some_and(|user| user.can_administer(None, false));
    let workspace = api::Workspace {
        name: account.name.clone(),
        logo_url: branding.logo_url.unwrap_or_else(|| account_presenters::fresh_account_logo_path(Some(&account), None)),
        logo_still_url: branding.logo_still_url,
        banner_url: branding.banner_url,
        banner_still_url: branding.banner_still_url,
        logo_attached,
        join_url: c.url_for(&campfire_routes::join(&account.join_code)),
        can_administer,
        restrict_room_creation_to_administrators: account
            .settings()
            .restrict_room_creation_to_administrators(),
        upload_limit_bytes: account.settings().upload_limit_bytes(),
        version: c.app().config.app_version.clone(),
        description: account.settings().description().to_string(),
        vanity_slug: account.settings().vanity_slug().map(str::to_string),
        vanity_url: account.settings().vanity_slug().map(|slug| c.url_for(&campfire_routes::join(slug))),
    };
    c.json(StatusCode::OK, &workspace)
}

async fn save_workspace(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let update: api::UpdateWorkspace = body(c).await?;
    let current = account(c).await?;
    if let Some(bytes) = update.upload_limit_bytes
        && !(1..=campfire_db::models::account::MAX_UPLOAD_LIMIT_BYTES).contains(&bytes)
    {
        return Err(fail(
            c,
            validation("uploadLimitBytes", "must be a positive safe integer"),
        ));
    }
    let description = update
        .description
        .as_deref()
        .map(validate_description)
        .transpose()
        .map_err(|error| fail(c, validation("description", &error.to_string())))?
        .map(str::to_string);
    let vanity_slug = update
        .vanity_slug
        .as_deref()
        .map(|slug| validate_vanity_slug_for(slug, &current.join_code))
        .transpose()
        .map_err(|error| fail(c, validation("vanitySlug", &error.to_string())))?
        .map(str::to_string);
    let mut settings = update
        .restrict_room_creation_to_administrators
        .map(|restrict| {
            vec![(
                "restrict_room_creation_to_administrators".to_string(),
                restrict.to_string(),
            )]
        });
    if let Some(bytes) = update.upload_limit_bytes {
        settings
            .get_or_insert_with(Vec::new)
            .push(("upload_limit_bytes".into(), bytes.to_string()));
    }
    for (key, value) in [("description", description), ("vanity_slug", vanity_slug)] {
        if let Some(value) = value {
            settings
                .get_or_insert_with(Vec::new)
                .push((key.into(), value));
        }
    }
    write_workspace(
        c,
        update.name,
        settings,
        Assignment::Unchanged,
        Assignment::Unchanged,
        None,
    )
    .await?;
    reply_workspace(c).await
}

async fn attach_logo(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let update: api::UpdateLogo = body(c).await?;
    let prepared = branding_image(c, &update.signed_id, Kind::Logo).await?;
    let logo = Assignment::Existing(prepared.blob.clone());
    write_workspace(c, None, None, logo, Assignment::Unchanged, Some(prepared)).await?;
    reply_workspace(c).await
}

async fn attach_banner(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let update: api::UpdateBanner = body(c).await?;
    let prepared = branding_image(c, &update.signed_id, Kind::Banner).await?;
    let banner = Assignment::Existing(prepared.blob.clone());
    write_workspace(c, None, None, Assignment::Unchanged, banner, Some(prepared)).await?;
    reply_workspace(c).await
}

async fn branding_image(c: &mut Ctx, signed_id: &str, kind: Kind) -> Result<Prepared> {
    let id = campfire_storage::paths::verify_signed_blob_id(
        &*c.app().storage.verifier,
        signed_id,
        c.app().clock.now(),
    )
    .ok_or_else(|| fail(c, validation("signedId", "isn't an uploaded file")))?;
    let blob = c
        .app()
        .db
        .read(move |conn| {
            campfire_storage::Blob::find(conn, id).map_err(attachments::storage_error)
        })
        .await
        .map_err(Error::internal)?
        .ok_or_else(|| fail(c, validation("signedId", "isn't an uploaded file")))?;
    if !matches!(
        blob.content_type(),
        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
    ) {
        return Err(fail(
            c,
            validation("signedId", "must be a PNG, JPEG, GIF or WebP image"),
        ));
    }
    if blob.byte_size > branding::MAX_BYTES as i64 {
        return Err(fail(c, validation("signedId", "must be 10 MB or smaller")));
    }
    let storage = c.app().storage.clone();
    let prepared = campfire_runtime::active_storage::process_branding_with_deadline(
        branding::processing_timeout(&blob),
        move |cancel| Ok(branding::prepare(&storage, blob, kind, &cancel)),
    )
    .await
    .map_err(|_| fail(c, validation("signedId", "couldn't be read as an image")))?;
    prepared.map_err(|invalid| fail(c, validation("signedId", invalid.message(kind))))
}

/// `accounts#update`: the account, its logo, then the audit of what changed.
async fn write_workspace(
    c: &mut Ctx,
    name: Option<String>,
    settings: Option<Vec<(String, String)>>,
    logo: Assignment,
    banner: Assignment,
    prepared: Option<Prepared>,
) -> Result<()> {
    let mut account = account(c).await?;
    let logo = logo.stage(c.app()).await?;
    let banner = banner.stage(c.app()).await?;
    let audit = audit_context(c)?;
    let storage = c.app().storage.clone();
    let secrets = c.app().secrets.clone();
    let (before, account, before_logo, after_logo, banner_change) = c
        .app()
        .db
        .write(move |tx| {
            if let Some(prepared) = prepared {
                save_branding(
                    tx,
                    &storage,
                    rails_compat::blob_branding::Marker::new(&secrets),
                    prepared,
                )?;
            }
            let before = account.clone();
            let before_logo =
                attachments::attached_blob(tx.conn(), "Account", account.id, "logo")?.is_some();
            let before_banner = attachments::attached_blob(tx.conn(), "Account", account.id, "banner")?.map(|blob| blob.id);
            let settings: Option<Vec<(&str, &str)>> = settings.as_ref().map(|settings| {
                settings
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str()))
                    .collect()
            });
            account.update(tx, name.as_deref(), None, settings.as_deref())?;
            attachments::assign(tx, Record::account(account.id, &secrets), "logo", logo)?;
            attachments::assign(tx, Record::account(account.id, &secrets), "banner", banner)?;
            let after_logo =
                attachments::attached_blob(tx.conn(), "Account", account.id, "logo")?.is_some();
            let after_banner = attachments::attached_blob(tx.conn(), "Account", account.id, "banner")?.map(|blob| blob.id);
            Ok((before, account, before_logo, after_logo, Some((before_banner, after_banner))))
        })
        .await
        .map_err(Error::internal)?;
    c.app()
        .db
        .write(move |tx| {
            account_security::settings_changed(
                tx,
                &before,
                &account,
                before_logo,
                after_logo,
                banner_change,
                &audit,
            )
        })
        .await
        .map_err(Error::internal)?;
    presenters::workspace_branding::publish(c.app()).await;
    Ok(())
}

/// Save the probe and its still atomically with the attachment, retaining analyzer metadata.
fn save_branding(
    tx: &mut campfire_db::Tx<'_>,
    storage: &campfire_storage::Storage,
    marker: rails_compat::blob_branding::Marker,
    prepared: Prepared,
) -> campfire_db::Result<()> {
    let mut blob = campfire_storage::Blob::find(tx.conn(), prepared.blob.id)
        .map_err(attachments::storage_error)?
        .ok_or(campfire_db::Error::RecordNotFound("ActiveStorage::Blob"))?;
    let mut animated = false;
    if let Some((variation, image)) = prepared.still {
        if storage
            .record_variant(tx.conn(), &blob, &variation, &image, tx.now().jiff())
            .map_err(attachments::storage_error)?
            .is_some()
        {
            campfire_runtime::active_storage::keep_after_commit(tx, image);
            animated = true;
        } else {
            // A competing upload may have recorded it, or a previous still may be missing.
            animated = storage
                .existing_variant_file(tx.conn(), &blob, &variation)
                .ok()
                .flatten()
                .is_some();
        }
    }
    blob.metadata.merge(&prepared.blob.metadata);
    blob.metadata.set(
        branding::ANIMATED_KEY,
        campfire_storage::Json::Bool(animated),
    );
    blob.metadata.set(
        branding::MARK_KEY,
        campfire_storage::Json::String(marker.sign(&blob.key)),
    );
    tx.conn().execute(
        "UPDATE active_storage_blobs SET content_type = ?1, metadata = ?2 WHERE id = ?3",
        rusqlite::params![prepared.blob.content_type, blob.metadata.encode(), blob.id],
    )?;
    campfire_runtime::active_storage::mark_branding_tree(tx.conn(), marker, blob.id)?;
    Ok(())
}

/// `accounts/logos#destroy`: the logo goes, then the audit.
async fn detach_logo(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let account = account(c).await?;
    let secrets = c.app().secrets.clone();
    let audit = audit_context(c)?;
    let account = c
        .app()
        .db
        .write(move |tx| {
            attachments::destroy(tx, Record::account(account.id, &secrets), "logo")?;
            Ok(account)
        })
        .await
        .map_err(Error::internal)?;
    c.app()
        .db
        .write(move |tx| account_security::logo_removed(tx, &account, &audit))
        .await
        .map_err(Error::internal)?;
    presenters::workspace_branding::publish(c.app()).await;
    reply_workspace(c).await
}

async fn detach_banner(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let account = account(c).await?;
    let secrets = c.app().secrets.clone();
    let audit = audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            attachments::destroy(tx, Record::account(account.id, &secrets), "banner")?;
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    c.app()
        .db
        .write(move |tx| account_security::banner_removed(tx, &account, &audit))
        .await
        .map_err(Error::internal)?;
    presenters::workspace_branding::publish(c.app()).await;
    reply_workspace(c).await
}

/// `accounts/join_codes#create`
async fn new_join_code(c: &mut Ctx) -> Result {
    administrator(c).await?;
    require_sudo(c).await?;
    let mut account = account(c).await?;
    let audit = audit_context(c)?;
    let account = c
        .app()
        .db
        .write(move |tx| {
            account.reset_join_code(tx)?;
            Ok(account)
        })
        .await
        .map_err(Error::internal)?;
    c.app()
        .db
        .write(move |tx| account_security::join_code_reset(tx, &account, &audit))
        .await
        .map_err(Error::internal)?;
    reply_workspace(c).await
}

// --- People ------------------------------------------------------------------------------------

/// One row of the people list, the administrator-only fields blanked for everyone else.
fn person(summary: campfire_presentation::users::UserSummary, viewer: i64, admin: bool) -> api::Person {
    let offer = summary.offer_google_email_link();
    api::Person {
        id: summary.id,
        name: summary.name,
        avatar_url: summary.avatar_path,
        role: if summary.role == campfire_presentation::users::Role::Administrator {
            api::PersonRole::Administrator
        } else {
            api::PersonRole::Member
        },
        banned: summary.status == campfire_presentation::users::Status::Banned,
        you: summary.id == viewer,
        two_factor_enabled: admin && summary.two_factor_enabled,
        email_address: summary.email_address.filter(|_| admin),
        google_identity_email: summary.google_identity_email.filter(|_| admin),
        offer_google_email_link: admin && offer,
    }
}

/// The row `user` makes on the account page.
async fn load_person(c: &Ctx, user: User) -> Result<api::Person> {
    let secrets = c.app().secrets.clone();
    let summary = c
        .app()
        .db
        .read(move |conn| presenters::account_user_summary(conn, &secrets, &user))
        .await
        .map_err(Error::internal)?;
    let viewer = concerns::require_current_user(c)?.id;
    let admin = concerns::current_user(c).is_some_and(|user| user.can_administer(None, false));
    Ok(person(summary, viewer, admin))
}

/// `accounts#edit`'s list: everyone (administrators also see banned people), administrators
/// first, then 500 a page.
async fn index_people(c: &mut Ctx) -> Result {
    let viewer = viewer(c).await?;
    let admin = viewer.can_administer(None, false);
    let secrets = c.app().secrets.clone();
    let page_param = c.param_str("page").map(str::to_string);
    let (people, next_page) = c
        .app()
        .db
        .read(move |conn| {
            let mut users = account_presenters::account_users(conn, admin)?;
            // Stable, so each group keeps the list's name order.
            users.sort_by_key(|user| user.role != Role::Administrator);
            let page = Page::new(page_param.as_deref(), users.len() as i64, PEOPLE_PER_PAGE);
            let people = page
                .records(&users)
                .iter()
                .map(|user| {
                    presenters::account_user_summary(conn, &secrets, user)
                        .map(|summary| person(summary, viewer.id, admin))
                })
                .collect::<campfire_db::Result<Vec<_>>>()?;
            Ok((
                people,
                (!page.is_last()).then(|| page.next_param().to_string()),
            ))
        })
        .await
        .map_err(Error::internal)?;
    c.json(StatusCode::OK, &api::PeoplePage { people, next_page })
}

/// `User.active.find(params[:id])`
async fn active_person(c: &Ctx) -> Result<User> {
    let id = record_id(c)?;
    c.app()
        .db
        .read(move |conn| match User::find_active(conn, id) {
            Ok(user) => Ok(Some(user)),
            Err(campfire_db::Error::RecordNotFound(_)) => Ok(None),
            Err(error) => Err(error),
        })
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)
}

/// `accounts/users#update`: an invalid save changes nothing, as the classic redirect shows.
async fn change_role(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let mut user = active_person(c).await?;
    require_sudo(c).await?;
    let update: api::UpdatePerson = body(c).await?;
    let role = match update.role {
        api::PersonRole::Administrator => Role::Administrator,
        api::PersonRole::Member => Role::Member,
    };
    let audit = audit_context(c)?;
    let id = user.id;
    let saved = c
        .app()
        .db
        .write(move |tx| {
            campfire_db::models::user::profile_settings::update(tx, user.id, Default::default())?;
            campfire_runtime::authentication::update_role(tx, &mut user, role, &audit)
        })
        .await;
    match saved {
        Ok(()) | Err(campfire_db::Error::RecordInvalid(_)) => (),
        Err(error) => return Err(Error::internal(error)),
    }
    let user = c
        .app()
        .db
        .read(move |conn| User::find(conn, id))
        .await
        .map_err(db_error)?;
    let person = load_person(c, user).await?;
    c.json(
        StatusCode::OK,
        &api::PersonChange {
            person,
            notice: None,
        },
    )
}

/// `accounts/users#destroy`: the calendar watch stops, then the person is deactivated.
async fn deactivate_person(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let mut user = active_person(c).await?;
    require_sudo(c).await?;
    let audit = audit_context(c)?;
    let id = user.id;
    campfire_app::integrations::google::calendar::stop_remote(c.app(), user.id)
        .await
        .map_err(Error::internal)?;
    c.app()
        .db
        .write(move |tx| campfire_runtime::authentication::deactivate_user(tx, &mut user, &audit))
        .await
        .map_err(Error::internal)?;
    c.json(StatusCode::OK, &api::PersonRemoved { id })
}

/// `accounts/users/two_factor_resets#create`: never yourself, a bot, or someone without it.
async fn create_two_factor_reset(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let id = concerns::ruby_to_i(c.param_str("id").unwrap_or(""));
    let user = c
        .app()
        .db
        .read(move |conn| Ok(User::find_by_id(conn, id)?.filter(|u| u.is_active() && !u.is_bot())))
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)?;
    if user.id == concerns::require_current_user(c)?.id {
        return Err(refusal(
            c,
            "Reset someone else's two-step sign-in from here. To change your own, use Disable on your profile.",
        ));
    }
    let user_id = user.id;
    let enabled = c
        .app()
        .db
        .read(move |conn| User::find(conn, user_id)?.two_factor_enabled(conn))
        .await
        .map_err(Error::internal)?;
    if !enabled {
        let message = format!("{} doesn't have two-step sign-in enabled.", user.name);
        return Err(refusal(c, &message));
    }
    let name = user.name.clone();
    let audit = audit_context(c)?;
    let reset = user.clone();
    c.app()
        .db
        .write(move |tx| campfire_runtime::authentication::reset_two_factor(tx, &reset, &audit))
        .await
        .map_err(Error::internal)?;
    let person = load_person(c, user).await?;
    c.json(
        StatusCode::OK,
        &api::PersonChange {
            person,
            notice: Some(format!(
                "Two-step sign-in reset for {name}. They will set it up again at next sign-in."
            )),
        },
    )
}

/// The classic page's alert, as a 422 naming no field.
pub(crate) fn refusal(c: &mut Ctx, message: &str) -> Error {
    fail(
        c,
        api::ApiError::Validation {
            message: message.into(),
            fields: Default::default(),
        },
    )
}

async fn create_google_link(c: &mut Ctx) -> Result {
    google_link(c, true).await
}

async fn destroy_google_link(c: &mut Ctx) -> Result {
    google_link(c, false).await
}

/// `accounts/users/google_links#create` and `#destroy` (`google_sign_in::admin`).
async fn google_link(c: &mut Ctx, allow: bool) -> Result {
    administrator(c).await?;
    let id = record_id(c)?;
    let context = audit_context(c)?;
    let user = c
        .app()
        .db
        .write(move |tx| GoogleIdentity::admin_set_link(tx, id, allow, &context))
        .await
        .map_err(db_error)?;
    let notice = if allow {
        format!(
            "{} can now link Google sign-in for {}.",
            user.name,
            user.email_address.clone().unwrap_or_default()
        )
    } else {
        format!("Google sign-in unlinked from {}.", user.name)
    };
    let person = load_person(c, user).await?;
    c.json(
        StatusCode::OK,
        &api::PersonChange {
            person,
            notice: Some(notice),
        },
    )
}

// --- Custom styles -----------------------------------------------------------------------------

async fn show_custom_styles(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let account = account(c).await?;
    c.json(
        StatusCode::OK,
        &api::CustomStyles {
            css: account.custom_styles,
        },
    )
}

/// `accounts/custom_styles#update`
async fn save_custom_styles(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let mut account = account(c).await?;
    require_sudo(c).await?;
    let update: StylesUpdate = body(c).await?;
    let audit = audit_context(c)?;
    // An absent `css` changes nothing, as an absent `custom_styles` param doesn't classically.
    let css = update.css;
    let (before, account) = c
        .app()
        .db
        .write(move |tx| {
            let before = account.clone();
            account.update(tx, None, css.as_ref().map(Option::as_deref), None)?;
            Ok((before, account))
        })
        .await
        .map_err(Error::internal)?;
    let css = account.custom_styles.clone();
    c.app()
        .db
        .write(move |tx| account_security::styles_changed(tx, &before, &account, &audit))
        .await
        .map_err(Error::internal)?;
    campfire_app::cable::sync::workspace_styles_updated(&c.app().cable, css.clone());
    c.json(StatusCode::OK, &api::CustomStyles { css })
}

/// The body of `PATCH /api/v1/admin/custom_styles`: `css` absent, `null` (clears) or the text.
#[derive(serde::Deserialize)]
struct StylesUpdate {
    #[serde(default, deserialize_with = "given")]
    css: Option<Option<String>>,
}

/// A key that is present, `null` included: `Some(None)` for `null`, `None` when absent.
fn given<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

// --- Icons -------------------------------------------------------------------------------------

async fn index_icons(c: &mut Ctx) -> Result {
    administrator(c).await?;
    reply_icons(c).await
}

async fn reply_icons(c: &mut Ctx) -> Result {
    let list = c
        .app()
        .db
        .read(|conn| {
            let icons = WorkspaceIcon::ordered(conn)?
                .into_iter()
                .map(|icon| {
                    let animated = WorkspaceIcon::animated_by_name(conn, &icon.name)?;
                    let image_url = campfire_routes::workspace_icon(&icon.name);
                    Ok(api::WorkspaceIcon {
                        still_url: if animated {
                            format!("{image_url}?still=1")
                        } else {
                            image_url.clone()
                        },
                        animated,
                        image_url,
                        id: icon.id,
                        name: icon.name,
                        title: icon.title,
                        creator_name: icon.creator_name,
                    })
                })
                .collect::<campfire_db::Result<Vec<_>>>()?;
            let animated_limit = Account::first(conn)?
                .map(|account| account.settings().animated_emoji_limit())
                .unwrap_or(campfire_db::models::account::DEFAULT_ANIMATED_EMOJI_LIMIT);
            Ok(api::WorkspaceIconList {
                icons,
                animated_limit,
                animated_usage: WorkspaceIcon::animated_usage(conn)?,
            })
        })
        .await
        .map_err(Error::internal)?;
    c.json(StatusCode::OK, &list)
}

/// `accounts/icons#create`: the icon and its image, then the audit.
async fn save_icon(c: &mut Ctx) -> Result {
    let creator = administrator(c).await?;
    let create: api::CreateIcon = body(c).await?;
    let icon = NewIcon {
        name: Some(create.name),
        title: Some(create.title),
        creator_id: creator.id,
    }
    .normalized();
    let image = match create.signed_id {
        Some(signed_id) => {
            let verified = campfire_storage::paths::verify_signed_blob_id(
                &*c.app().storage.verifier,
                &signed_id,
                c.app().clock.now(),
            );
            if verified.is_none() {
                return Err(fail(c, validation("image", "isn't an uploaded file")));
            }
            Assignment::Signed(signed_id)
        }
        None => Assignment::Unchanged,
    };
    let assignment = image.stage(c.app()).await?;
    let (facts, prepared) = image_facts(c, &assignment).await?;
    let storage = c.app().storage.clone();
    let audit = audit_context(c)?;
    let brand = campfire_runtime::rich_text::builtin_icon(icon.name.as_deref().unwrap_or(""));
    let saved = c
        .app()
        .db
        .write(move |tx| {
            let icon = icon.save(tx, brand, facts.as_ref())?;
            attachments::assign(tx, Record::workspace_icon(icon.id), "image", assignment)?;
            save_image(tx, &storage, icon.id, prepared)?;
            Ok(icon)
        })
        .await
        .map_err(Error::internal)?;
    c.app()
        .db
        .write(move |tx| {
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "workspace_icon.create".into(),
                    target: Some(Target {
                        record_type: "WorkspaceIcon".into(),
                        id: saved.id,
                        label: Some(format!(":{}:", saved.name)),
                    }),
                    changes: Some(json!({"name": saved.name, "title": saved.title})),
                    ..Default::default()
                },
                &audit,
            )?;
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    reply_icons(c).await
}

/// `accounts/icons#destroy`
async fn delete_icon(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let id = c
        .param_str("id")
        .and_then(|id| id.parse::<i64>().ok())
        .ok_or(Error::NotFound)?;
    let audit = audit_context(c)?;
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
                    changes: Some(json!({"name": icon.name})),
                    ..Default::default()
                },
                &audit,
            )?;
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    reply_icons(c).await
}

// --- Audit log ---------------------------------------------------------------------------------

/// `accounts/audit_logs#show` (HTML): the same filters, selection and page.
async fn show_audit_log(c: &mut Ctx) -> Result {
    let viewer = administrator(c).await?;
    let time_zone = c
        .app()
        .db
        .read(move |conn| {
            campfire_db::models::user::profile_settings::appearance(conn, viewer.id)
                .map(|settings| settings.time_zone)
        })
        .await
        .map_err(Error::internal)?;
    let zone = Zone::for_user(time_zone.as_deref());
    // The classic form names the action filter `audit_action`; the API calls it `action`.
    let filters = audit_logs::filters(|key| {
        let key = match key {
            "audit_action" => "action",
            "target_type" => "targetType",
            other => other,
        };
        c.param_str(key).map(str::to_owned)
    });
    let selection = audit_logs::selection(&filters, &zone)?;
    let page_param = c.param_str("page").map(str::to_owned);
    let limit = audit_log::browsing::CSV_EXPORT_LIMIT;
    let (count, page, rows) = c
        .app()
        .db
        .read(move |conn| {
            let count = selection.count(conn)?;
            let page = Page::new(
                page_param.as_deref(),
                count,
                &[audit_log::browsing::PAGE_SIZE],
            );
            let rows = selection.entries(conn, page.limit(), page.offset())?;
            Ok((count, page, rows))
        })
        .await
        .map_err(Error::internal)?;
    let entries = rows
        .into_iter()
        .map(|row| api::AuditLogEntry {
            id: row.id,
            created_at: time(row.created_at),
            changes: campfire_presentation::accounts::audit_logs::changes_summary(&row.details),
            action: row.action,
            actor: present(row.actor_label),
            target: present(row.target_label),
            target_type: row.target_type,
            ip_address: present(row.ip_address),
        })
        .collect();
    let page = api::AuditLogPage {
        export_url: filters.export_path(),
        filters: api::AuditLogFilters {
            actor: filters.actor,
            action: filters.action,
            target_type: filters.target_type,
            from: filters.from,
            to: filters.to,
        },
        entries,
        next_page: (!page.is_last()).then(|| page.next_param().to_string()),
        actions: audit_log::actions(),
        target_types: audit_log::browsing::TARGET_TYPES
            .iter()
            .map(|kind| (*kind).to_string())
            .collect(),
        export_truncated: count > limit,
        export_limit: limit,
        time_zone: zone.tz().iana_name().unwrap_or("UTC").to_string(),
    };
    c.json(StatusCode::OK, &page)
}

/// A label, `None` when blank (the classic page prints a dash).
fn present(label: Option<String>) -> Option<String> {
    label.filter(|label| !campfire_richtext::ruby::is_blank(label))
}

// --- Integrations health -----------------------------------------------------------------------

async fn show_integrations_health(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let now = campfire_db::Timestamp::from_jiff(c.now());
    let snapshot = c
        .app()
        .db
        .read(move |conn| {
            campfire_app::integrations::health::snapshot(conn, now, |key| std::env::var(key).ok())
        })
        .await
        .map_err(Error::internal)?;
    c.json(StatusCode::OK, &health(&snapshot))
}

fn text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn count(value: &Value) -> i64 {
    value.as_i64().unwrap_or(0)
}

fn rows(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}

/// Each `[subject parts.., detail]` row, its subject worded by `subject` as the classic page does.
fn issues(value: &Value, subject: impl Fn(&[Value]) -> String) -> Vec<api::HealthIssue> {
    rows(value)
        .iter()
        .filter_map(Value::as_array)
        .filter_map(|row| {
            let (detail, parts) = row.split_last()?;
            Some(api::HealthIssue {
                subject: subject(parts),
                detail: text(detail),
            })
        })
        .collect()
}

/// The classic health page's snapshot, typed.
fn health(snapshot: &Value) -> api::IntegrationsHealth {
    let github = &snapshot["github"];
    let google = &snapshot["google"];
    let delivery = &snapshot["agent_delivery"];
    let login = |parts: &[Value]| format!("@{}", parts.first().map(text).unwrap_or_default());
    api::IntegrationsHealth {
        github: api::GithubHealth {
            workspace_token: github["workspace_token"] == true,
            app_configured: github["app_configured"] == true,
            webhook_secret: github["webhook_secret"] == true,
            connected: count(&github["connected"]),
            app_tokens: count(&github["app_tokens"]),
            deliveries_24h: count(&github["deliveries_24h"]),
            disconnected: issues(&github["disconnected"], login),
            last_errors: issues(&github["last_errors"], login),
            fetch_errors: issues(&github["fetch_errors"], |parts| match parts {
                [owner, repo, number] => format!("{}/{}#{}", text(owner), text(repo), text(number)),
                _ => String::new(),
            }),
        },
        google: api::GoogleHealth {
            configured: google["configured"] == true,
            connected: count(&google["connected"]),
            push_enabled: google["push"]["enabled"] == true,
            push_channels: count(&google["push"]["count"]),
            disconnected: issues(&google["disconnected"], |parts| {
                parts.first().map(text).unwrap_or_default()
            }),
            entry_errors: issues(&google["entry_errors"], |parts| match parts {
                [event, user] => format!("event {} / user {}", text(event), text(user)),
                _ => String::new(),
            }),
            expiring: rows(&google["push"]["expiring"])
                .iter()
                .map(|row| api::PushChannelExpiry {
                    user_id: count(&row[0]),
                    expires_at: row[1].as_str().map(str::to_string),
                    error: row[2]
                        .as_str()
                        .filter(|error| !error.chars().all(char::is_whitespace))
                        .map(str::to_string),
                })
                .collect(),
        },
        fizzy: api::FizzyHealth {
            configured: snapshot["fizzy"]["configured"] == true,
            note: snapshot["fizzy"]["note"].as_str().map(str::to_string),
        },
        agent_delivery: api::DeliveryHealth {
            pending: count(&delivery["pending"]),
            failed_24h: count(&delivery["failed"]),
            recent_errors: issues(&delivery["recent_errors"], |parts| match parts {
                [agent, event] => format!("agent {} / {}", text(agent), text(event)),
                _ => String::new(),
            }),
        },
        email: api::EmailHealth {
            enabled: snapshot["email"]["enabled"] == true,
            rooms_with_addresses: count(&snapshot["email"]["rooms_with_addresses"]),
        },
    }
}
