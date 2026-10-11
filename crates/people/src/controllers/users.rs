//! `UsersController` (reference/app/controllers/users_controller.rb): joining with the account's
//! join code, and a user's page.

pub mod avatars;
pub mod stars;
pub mod bans;
pub mod dnd_allowances;
pub mod notification_settings;
pub mod statuses;
pub mod profiles;
pub mod presences;
pub mod push_subscriptions;
pub mod time_zones;
pub mod tours;

pub mod sessions;

use campfire_api_types::{InviteRefusal, JoinPage, SignInHelpContact, SignInWorkspace, WorkspaceJoin};
use campfire_db::{Account, NewUser, User};
use campfire_db::models::workspace_invite::{InviteState, WorkspaceInvite};
use campfire_kit::{Ctx, Error, Param, ParamMap, Result, StatusCode, format, halt, permit_keys};

use super::auth::{self, ResponseMode};
use super::presenters::attachments::{self, Assignment, Record};
use super::presenters::{self};
use crate::app::AppCtx;
use crate::controllers::presenters::page::{retained_page};
use crate::concerns::{self, Before, cast_integer};

// Joining with the join code (`/join/:join_code`) or a workspace invite (`/invite/:token`). The
// retained form and the SPA's JSON endpoints (`/api/v1/join/:join_code`, `/api/v1/invite/:token`)
// run the same callbacks, checks and user creation; only the answer's shape differs.

/// `require_unauthenticated_access only: %i[ new create ]`, `before_action :verify_join_code`
pub async fn new(c: &mut Ctx) -> Result {
    join_new(c, ResponseMode::Html).await
}

pub async fn new_json(c: &mut Ctx) -> Result {
    let result = join_new(c, ResponseMode::Json).await;
    auth::complete(c, result)
}

pub async fn create(c: &mut Ctx) -> Result {
    join_create(c, ResponseMode::Html).await
}

pub async fn create_json(c: &mut Ctx) -> Result {
    let result = join_create(c, ResponseMode::Json).await;
    auth::complete(c, result)
}

pub async fn invite_new(c: &mut Ctx) -> Result {
    invite_new_response(c, ResponseMode::Html).await
}

pub async fn invite_new_json(c: &mut Ctx) -> Result {
    let result = invite_new_response(c, ResponseMode::Json).await;
    auth::complete(c, result)
}

pub async fn invite_create(c: &mut Ctx) -> Result {
    invite_create_response(c, ResponseMode::Html).await
}

pub async fn invite_create_json(c: &mut Ctx) -> Result {
    let result = invite_create_response(c, ResponseMode::Json).await;
    auth::complete(c, result)
}

async fn join_new(c: &mut Ctx, mode: ResponseMode) -> Result {
    before_actions(c, "users#new", mode).await?;
    let account = verify_join_code(c).await?;
    let description = account.settings().description().to_string();
    join_page(c, StatusCode::OK, campfire_routes::join(&account.join_code), description, None, mode).await
}

async fn join_create(c: &mut Ctx, mode: ResponseMode) -> Result {
    before_actions(c, "users#create", mode).await?;
    verify_join_code(c).await?;
    create_user(c, None, mode).await
}

async fn invite_new_response(c: &mut Ctx, mode: ResponseMode) -> Result {
    before_actions(c, "users#new", mode).await?;
    verify_invite(c, mode).await?;
    let description = c
        .app()
        .db
        .read(Account::first)
        .await
        .map_err(Error::internal)?
        .map(|account| account.settings().description().to_string())
        .unwrap_or_default();
    join_page(c, StatusCode::OK, c.request.path().to_owned(), description, None, mode).await
}

async fn invite_create_response(c: &mut Ctx, mode: ResponseMode) -> Result {
    before_actions(c, "users#create", mode).await?;
    verify_invite(c, mode).await?;
    create_user(c, Some(c.param_str("token").unwrap_or_default().to_owned()), mode).await
}

/// The retained callbacks, with the JSON body's authenticity token exposed to the forgery check.
async fn before_actions(c: &mut Ctx, endpoint: &'static str, mode: ResponseMode) -> Result<()> {
    auth::before_actions(c, endpoint, Before::default().require_unauthenticated_access(), mode).await
}

async fn join_page(c: &mut Ctx, status: StatusCode, join_path: String, description: String, refusal: Option<InviteRefusal>, mode: ResponseMode) -> Result {
    if mode == ResponseMode::Json {
        let (workspace, help_contact) = join_workspace(c, description).await?;
        let page = match refusal {
            None => JoinPage::Join { workspace, help_contact },
            Some(refusal) => JoinPage::InviteInvalid { workspace, help_contact, refusal, reason: refusal_reason(refusal).to_owned() },
        };
        return auth::json(c, status, &page);
    }
    c.respond_to(&[&format::HTML])?;
    c.no_store();
    let help_contact = c.app().db.read(presenters::accounts::help_contact).await.map_err(Error::internal)?;
    let invite_error = refusal.map(refusal_reason);
    retained_page!(c, status, |ctx| campfire_retained::users::New {
        ctx,
        join_path: join_path.clone(),
        description: description.clone(),
        help_contact: help_contact.clone(),
        invite_error
    }).await
}

/// The workspace's name, logo and description, and the help contact, as the join page shows them.
async fn join_workspace(c: &Ctx, description: String) -> Result<(SignInWorkspace, Option<SignInHelpContact>)> {
    let (account, help_contact) = c
        .app()
        .db
        .read(|conn| Ok((Account::first(conn)?, presenters::accounts::help_contact(conn)?)))
        .await
        .map_err(Error::internal)?;
    let logo_url = match &account {
        Some(account) => presenters::workspace_branding::for_account(c.app(), account).await?.logo_url,
        None => None,
    };
    let workspace = SignInWorkspace { name: account.map(|account| account.name), logo_url, description };
    let help_contact = help_contact.map(|owner| SignInHelpContact { name: owner.name, email_address: owner.email_address });
    Ok((workspace, help_contact))
}

/// The retained page's status for each refusal: gone, or not found for a token nobody issued.
fn refusal_status(refusal: InviteRefusal) -> StatusCode {
    match refusal {
        InviteRefusal::Unknown => StatusCode::NOT_FOUND,
        _ => StatusCode::GONE,
    }
}

fn refusal_reason(refusal: InviteRefusal) -> &'static str {
    match refusal {
        InviteRefusal::Expired => "It has expired.",
        InviteRefusal::Exhausted => "All its uses have been taken.",
        InviteRefusal::Revoked => "It has been revoked.",
        InviteRefusal::Unknown => "The invite could not be found.",
    }
}

async fn verify_invite(c: &mut Ctx, mode: ResponseMode) -> Result<()> {
    let Some(refusal) = invite_refusal(c).await? else { return Ok(()) };
    halt(join_page(c, refusal_status(refusal), String::new(), String::new(), Some(refusal), mode).await?)
}

/// Why `params[:token]` can't enroll anyone, if it can't.
async fn invite_refusal(c: &Ctx) -> Result<Option<InviteRefusal>> {
    let token = c.param_str("token").unwrap_or_default().to_owned();
    let invite = c.app().db.read(move |conn| WorkspaceInvite::find_by_token(conn, &token)).await.map_err(Error::internal)?;
    Ok(match invite.map(|invite| invite.state(campfire_db::Timestamp::from_jiff(c.now()))) {
        Some(InviteState::Active) => None,
        Some(InviteState::Expired) => Some(InviteRefusal::Expired),
        Some(InviteState::Exhausted) => Some(InviteRefusal::Exhausted),
        Some(InviteState::Revoked) => Some(InviteRefusal::Revoked),
        None => Some(InviteRefusal::Unknown),
    })
}

/// The status the retained join or invite page answers for these params, for the SPA shell that
/// draws its counterpart (`/app/join/:join_code`, `/app/invite/:token`).
pub async fn page_status(c: &Ctx, invite: bool) -> Result<StatusCode> {
    if invite {
        return Ok(invite_refusal(c).await?.map_or(StatusCode::OK, refusal_status));
    }
    Ok(if join_code_account(c).await?.1 { StatusCode::OK } else { StatusCode::NOT_FOUND })
}

async fn create_user(c: &mut Ctx, invite_token: Option<String>, mode: ResponseMode) -> Result {
    if mode == ResponseMode::Json {
        user_params_from_join(c)?;
    }
    let params = user_params(c)?;
    let email_address = params.get("email_address").and_then(|p| p.to_s());
    let attributes = NewUser {
        // users.name is NOT NULL: a missing name fails the insert, as in Rails.
        name: params.get("name").and_then(|p| p.to_s()).ok_or_else(|| Error::internal(anyhow::anyhow!("NOT NULL constraint failed: users.name")))?,
        email_address: email_address.clone(),
        password_digest: concerns::password_digest(c, params.get("password").and_then(|p| p.to_s()).filter(|password| !password.is_empty())).await?,
        ..NewUser::default()
    };
    let avatar = Assignment::from_params(&params, "avatar")?.stage(c.app()).await?;

    // `User.create!(user_params)`
    let result = c
        .app()
        .db
        .write(move |tx| {
            let user = match invite_token {
                Some(token) => WorkspaceInvite::redeem(tx, &token, attributes)?,
                None => Some(User::create(tx, attributes)?),
            };
            let Some(user) = user else { return Ok(None) };
            attachments::assign(tx, Record::user(user.id), "avatar", avatar)?;
            Ok(Some(user))
        })
        .await;
    match result {
        Ok(Some(user)) => {
            concerns::start_new_session_for(c, user).await?;
            let root = c.url_for(&campfire_routes::root());
            mode.signed_in(c, &root)
        }
        Ok(None) => {
            verify_invite(c, mode).await?;
            Err(Error::NotFound)
        }
        // rescue ActiveRecord::RecordNotUnique: `redirect_to new_session_url(email_address: user_params[:email_address])`
        Err(error) if presenters::accounts::is_record_not_unique(&error) => {
            let mut location = c.url_for(&campfire_routes::new_session());
            if let Some(email_address) = email_address {
                location.push_str(&format!("?email_address={}", campfire_presentation::helpers::url::cgi_escape(&email_address)));
            }
            mode.navigate(c, &location)
        }
        Err(error) => Err(Error::internal(error)),
    }
}

/// The JSON endpoints' [`WorkspaceJoin`] fields (multipart parts or a JSON body) and the optional
/// `avatar` part, as the retained form's `user` params. Nothing else the request carries reaches
/// `user_params`.
fn user_params_from_join(c: &mut Ctx) -> Result<()> {
    let text = |key: &str| c.params.get(key).and_then(Param::as_str).map(str::to_owned);
    let (Some(name), Some(email_address), Some(password)) = (text("name"), text("emailAddress"), text("password")) else {
        return halt(auth::field_error(c, StatusCode::UNPROCESSABLE_ENTITY, "base", "The request body isn't valid.")?);
    };
    let join = WorkspaceJoin { name, email_address, password };
    let mut user = ParamMap::new();
    user.insert("name", Param::Str(join.name));
    user.insert("email_address", Param::Str(join.email_address));
    user.insert("password", Param::Str(join.password));
    if let Some(avatar) = c.params.get("avatar").cloned() {
        user.insert("avatar", avatar);
    }
    c.params.insert("user", Param::Hash(user));
    Ok(())
}

/// `User.find(params[key])`: 404 when there's no such user.
pub async fn find_user(c: &Ctx, key: &str) -> Result<User> {
    let id = c.param_str(key).and_then(cast_integer).ok_or(Error::NotFound)?;
    c.app().db.read(move |conn| User::find_by_id(conn, id)).await.map_err(Error::internal)?.ok_or(Error::NotFound)
}

/// `head :not_found if Current.account.join_code != params[:join_code]`
async fn verify_join_code(c: &mut Ctx) -> Result<Account> {
    let (account, valid) = join_code_account(c).await?;
    if !valid {
        return halt(c.head(StatusCode::NOT_FOUND));
    }
    Ok(account)
}

/// `Current.account`, and whether `params[:join_code]` is its join code (or its vanity slug).
async fn join_code_account(c: &Ctx) -> Result<(Account, bool)> {
    let account = c
        .app()
        .db
        .read(Account::first)
        .await
        .map_err(Error::internal)?
        // `Current.account.join_code` on nil raises NoMethodError.
        .ok_or_else(|| Error::internal(anyhow::anyhow!("undefined method 'join_code' for nil")))?;
    let settings = account.settings();
    let valid = c
        .param_str("join_code")
        .is_some_and(|code| code == account.join_code || settings.vanity_slug() == Some(code));
    Ok((account, valid))
}

/// `params.require(:user).permit(:name, :avatar, :email_address, :password)`
fn user_params(c: &Ctx) -> Result<ParamMap> {
    Ok(c.params.require("user")?.permit(&permit_keys(&["name", "avatar", "email_address", "password"])))
}
