//! `/api/v1/admin/bots` (S7): chat bots and their agents, the SPA's twin of the classic
//! `accounts/bots` pages, the bot key, the webhook signing secret, the GitHub connection, the kill
//! switch, and `accounts/bots/credentials` and `accounts/bots/grants`.
//!
//! Every write runs the classic controller's own save path (`campfire_people`'s
//! `accounts::bots` functions the classic actions call too), after the classic before-actions
//! in the classic order. Only the password confirmation differs, as in [`crate::admin`]: a lapsed
//! one answers `SudoRequired` with a visit to the asking SPA page stashed for `/sudo`.

use axum::Router;
use axum::routing::{get, post, put};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::User;
use campfire_db::models::user::icon::normalize_input;
use campfire_db::{AgentChanges, Timestamp};
use campfire_kit::{Ctx, Error, Kit, Param, Result, StatusCode, action, unparsed_action};
use campfire_people::controllers::accounts::bots::{
    self as classic, BotUpdate, NewBot, credentials, github_connections, grants, keys,
    webhook_secrets,
};
use campfire_presentation::accounts::bot_access::{CAPABILITIES, CredentialExpiry};
use campfire_presentation::helpers::AvatarIcon;
use campfire_runtime::concerns;
use campfire_runtime::presenters::attachments::Assignment;
use campfire_runtime::presenters::{self, accounts as account_presenters};
use serde_json::json;

use crate::admin::{administrator, body, refusal, require_sudo, viewer};
use crate::dto::time;
use crate::error::{fail, validation};

/// The bot routes, merged into [`crate::routes`].
pub fn routes() -> Router<Kit> {
    Router::new()
        .route(
            "/api/v1/admin/bots",
            get(action(bots)).post(unparsed_action(create_bot)),
        )
        .route(
            "/api/v1/admin/bots/{id}",
            get(action(bot))
                .patch(unparsed_action(update_bot))
                .delete(action(remove_bot)),
        )
        .route(
            "/api/v1/admin/bots/{id}/kill_switch",
            post(action(suspend_bot)),
        )
        .route("/api/v1/admin/bots/{id}/key", put(action(reset_bot_key)))
        .route(
            "/api/v1/admin/bots/{id}/webhook_secret",
            post(action(reset_webhook_secret)),
        )
        .route(
            "/api/v1/admin/bots/{id}/github_connection",
            put(unparsed_action(connect_github)).delete(action(disconnect_github)),
        )
        .route(
            "/api/v1/admin/bots/{id}/credentials",
            get(action(bot_credentials)).post(unparsed_action(issue_credential)),
        )
        .route(
            "/api/v1/admin/bots/{id}/credentials/{credential_id}",
            axum::routing::delete(action(revoke_credential)),
        )
        .route(
            "/api/v1/admin/bots/{id}/grants",
            get(action(bot_grants)).post(unparsed_action(create_grant)),
        )
        .route(
            "/api/v1/admin/bots/{id}/grants/{grant_id}",
            axum::routing::delete(action(revoke_grant)),
        )
}

endpoint!(
    /// `GET /api/v1/admin/bots`
    bots => index_bots
);
endpoint!(
    /// `POST /api/v1/admin/bots`
    create_bot => save_new_bot
);
endpoint!(
    /// `GET /api/v1/admin/bots/:id`
    bot => show_bot
);
endpoint!(
    /// `PATCH /api/v1/admin/bots/:id`
    update_bot => save_bot
);
endpoint!(
    /// `DELETE /api/v1/admin/bots/:id`
    remove_bot => deactivate_bot
);
endpoint!(
    /// `POST /api/v1/admin/bots/:id/kill_switch`
    suspend_bot => kill_switch
);
endpoint!(
    /// `PUT /api/v1/admin/bots/:id/key`
    reset_bot_key => new_key
);
endpoint!(
    /// `POST /api/v1/admin/bots/:id/webhook_secret`
    reset_webhook_secret => new_webhook_secret
);
endpoint!(
    /// `PUT /api/v1/admin/bots/:id/github_connection`
    connect_github => link_github
);
endpoint!(
    /// `DELETE /api/v1/admin/bots/:id/github_connection`
    disconnect_github => unlink_github
);
endpoint!(
    /// `GET /api/v1/admin/bots/:id/credentials`
    bot_credentials => index_credentials
);
endpoint!(
    /// `POST /api/v1/admin/bots/:id/credentials`
    issue_credential => save_credential
);
endpoint!(
    /// `DELETE /api/v1/admin/bots/:id/credentials/:credential_id`
    revoke_credential => delete_credential
);
endpoint!(
    /// `GET /api/v1/admin/bots/:id/grants`
    bot_grants => index_grants
);
endpoint!(
    /// `POST /api/v1/admin/bots/:id/grants`
    create_grant => save_grant
);
endpoint!(
    /// `DELETE /api/v1/admin/bots/:id/grants/:grant_id`
    revoke_grant => delete_grant
);

// --- Helpers -----------------------------------------------------------------------------------

/// `User.active_bots.find(params[:id])`: anything else is not found.
async fn active_bot(c: &Ctx) -> Result<User> {
    classic::find_active_bot(c, "id").await
}

/// A record id from the path; anything else is not found.
fn path_id(c: &Ctx, key: &str) -> Result<i64> {
    c.param_str(key)
        .and_then(concerns::cast_integer)
        .ok_or(Error::NotFound)
}

/// `root_url` without its slash: what the classic page's absolute URLs start with.
fn base_url(c: &Ctx) -> String {
    c.url_for("")
}

/// An icon on the wire, its picture's URL as the page's `image_tag` resolves it.
fn icon(icon: AvatarIcon) -> api::BotIcon {
    match icon {
        AvatarIcon::Emoji { title, character } => api::BotIcon::Emoji { title, character },
        AvatarIcon::Image { title, url, .. } => api::BotIcon::Image {
            title,
            url: if url.starts_with('/') || url.contains("://") {
                url
            } else {
                campfire_static_assets::asset_path(&url)
            },
        },
    }
}

/// A verified direct upload, as the classic form's `avatar` field gives it.
fn signed_upload(c: &mut Ctx, field: &str, signed_id: Option<String>) -> Result<Assignment> {
    let Some(signed_id) = signed_id else {
        return Ok(Assignment::Unchanged);
    };
    let verified = campfire_storage::paths::verify_signed_blob_id(
        &*c.app().storage.verifier,
        &signed_id,
        c.app().clock.now(),
    );
    if verified.is_none() {
        return Err(fail(c, validation(field, "isn't an uploaded file")));
    }
    Ok(Assignment::Signed(signed_id))
}

/// A form's icon field: an empty or unknown name clears it, as `icon_name=` normalizes it.
fn icon_input(value: &str) -> Option<String> {
    normalize_input(&json!(value))
}

/// A save's outcome as the API answers it: `RecordInvalid` is the 422 envelope.
fn saved<T>(outcome: campfire_db::Result<T>) -> Result<T> {
    outcome.map_err(Error::internal)
}

// --- The list and one bot ----------------------------------------------------------------------

/// `accounts/bots#index`: the active bots, by name.
async fn index_bots(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let secrets = c.app().secrets.clone();
    let bots = c
        .app()
        .db
        .read(move |conn| {
            account_presenters::bots(conn, &secrets, &User::active_bots_ordered(conn)?)
        })
        .await
        .map_err(Error::internal)?;
    let base = base_url(c);
    let bots = bots
        .into_iter()
        .map(|bot| {
            let ownership = bot.ownership_line();
            api::BotSummary {
                id: bot.user.id,
                name: bot.user.name,
                avatar_url: bot.user.avatar_path,
                icon: bot.icon.map(icon),
                ownership,
                rooms: bot
                    .rooms
                    .into_iter()
                    .map(|room| {
                        let url = format!(
                            "{base}{}",
                            campfire_routes::room_bot_messages(room.id, "BOT_KEY")
                        );
                        api::BotRoom {
                            id: room.id,
                            name: room.name,
                            message_command: campfire_presentation::helpers::curl_text_line(&url),
                            attachment_command: campfire_presentation::helpers::curl_upload_line(&url),
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    c.json(StatusCode::OK, &api::BotList { bots })
}

/// `bot` as its classic edit page shows it.
async fn load_bot(c: &Ctx, bot: &User) -> Result<api::Bot> {
    let form = classic::edit_form(c, bot).await?;
    let secrets = c.app().secrets.clone();
    let summary = presenters::user_summary(&secrets, bot);
    let can_administer = concerns::current_user(c).is_some_and(User::is_administrator);
    Ok(api::Bot {
        id: bot.id,
        name: form.name.unwrap_or_else(|| bot.name.clone()),
        avatar_url: summary.avatar_path,
        avatar_attached: form.avatar_attachment_url.is_some(),
        icon_name: form.icon_name,
        icon: form.icon.map(icon),
        webhook_url: form.webhook_url,
        can_administer,
        agent: form.agent.map(|agent| api::BotAgent {
            id: agent.id,
            provider: agent.provider,
            runtime: agent.runtime,
            description: agent.description,
            daily_message_cap: agent.daily_message_cap,
            daily_board_post_cap: agent.daily_board_post_cap,
            daily_external_action_cap: agent.daily_external_action_cap,
            usage: form.budget_usage_line.clone(),
            suspended: agent.suspended,
            ledger_url: campfire_routes::agent_events(agent.id),
            approvals_url: campfire_routes::agent_approvals(agent.id),
        }),
        signing_secret: form.signing_secret.filter(|secret| !secret.is_empty()),
        github: form.github.map(|github| api::BotGithub {
            login: github.login,
            usable: github.usable,
            disconnected_reason: github.disconnected_reason,
        }),
    })
}

/// The bot (read afresh) and a notice, as a bot write answers.
async fn reply_bot(c: &mut Ctx, id: i64, notice: Option<String>) -> Result {
    let bot = c
        .app()
        .db
        .read(move |conn| User::find(conn, id))
        .await
        .map_err(Error::internal)?;
    let bot = load_bot(c, &bot).await?;
    c.json(StatusCode::OK, &api::BotChange { bot, notice })
}

/// `accounts/bots#edit`: administrators and the agent's owner.
async fn show_bot(c: &mut Ctx) -> Result {
    viewer(c).await?;
    let bot = active_bot(c).await?;
    classic::ensure_can_manage_bot(c, &bot).await?;
    let bot = load_bot(c, &bot).await?;
    c.json(StatusCode::OK, &bot)
}

/// A bot's key, shown once, with the classic page's example command.
fn reply_key(c: &mut Ctx, bot: &User, key: String) -> Result {
    let url = format!(
        "{}{}",
        base_url(c),
        campfire_routes::room_bot_messages("ROOM_ID", &key)
    );
    c.json(
        StatusCode::OK,
        &api::BotKey {
            id: bot.id,
            name: bot.name.clone(),
            example_command: campfire_presentation::helpers::curl_text_line(&url),
            key,
        },
    )
}

// --- Writes ------------------------------------------------------------------------------------

/// `accounts/bots#create`: administrators, with the password confirmed.
async fn save_new_bot(c: &mut Ctx) -> Result {
    administrator(c).await?;
    require_sudo(c)?;
    let create: api::CreateBot = body(c).await?;
    let avatar = signed_upload(c, "avatar", create.avatar)?
        .stage(c.app())
        .await?;
    let new = NewBot {
        name: create.name,
        icon_name: create.icon_name.as_deref().and_then(icon_input),
        webhook_url: create.webhook_url,
        avatar,
    };
    let bot = saved(classic::create_bot(c, new).await?)?;
    let key = bot.bot_key();
    reply_key(c, &bot, key)
}

/// The agent fields as the classic form posts them: text, a budget's as typed.
fn agent_changes(agent: Option<api::UpdateBotAgent>) -> AgentChanges {
    let Some(agent) = agent else {
        return AgentChanges::default();
    };
    let cap = |value: Option<String>| value.map(serde_json::Value::String);
    AgentChanges {
        provider: agent.provider.map(Some),
        runtime: agent.runtime.map(Some),
        description: agent.description.map(Some),
        daily_message_cap_before_type_cast: cap(agent.daily_message_cap),
        daily_board_post_cap_before_type_cast: cap(agent.daily_board_post_cap),
        daily_external_action_cap_before_type_cast: cap(agent.daily_external_action_cap),
        ..Default::default()
    }
}

/// `accounts/bots#update`: administrators and the agent's owner; a webhook URL change is an
/// administrator's, with the password confirmed.
async fn save_bot(c: &mut Ctx) -> Result {
    viewer(c).await?;
    let bot = active_bot(c).await?;
    classic::ensure_can_manage_bot(c, &bot).await?;
    let update: api::UpdateBot = body(c).await?;
    if classic::webhook_url_changing(c, &bot, update.webhook_url.as_deref()).await? {
        concerns::ensure_can_administer(c)?;
        require_sudo(c)?;
    }
    let avatar = signed_upload(c, "avatar", update.avatar)?
        .stage(c.app())
        .await?;
    let id = bot.id;
    let changes = BotUpdate {
        name: update.name,
        icon_name: update.icon_name.as_deref().map(icon_input),
        webhook_url: update.webhook_url.map(Some),
        agent: agent_changes(update.agent),
        avatar,
    };
    saved(classic::update_bot(c, bot, changes).await?)?;
    reply_bot(c, id, None).await
}

/// `accounts/bots#destroy`: administrators; the bot is deactivated, its agent suspended.
async fn deactivate_bot(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let bot = active_bot(c).await?;
    let id = bot.id;
    classic::remove_bot(c, bot).await?;
    c.json(StatusCode::OK, &api::BotRemoved { id })
}

/// `accounts/bots#kill_switch`: administrators and the owner, without a password prompt.
async fn kill_switch(c: &mut Ctx) -> Result {
    viewer(c).await?;
    let bot = active_bot(c).await?;
    classic::ensure_can_manage_bot(c, &bot).await?;
    let Some(cancelled) = classic::suspend_agent(c, &bot).await? else {
        return Err(Error::NotFound);
    };
    let plural = if cancelled == 1 { "" } else { "s" };
    let notice = format!("Agent suspended; {cancelled} approval{plural} cancelled.");
    reply_bot(c, bot.id, Some(notice)).await
}

/// `accounts/bots/keys#update`: administrators, with the password confirmed.
async fn new_key(c: &mut Ctx) -> Result {
    administrator(c).await?;
    require_sudo(c)?;
    let bot = active_bot(c).await?;
    let key = keys::reset_key(c, bot.clone()).await?;
    reply_key(c, &bot, key)
}

/// `accounts/bots/webhook_secrets#create`: administrators and the owner, with the password
/// confirmed.
async fn new_webhook_secret(c: &mut Ctx) -> Result {
    viewer(c).await?;
    let bot = active_bot(c).await?;
    classic::ensure_can_manage_bot(c, &bot).await?;
    require_sudo(c)?;
    let id = bot.id;
    if !webhook_secrets::reset_signing_secret(c, bot).await? {
        return Err(refusal(
            c,
            "Set a webhook URL before generating a signing secret.",
        ));
    }
    let notice = "Signing secret reset. Update the receiving service with the new secret.";
    reply_bot(c, id, Some(notice.into())).await
}

/// `accounts/bots/github_connections#create`: administrators, with the password confirmed. A
/// token GitHub won't take is refused with the classic alert.
async fn link_github(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let bot = active_bot(c).await?;
    require_sudo(c)?;
    let connect: api::ConnectGithub = body(c).await?;
    let id = bot.id;
    let (message, notice) = github_connections::connect(c, bot, &connect.access_token).await?;
    if !notice {
        return Err(fail(
            c,
            api::ApiError::Validation {
                fields: [("accessToken".to_string(), vec![message.clone()])].into(),
                message,
            },
        ));
    }
    reply_bot(c, id, Some(message)).await
}

/// `accounts/bots/github_connections#destroy`: administrators, with the password confirmed.
async fn unlink_github(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let bot = active_bot(c).await?;
    require_sudo(c)?;
    let id = bot.id;
    github_connections::disconnect(c, bot).await?;
    reply_bot(c, id, Some("GitHub disconnected.".into())).await
}

// --- Credentials -------------------------------------------------------------------------------

/// The bot's credentials, newest first, as `accounts/bots/credentials#index` lists them.
async fn credential_list(c: &Ctx, bot: &User, agent_id: i64) -> Result<api::CredentialList> {
    let zone = classic::viewer_zone(c).await?;
    let rows = c
        .app()
        .db
        .read(move |conn| account_presenters::bot_access::credentials(conn, agent_id, &zone))
        .await
        .map_err(Error::internal)?;
    let now = c.now();
    let credentials = rows
        .into_iter()
        .map(|row| {
            let expired = row.expires_at.as_ref().is_some_and(|at| at.passed(now));
            api::Credential {
                id: row.id,
                name: row.name,
                last_four: row.last_four,
                created_by: row.created_by,
                created_at: time(Timestamp::from_jiff(row.created_at)),
                last_used_at: row.last_used_at.map(|at| time(Timestamp::from_jiff(at))),
                expires_at: row.expires_at.map(|at| match at {
                    CredentialExpiry::Time(at) => time(Timestamp::from_jiff(at)),
                    CredentialExpiry::Extended { datetime, .. } => datetime,
                }),
                state: if row.revoked {
                    api::CredentialState::Revoked
                } else if expired {
                    api::CredentialState::Expired
                } else {
                    api::CredentialState::Active
                },
            }
        })
        .collect();
    Ok(api::CredentialList {
        bot_id: bot.id,
        bot_name: bot.name.clone(),
        can_issue: concerns::current_user(c).is_some_and(User::is_administrator),
        credentials,
    })
}

/// `accounts/bots/credentials#index`: administrators and the owner; a legacy bot gets its agent.
async fn index_credentials(c: &mut Ctx) -> Result {
    viewer(c).await?;
    let bot = active_bot(c).await?;
    classic::ensure_can_manage_bot(c, &bot).await?;
    let agent = classic::ensure_agent(c, &bot).await?;
    let list = credential_list(c, &bot, agent.id).await?;
    c.json(StatusCode::OK, &list)
}

/// `accounts/bots/credentials#create`: administrators, with the password confirmed. The secret
/// is shown once.
async fn save_credential(c: &mut Ctx) -> Result {
    viewer(c).await?;
    let bot = active_bot(c).await?;
    concerns::ensure_can_administer(c)?;
    require_sudo(c)?;
    let agent = classic::ensure_agent(c, &bot).await?;
    let create: api::CreateCredential = body(c).await?;
    let zone = classic::viewer_zone(c).await?;
    let raw = create.expires_at.map(Param::Str);
    let expires_at = campfire_runtime::presenters::bot_input_casts::datetime(
        raw.as_ref(),
        &zone,
        Timestamp::from_jiff(c.now()),
    )
    .map_err(Error::internal)?;
    let secret = saved(
        credentials::issue(
            c,
            &bot,
            agent.id,
            create.name,
            credentials::Expiry::At(expires_at),
        )
        .await?,
    )?;
    let credentials = credential_list(c, &bot, agent.id).await?;
    c.json(
        StatusCode::OK,
        &api::CredentialCreated {
            secret,
            credentials,
        },
    )
}

/// `accounts/bots/credentials#destroy`: administrators and the owner, with the password
/// confirmed.
async fn delete_credential(c: &mut Ctx) -> Result {
    viewer(c).await?;
    let bot = active_bot(c).await?;
    classic::ensure_can_manage_bot(c, &bot).await?;
    require_sudo(c)?;
    let agent = classic::ensure_agent(c, &bot).await?;
    let id = path_id(c, "credential_id")?;
    if !credentials::revoke(c, &bot, agent.id, id).await? {
        return Err(Error::NotFound);
    }
    let list = credential_list(c, &bot, agent.id).await?;
    c.json(StatusCode::OK, &list)
}

// --- Grants ------------------------------------------------------------------------------------

/// The bot's grants (active first) and what the form offers, as `accounts/bots/grants#index`
/// shows them.
async fn grant_list(c: &Ctx, bot: &User, agent_id: i64) -> Result<api::GrantList> {
    let (bot_id, viewer) = (bot.id, concerns::require_current_user(c)?.clone());
    let (legacy, rows, rooms) = c
        .app()
        .db
        .read(move |conn| account_presenters::bot_access::grants(conn, agent_id, bot_id, &viewer))
        .await
        .map_err(Error::internal)?;
    Ok(api::GrantList {
        bot_id: bot.id,
        bot_name: bot.name.clone(),
        can_grant: concerns::current_user(c).is_some_and(User::is_administrator),
        legacy,
        grants: rows
            .into_iter()
            .map(|row| api::Grant {
                id: row.id,
                capability: row.capability,
                room_name: row.room_name,
                granted_by: row.granted_by,
                created_at: time(Timestamp::from_jiff(row.created_at)),
                revoked: row.revoked,
            })
            .collect(),
        capabilities: CAPABILITIES.iter().map(|name| name.to_string()).collect(),
        rooms: rooms
            .into_iter()
            .filter_map(|(name, id)| id.parse().ok().map(|id| api::GrantRoom { id, name }))
            .collect(),
    })
}

/// `accounts/bots/grants#index`: administrators and the owner; a legacy bot gets its agent.
async fn index_grants(c: &mut Ctx) -> Result {
    viewer(c).await?;
    let bot = active_bot(c).await?;
    classic::ensure_can_manage_bot(c, &bot).await?;
    let agent = classic::ensure_agent(c, &bot).await?;
    let list = grant_list(c, &bot, agent.id).await?;
    c.json(StatusCode::OK, &list)
}

/// `accounts/bots/grants#create`: administrators, with the password confirmed. An active grant
/// already there is left as it is.
async fn save_grant(c: &mut Ctx) -> Result {
    viewer(c).await?;
    let bot = active_bot(c).await?;
    classic::ensure_can_manage_bot(c, &bot).await?;
    concerns::ensure_can_administer(c)?;
    require_sudo(c)?;
    let agent = classic::ensure_agent(c, &bot).await?;
    let create: api::CreateGrant = body(c).await?;
    // The classic form offers only the rooms the bot is in; a room id from anywhere else
    // (`0`, a room it isn't in) is refused here rather than saved as an inert grant.
    if let Some(room_id) = create.room_id {
        let offered = grant_list(c, &bot, agent.id).await?;
        if !offered.rooms.iter().any(|room| room.id == room_id) {
            return Err(fail(c, validation("room_id", "isn't one of the bot's rooms")));
        }
    }
    match grants::grant(c, &bot, agent.id, create.capability, create.room_id).await? {
        Ok(()) => (),
        Err(error) if error.is_record_not_unique() => (),
        Err(error) => return Err(Error::internal(error)),
    }
    let list = grant_list(c, &bot, agent.id).await?;
    c.json(StatusCode::OK, &list)
}

/// `accounts/bots/grants#destroy`: administrators and the owner, with the password confirmed.
async fn delete_grant(c: &mut Ctx) -> Result {
    viewer(c).await?;
    let bot = active_bot(c).await?;
    classic::ensure_can_manage_bot(c, &bot).await?;
    require_sudo(c)?;
    let agent = classic::ensure_agent(c, &bot).await?;
    let id = path_id(c, "grant_id")?;
    if !grants::revoke(c, &bot, agent.id, id).await? {
        return Err(Error::NotFound);
    }
    let list = grant_list(c, &bot, agent.id).await?;
    c.json(StatusCode::OK, &list)
}
