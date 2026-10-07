//! `Accounts::BotsController` (reference/app/controllers/accounts/bots_controller.rb).

pub mod credentials;
pub mod github_connections;
pub mod grants;
pub use crate::controllers::presenters::bot_input_casts as input_casts;
pub mod keys;
pub mod webhook_secrets;

use campfire_db::models::audit_log::{self, AuditLog, Context, NewAuditLog, Target};
use campfire_db::{Agent, AgentChanges, AgentKind, NewAgent, NewUser, User, UserChanges};
use campfire_kit::{Ctx, Error, Param, ParamMap, Result, StatusCode, format, permit_keys};
use campfire_views::accounts;
use serde_json::{Value, json};

use crate::app::AppCtx;
use crate::concerns::{self, Before, cast_integer};
use crate::controllers::presenters;
use crate::controllers::presenters::attachments::{self, Assignment, Record};
use crate::controllers::presenters::page::framed_page;

/// `@bots = User.active_bots.ordered`
pub async fn index(c: &mut Ctx) -> Result {
    before(c).await?;
    c.respond_to(&[&format::HTML])?;
    let secrets = c.app().secrets.clone();
    let bots: Vec<_> = c
        .app()
        .db
        .read(move |conn| {
            presenters::accounts::bots(conn, &secrets, &User::active_bots_ordered(conn)?)
        })
        .await
        .map_err(Error::internal)?;
    framed_page!(c, StatusCode::OK, |ctx| accounts::BotsIndex {
        ctx,
        bots: bots.clone()
    })
    .await
}

pub async fn new(c: &mut Ctx) -> Result {
    before(c).await?;
    c.respond_to(&[&format::HTML])?;
    framed_page!(c, StatusCode::OK, |ctx| accounts::BotsNew {
        ctx,
        bot: accounts::BotForm::default()
    })
    .await
}

/// Rails creates a workspace Agent and reveals the digest-backed key once.
pub async fn create(c: &mut Ctx) -> Result {
    before(c).await?;
    concerns::sudo::require_sudo_mode(c)?;
    let params = bot_params(c)?;
    let name = params.get("name").and_then(Param::to_s).ok_or_else(|| {
        Error::internal(anyhow::anyhow!("NOT NULL constraint failed: users.name"))
    })?;
    let webhook_url = params.get("webhook_url").and_then(Param::to_s);
    let icon_name = icon_attribute(&params).flatten();
    let avatar = Assignment::from_params(&params, "avatar")?
        .stage(c.app())
        .await?;
    let result = create_bot(
        c,
        NewBot {
            name,
            icon_name,
            webhook_url,
            avatar,
        },
    )
    .await?;
    match result {
        Ok(bot) => {
            c.set_header("cache-control", "no-store");
            c.set_header("pragma", "no-cache");
            let key = bot.bot_key();
            framed_page!(c, StatusCode::CREATED, |ctx| accounts::BotKey {
                ctx,
                bot_name: &bot.name,
                bot_key: &key
            })
            .await
        }
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            let form = accounts::BotForm {
                name: params.get("name").and_then(Param::to_s),
                webhook_url: params.get("webhook_url").and_then(Param::to_s),
                icon_name: icon_attribute(&params).flatten(),
                errors: Some(errors.to_string()),
                error_fields: errors
                    .0
                    .iter()
                    .map(|(field, _)| field.to_string())
                    .collect(),
                ..Default::default()
            };
            framed_page!(c, StatusCode::UNPROCESSABLE_ENTITY, |ctx| {
                accounts::BotsNew {
                    ctx,
                    bot: form.clone(),
                }
            })
            .await
        }
        Err(error) => Err(Error::internal(error)),
    }
}

/// A new bot, as `bot_params` give it on create.
pub struct NewBot {
    pub name: String,
    pub icon_name: Option<String>,
    pub webhook_url: Option<String>,
    pub avatar: Assignment<campfire_storage::Staged>,
}

/// `create`'s writes once the gates passed: the bot, its webhook, its workspace agent owned by
/// the viewer, and the audit. The inner result is the save's (`RecordInvalid` for the form).
pub async fn create_bot(c: &Ctx, new: NewBot) -> Result<campfire_db::Result<User>> {
    let NewBot {
        name,
        icon_name,
        webhook_url,
        avatar,
    } = new;
    let owner_id = concerns::require_current_user(c)?.id;
    let context = audit_context(c)?;
    // User.create_bot!, create_agent! and AuditLog.record! are independent Rails
    // saves. A later failure must leave the earlier committed rows observable.
    Ok(async {
        let bot = c
            .app()
            .db
            .write(move |tx| {
                let bot = User::create_bot_with_attributes(
                    tx,
                    NewUser {
                        name,
                        icon_name,
                        ..Default::default()
                    },
                    None,
                )?;
                attachments::assign(tx, Record::user(bot.id), "avatar", avatar)?;
                Ok(bot)
            })
            .await?;
        if let Some(url) = webhook_url {
            let id = bot.id;
            c.app()
                .db
                .write(move |tx| campfire_db::Webhook::create(tx, id, Some(&url)))
                .await?;
        }
        let id = bot.id;
        let agent = c
            .app()
            .db
            .write(move |tx| {
                Agent::create(
                    tx,
                    NewAgent {
                        user_id: id,
                        owner_id: Some(owner_id),
                        kind: AgentKind::Workspace,
                        ..Default::default()
                    },
                )
            })
            .await?;
        let audit = NewAuditLog {
            action: "agent.create".into(),
            target: Some(audit_target(&bot, Some(&agent))),
            changes: Some(json!({"name":bot.name,"kind":"workspace"})),
            ..Default::default()
        };
        c.app()
            .db
            .write(move |tx| AuditLog::record(tx, audit, &context))
            .await?;
        Ok::<_, campfire_db::Error>(bot)
    }
    .await)
}

pub async fn edit(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = set_bot(c).await?;
    ensure_can_manage_bot(c, &bot).await?;
    let bot_id = bot.id;
    c.respond_to(&[&format::HTML])?;
    let form = edit_form(c, &bot).await?;
    framed_page!(c, StatusCode::OK, |ctx| accounts::BotsEdit {
        ctx,
        bot_id,
        bot: form.clone()
    })
    .await
}

/// Authorization stays in the controller; validated writes use WS11's domain.
pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = set_bot(c).await?;
    ensure_can_manage_bot(c, &bot).await?;
    let submitted_url = c
        .params
        .get("user")
        .and_then(Param::as_hash)
        .filter(|params| params.contains_key("webhook_url"))
        .map(|params| {
            params
                .get("webhook_url")
                .and_then(Param::to_s)
                .unwrap_or_default()
        });
    if webhook_url_changing(c, &bot, submitted_url.as_deref()).await? {
        concerns::ensure_can_administer(c)?;
        concerns::sudo::require_sudo_mode(c)?;
    }
    let params = bot_params(c)?;
    let agent_changes = agent_params(c);
    let requested_agent = agent_changes.clone();
    let avatar = Assignment::from_params(&params, "avatar")?
        .stage(c.app())
        .await?;
    let update = BotUpdate {
        name: params.get("name").and_then(Param::to_s),
        icon_name: icon_attribute(&params),
        webhook_url: params
            .contains_key("webhook_url")
            .then(|| params.get("webhook_url").and_then(Param::to_s)),
        agent: agent_changes,
        avatar,
    };
    let result = update_bot(c, bot.clone(), update).await?;
    match result {
        Ok(()) => redirect_to_bots(c),
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            let bot = set_bot(c).await?;
            let mut form = edit_form(c, &bot).await?;
            if errors
                .0
                .iter()
                .any(|(field, _)| matches!(*field, "name" | "icon_name" | "webhook_url"))
            {
                form.name = params.get("name").and_then(Param::to_s).or(form.name);
                if let Some(icon) = icon_attribute(&params) {
                    form.icon = c
                        .app()
                        .db
                        .read({
                            let icon = icon.clone();
                            move |conn| {
                                Ok(icon
                                    .as_deref()
                                    .and_then(|name| presenters::resolve_avatar_icon(conn, name)))
                            }
                        })
                        .await
                        .map_err(Error::internal)?;
                    form.icon_name = icon;
                }
                if let Some(url) = params.get("webhook_url") {
                    form.webhook_url = url.to_s();
                }
                form.errors = Some(errors.to_string());
                form.error_fields = errors
                    .0
                    .iter()
                    .map(|(field, _)| field.to_string())
                    .collect();
                if let Some(agent) = &mut form.agent {
                    apply_agent_form(agent, &requested_agent);
                }
            } else if let Some(agent) = &mut form.agent {
                apply_agent_form(agent, &requested_agent);
                agent.errors = Some(errors.to_string());
                agent.error_fields = errors
                    .0
                    .iter()
                    .map(|(field, _)| field.to_string())
                    .collect();
            }
            let bot_id = bot.id;
            framed_page!(c, StatusCode::UNPROCESSABLE_ENTITY, |ctx| {
                accounts::BotsEdit {
                    ctx,
                    bot_id,
                    bot: form.clone(),
                }
            })
            .await
        }
        Err(error) => Err(Error::internal(error)),
    }
}


/// What an edit gives, as `bot_params` and `agent_params` read the form: `None` where a key
/// wasn't given.
pub struct BotUpdate {
    pub name: Option<String>,
    pub icon_name: Option<Option<String>>,
    /// The webhook URL's value (`to_s`) when its key was given; only an administrator's counts.
    pub webhook_url: Option<Option<String>>,
    pub agent: AgentChanges,
    pub avatar: Assignment<campfire_storage::Staged>,
}

/// Whether `submitted` (the form's webhook URL, when given) differs from the bot's: the change
/// `update` lets only an administrator make, with the password confirmed.
pub async fn webhook_url_changing(c: &Ctx, bot: &User, submitted: Option<&str>) -> Result<bool> {
    let Some(url) = submitted else {
        return Ok(false);
    };
    let id = bot.id;
    let previous_url = c
        .app()
        .db
        .read(move |conn| User::find(conn, id)?.webhook_url(conn))
        .await
        .map_err(Error::internal)?;
    Ok(campfire_richtext::ruby::strip(url) != previous_url.as_deref().unwrap_or(""))
}

/// `update`'s writes once the gates passed: the bot (and webhook, avatar), the agent's dirty
/// attributes, then the webhook and agent audits. The inner result is the save's
/// (`RecordInvalid` for the form).
pub async fn update_bot(c: &Ctx, bot: User, update: BotUpdate) -> Result<campfire_db::Result<()>> {
    let BotUpdate {
        name,
        icon_name,
        webhook_url,
        agent: agent_changes,
        avatar,
    } = update;
    let mut bot = bot;
    let previous_url = c
        .app()
        .db
        .read({
            let id = bot.id;
            move |conn| User::find(conn, id)?.webhook_url(conn)
        })
        .await
        .map_err(Error::internal)?;
    let changes = UserChanges {
        name,
        icon_name,
        ..Default::default()
    };
    // Rails set_agent/assign_attributes retain a request's original values;
    // save! only persists attributes dirtied against that snapshot.
    let before_agent = c
        .app()
        .db
        .read({
            let id = bot.id;
            move |conn| Agent::for_user(conn, id)
        })
        .await
        .map_err(Error::internal)?;
    let dirty_agent = before_agent
        .as_ref()
        .map(|agent| dirty_agent_changes(agent, agent_changes.clone()))
        .unwrap_or_default();
    let audit_changes = dirty_agent.clone();
    let context = audit_context(c)?;
    // Preserve Rails' distinction between an omitted URL and a submitted blank URL.
    let webhook_submitted =
        concerns::require_current_user(c)?.is_administrator() && webhook_url.is_some();
    let webhook_url = webhook_url.flatten();
    let before_bot = bot.clone();
    let validation_changes = agent_changes.clone();
    let mut agent = before_agent.clone();
    let validation_agent = before_agent.clone();
    Ok(async {
        let (bot, after_url) = c.app().db.write(move |tx| {
        let agent = validation_agent;
        if let Some(agent) = &agent { agent.validate_changes(tx.conn(), validation_changes)?.into_result()?; }
        if webhook_submitted {
            bot.update_bot(tx, changes, webhook_url.as_deref())?;
        } else {
            bot.update(tx, changes)?;
        }
        let after_url = bot.webhook_url(tx.conn())?;
        attachments::assign(tx, Record::user(bot.id), "avatar", avatar)?;
        Ok((bot, after_url))
        }).await?;
        if let Some(mut updated) = agent.take() {
            agent = Some(c.app().db.write(move |tx| {
                // Rails save! follows the already committed update_bot. A late
                // save failure is not the earlier invalid? form response.
                updated.update(tx, dirty_agent).map_err(|error| match error {
                    campfire_db::Error::RecordInvalid(_) => campfire_db::Error::Other(error.to_string()),
                    error => error,
                })?;
                Ok(updated)
            }).await?);
        }
        let target = audit_target(&bot, agent.as_ref());
        if previous_url != after_url {
            let audit = NewAuditLog { action: "agent.webhook_url.change".into(), target: Some(target.clone()),
                changes: Some(json!({"webhook_url":audit_log::pair(json!(audit_log::webhook_origin_summary(previous_url.as_deref())?),json!(audit_log::webhook_origin_summary(after_url.as_deref())?))})), ..Default::default() };
            let context = context.clone();
            c.app().db.write(move |tx| AuditLog::record(tx, audit, &context)).await?;
        }
        let mut pairs = serde_json::Map::new();
        if before_bot.name != bot.name { pairs.insert("name".into(), audit_log::pair(json!(before_bot.name), json!(bot.name))); }
        if before_bot.icon_name != bot.icon_name { pairs.insert("icon_name".into(), audit_log::pair(json!(before_bot.icon_name), json!(bot.icon_name))); }
        if let (Some(before), Some(after)) = (&before_agent, &agent) {
            macro_rules! changed { ($($field:ident),*) => {$(if agent_field_submitted(&audit_changes, stringify!($field)) && before.$field != after.$field { pairs.insert(stringify!($field).into(), audit_log::pair(json!(before.$field), json!(after.$field))); })*}; }
            changed!(provider, runtime, description, daily_message_cap, daily_board_post_cap, daily_external_action_cap);
        }
        if !pairs.is_empty() {
            let audit = NewAuditLog { action: "agent.update".into(), target: Some(target), changes: Some(Value::Object(pairs)), ..Default::default() };
            c.app().db.write(move |tx| AuditLog::record(tx, audit, &context)).await?;
        }
        Ok::<_, campfire_db::Error>(())
    }.await)
}

/// Casting stays with WS11; the controller only tracks Rails' dirty attributes.
fn dirty_agent_changes(before: &Agent, mut changes: AgentChanges) -> AgentChanges {
    macro_rules! string { ($($field:ident),*) => {$(
        if changes.$field.as_ref() == Some(&before.$field) { changes.$field = None; }
    )*}; }
    string!(provider, runtime, description);
    macro_rules! cap {
        ($field:ident, $raw:ident) => {
            if changes
                .budget_cap_input(stringify!($field))
                .is_some_and(|input| input.value == json!(before.$field))
            {
                changes.$field = None;
                changes.$raw = None;
            }
        };
    }
    cap!(daily_message_cap, daily_message_cap_before_type_cast);
    cap!(daily_board_post_cap, daily_board_post_cap_before_type_cast);
    cap!(
        daily_external_action_cap,
        daily_external_action_cap_before_type_cast
    );
    changes
}
fn agent_field_submitted(changes: &AgentChanges, field: &str) -> bool {
    match field {
        "provider" => changes.provider.is_some(),
        "runtime" => changes.runtime.is_some(),
        "description" => changes.description.is_some(),
        field => changes.budget_cap_input(field).is_some(),
    }
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before(c).await?;
    let bot = set_bot(c).await?;
    remove_bot(c, bot).await?;
    redirect_to_bots(c)
}

/// `destroy`'s writes once the gate passed: the bot deactivated, then its agent's suspension
/// audited.
pub async fn remove_bot(c: &Ctx, bot: User) -> Result<()> {
    let mut bot = bot;
    let context = audit_context(c)?;
    let deactivation_context = context.clone();
    let audit = c
        .app()
        .db
        .write(move |tx| {
            let agent = Agent::for_user(tx.conn(), bot.id)?;
            bot.deactivate_with_audit(tx, &deactivation_context)?;
            Ok(NewAuditLog {
                action: "agent.suspend".into(),
                target: Some(audit_target(&bot, agent.as_ref())),
                changes: Some(json!({"by":"bot removed"})),
                ..Default::default()
            })
        })
        .await
        .map_err(Error::internal)?;
    c.app()
        .db
        .write(move |tx| AuditLog::record(tx, audit, &context))
        .await
        .map_err(Error::internal)?;
    Ok(())
}

/// Rails permits administrators and the owner without a sudo prompt.
pub async fn kill_switch(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = set_bot(c).await?;
    ensure_can_manage_bot(c, &bot).await?;
    let id = bot.id;
    let Some(cancelled) = suspend_agent(c, &bot).await? else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    let plural = if cancelled == 1 { "" } else { "s" };
    c.flash().set_notice(format!(
        "Agent suspended; {cancelled} approval{plural} cancelled."
    ));
    c.redirect_to(&c.url_for(&campfire_routes::edit_account_bot(id)))
}

/// `kill_switch`'s write once the gate passed: the agent suspended and its pending approvals
/// cancelled (how many), or `None` for a bot without an agent.
pub async fn suspend_agent(c: &Ctx, bot: &User) -> Result<Option<usize>> {
    let id = bot.id;
    let agent = c
        .app()
        .db
        .read(move |conn| Agent::for_user(conn, id))
        .await
        .map_err(Error::internal)?;
    let Some(agent) = agent else {
        return Ok(None);
    };
    let context = audit_context(c)?;
    let cancelled = c
        .app()
        .db
        .write(move |tx| campfire_db::models::agent_lifecycle::kill_switch(tx, agent.id, &context))
        .await
        .map_err(Error::internal)?;
    Ok(Some(cancelled))
}

pub async fn ensure_can_manage_bot(c: &mut Ctx, bot: &User) -> Result<()> {
    if can_manage_bot(c, bot).await? {
        Ok(())
    } else {
        campfire_kit::halt(c.head(StatusCode::FORBIDDEN))
    }
}

/// Whether the viewer may manage `bot`: an administrator, or the owner of its agent.
pub async fn can_manage_bot(c: &Ctx, bot: &User) -> Result<bool> {
    let viewer = concerns::require_current_user(c)?;
    if viewer.is_administrator() {
        return Ok(true);
    }
    let (bot_id, viewer_id) = (bot.id, viewer.id);
    c.app()
        .db
        .read(move |conn| {
            Ok(Agent::for_user(conn, bot_id)?
                .is_some_and(|agent| agent.owner_id == Some(viewer_id)))
        })
        .await
        .map_err(Error::internal)
}

pub fn audit_context(c: &Ctx) -> Result<Context> {
    Ok(Context {
        actor: Some(concerns::require_current_user(c)?.into()),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_owned),
    })
}

pub fn audit_target(bot: &User, agent: Option<&Agent>) -> Target {
    agent
        .map(|agent| Target {
            record_type: "Agent".into(),
            id: agent.id,
            label: Some(format!("Agent {}", bot.name)),
        })
        .unwrap_or_else(|| Target::from(bot))
}

pub async fn edit_form(c: &Ctx, bot: &User) -> Result<accounts::BotForm> {
    let viewer_id = concerns::require_current_user(c)?.id;
    let zone = c
        .app()
        .db
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT time_zone FROM users WHERE id=?",
                [viewer_id],
                |row| row.get::<_, Option<String>>(0),
            )?)
        })
        .await
        .map_err(Error::internal)?;
    let zone = campfire_views::time::Zone::for_user(zone.as_deref());
    // `usable?` may mark an unreadable linked token disconnected. The owner
    // service performs that write outside the view reader and never returns a token.
    let id = bot.id;
    let account = c
        .app()
        .db
        .read(move |conn| crate::integrations::github::accounts::Account::for_user(conn, id))
        .await
        .map_err(Error::internal)?;
    let github_usable = if let Some(account) = account {
        c.app()
            .github_accounts
            .usable(account.id)
            .await
            .map_err(Error::internal)?
    } else {
        false
    };
    let (app, base_url, bot) = (c.app().clone(), c.url_for(""), bot.clone());
    c.app()
        .db
        .read(move |conn| {
            presenters::accounts::bot_form(conn, &app, &base_url, &bot, &zone, github_usable)
        })
        .await
        .map_err(Error::internal)
}

/// Preserve strong-parameter scalars for WS11's before-type-cast validation.
fn agent_params(c: &Ctx) -> AgentChanges {
    let params = c
        .params
        .get("agent")
        .map(|p| {
            p.permit(&permit_keys(&[
                "provider",
                "runtime",
                "description",
                "daily_message_cap",
                "daily_board_post_cap",
                "daily_external_action_cap",
            ]))
        })
        .unwrap_or_default();
    let string = |key| {
        params
            .get(key)
            .map(|p| if p.is_null() { None } else { p.to_s() })
    };
    let cap = |key| params.get(key).map(Param::to_json);
    AgentChanges {
        provider: string("provider"),
        runtime: string("runtime"),
        description: string("description"),
        daily_message_cap_before_type_cast: cap("daily_message_cap"),
        daily_board_post_cap_before_type_cast: cap("daily_board_post_cap"),
        daily_external_action_cap_before_type_cast: cap("daily_external_action_cap"),
        ..Default::default()
    }
}

fn apply_agent_form(form: &mut accounts::BotAgentForm, changes: &AgentChanges) {
    macro_rules! assign { ($($field:ident),*) => {$(if let Some(value) = &changes.$field { form.$field = value.clone(); })*}; }
    assign!(provider, runtime, description);
    for (field, noun) in [
        ("daily_message_cap", "messages"),
        ("daily_board_post_cap", "board_posts"),
        ("daily_external_action_cap", "external_actions"),
    ] {
        if let Some(input) = changes.budget_cap_input(field) {
            let raw = match input.before_type_cast {
                Value::Null => None,
                Value::String(value) => Some(value),
                value => Some(value.to_string()),
            };
            form.raw_caps.insert(noun.into(), raw);
        }
    }
}

/// ApplicationController's chain, then `ensure_can_administer`.
async fn before(c: &mut Ctx) -> Result<()> {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)
}

/// `User.active_bots.find(params[:id])`
async fn set_bot(c: &Ctx) -> Result<User> {
    find_active_bot(c, "id").await
}

pub async fn find_active_bot(c: &Ctx, key: &str) -> Result<User> {
    let id = c
        .param_str(key)
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    c.app()
        .db
        .read(move |conn| match User::find_active_bot(conn, id) {
            Ok(bot) => Ok(Some(bot)),
            Err(campfire_db::Error::RecordNotFound(_)) => Ok(None),
            Err(error) => Err(error),
        })
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)
}

/// `params.require(:user).permit(:name, :avatar, :icon_name, :webhook_url)`.
fn bot_params(c: &Ctx) -> Result<ParamMap> {
    Ok(c.params.require("user")?.permit(&permit_keys(&[
        "name",
        "avatar",
        "icon_name",
        "webhook_url",
    ])))
}

/// String casting and normalization belong to the User owner; absent differs from nil.
fn icon_attribute(params: &ParamMap) -> Option<Option<String>> {
    params
        .get("icon_name")
        .map(|input| campfire_db::models::user::icon::normalize_input(&input.to_json()))
}

fn redirect_to_bots(c: &mut Ctx) -> Result {
    let location = c.url_for(&campfire_routes::account_bots());
    c.redirect_to(&location)
}

/// Rails converts a legacy bot only after the caller has authorized this management visit.
pub async fn ensure_agent(c: &Ctx, bot: &User) -> Result<Agent> {
    let (bot, owner_id, context) = (
        bot.clone(),
        concerns::require_current_user(c)?.id,
        audit_context(c)?,
    );
    let (agent, audit) = c
        .app()
        .db
        .write(move |tx| {
            if let Some(agent) = Agent::for_user(tx.conn(), bot.id)? {
                return Ok((agent, None));
            }
            let agent = Agent::create(
                tx,
                NewAgent {
                    user_id: bot.id,
                    owner_id: Some(owner_id),
                    kind: AgentKind::Workspace,
                    ..Default::default()
                },
            )?;
            let audit = NewAuditLog {
                action: "agent.create".into(),
                target: Some(audit_target(&bot, Some(&agent))),
                changes: Some(json!({"name":bot.name,"kind":"workspace"})),
                ..Default::default()
            };
            Ok((agent, Some(audit)))
        })
        .await
        .map_err(Error::internal)?;
    if let Some(audit) = audit {
        c.app()
            .db
            .write(move |tx| AuditLog::record(tx, audit, &context))
            .await
            .map_err(Error::internal)?;
    }
    Ok(agent)
}
pub fn error_sentence(errors: &campfire_db::Errors) -> String {
    campfire_views::helpers::to_sentence(&errors.full_messages(), " and ")
}

pub async fn viewer_zone(c: &Ctx) -> Result<campfire_views::time::Zone> {
    let id = concerns::require_current_user(c)?.id;
    let name = c
        .app()
        .db
        .read(move |conn| {
            Ok(
                conn.query_row("SELECT time_zone FROM users WHERE id=?", [id], |r| {
                    r.get::<_, Option<String>>(0)
                })?,
            )
        })
        .await
        .map_err(Error::internal)?;
    Ok(campfire_views::time::Zone::for_user(name.as_deref()))
}
