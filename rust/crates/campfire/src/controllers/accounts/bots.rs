//! `Accounts::BotsController` (reference/app/controllers/accounts/bots_controller.rb).

pub mod credentials;
pub mod grants;
pub mod keys;
pub mod webhook_secrets;

use campfire_db::models::audit_log::{self, AuditLog, Context, NewAuditLog, Target};
use campfire_db::{Agent, AgentChanges, AgentKind, NewAgent, User, UserChanges};
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
            User::active_bots_ordered(conn)?
                .iter()
                .map(|bot| presenters::accounts::bot(conn, &secrets, bot))
                .collect()
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
    concerns::require_sudo_mode(c)?;
    let params = bot_params(c)?;
    let name = params.get("name").and_then(Param::to_s).ok_or_else(|| {
        Error::internal(anyhow::anyhow!("NOT NULL constraint failed: users.name"))
    })?;
    let webhook_url = params.get("webhook_url").and_then(Param::to_s);
    let avatar = Assignment::from_params(&params, "avatar")?
        .stage(c.app())
        .await?;
    let owner_id = concerns::require_current_user(c)?.id;
    let context = audit_context(c)?;
    let result = c
        .app()
        .db
        .write(move |tx| {
            let bot = User::create_bot(tx, &name, webhook_url.as_deref())?;
            let agent = Agent::create(
                tx,
                NewAgent {
                    user_id: bot.id,
                    owner_id: Some(owner_id),
                    kind: AgentKind::Workspace,
                    ..Default::default()
                },
            )?;
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "agent.create".into(),
                    target: Some(audit_target(&bot, Some(&agent))),
                    changes: Some(json!({"name":bot.name,"kind":"workspace"})),
                    ..Default::default()
                },
                &context,
            )?;
            let pending = attachments::assign(tx, Record::user(bot.id), "avatar", avatar)?;
            Ok((bot, pending))
        })
        .await;
    match result {
        Ok((bot, pending)) => {
            attachments::analyze_later(c.app(), pending);
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
    let mut bot = set_bot(c).await?;
    ensure_can_manage_bot(c, &bot).await?;
    let previous_url = c
        .app()
        .db
        .read({
            let id = bot.id;
            move |conn| User::find(conn, id)?.webhook_url(conn)
        })
        .await
        .map_err(Error::internal)?;
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
    let webhook_changing = submitted_url.as_deref().is_some_and(|url| {
        campfire_richtext::ruby::strip(url) != previous_url.as_deref().unwrap_or("")
    });
    if webhook_changing {
        concerns::ensure_can_administer(c)?;
        concerns::require_sudo_mode(c)?;
    }
    let params = bot_params(c)?;
    let changes = UserChanges {
        name: params.get("name").and_then(Param::to_s),
        ..Default::default()
    };
    let agent_changes = agent_params(c);
    let avatar = Assignment::from_params(&params, "avatar")?
        .stage(c.app())
        .await?;
    let context = audit_context(c)?;
    // User::update_bot's existing seam uses None to remove. Supply the current URL
    // for an omitted/owner-disabled field until WS11 adds an explicit presence seam.
    let webhook_url = if concerns::require_current_user(c)?.is_administrator()
        && params.contains_key("webhook_url")
    {
        params.get("webhook_url").and_then(Param::to_s)
    } else {
        previous_url.clone()
    };
    let before_bot = bot.clone();
    let requested_agent = agent_changes.clone();
    let result = c.app().db.write(move |tx| {
        let mut agent = Agent::for_user(tx.conn(), bot.id)?;
        let before_agent = agent.clone();
        if let Some(agent) = &mut agent { agent.update(tx, agent_changes)?; }
        bot.update_bot(tx, changes, webhook_url.as_deref())?;
        let target = audit_target(&bot, agent.as_ref());
        let after_url = bot.webhook_url(tx.conn())?;
        if previous_url != after_url {
            AuditLog::record(tx, NewAuditLog { action: "agent.webhook_url.change".into(), target: Some(target.clone()),
                changes: Some(json!({"webhook_url":audit_log::pair(json!(audit_log::webhook_origin_summary(previous_url.as_deref())?),json!(audit_log::webhook_origin_summary(after_url.as_deref())?))})), ..Default::default() }, &context)?;
        }
        let mut pairs = serde_json::Map::new();
        if before_bot.name != bot.name { pairs.insert("name".into(), audit_log::pair(json!(before_bot.name), json!(bot.name))); }
        if let (Some(before), Some(after)) = (&before_agent, &agent) {
            macro_rules! changed { ($($field:ident),*) => {$(if before.$field != after.$field { pairs.insert(stringify!($field).into(), audit_log::pair(json!(before.$field), json!(after.$field))); })*}; }
            changed!(provider, runtime, description, daily_message_cap, daily_board_post_cap, daily_external_action_cap);
        }
        if !pairs.is_empty() {
            AuditLog::record(tx, NewAuditLog { action: "agent.update".into(), target: Some(target), changes: Some(Value::Object(pairs)), ..Default::default() }, &context)?;
        }
        attachments::assign(tx, Record::user(bot.id), "avatar", avatar)
    }).await;
    match result {
        Ok(pending) => {
            attachments::analyze_later(c.app(), pending);
            redirect_to_bots(c)
        }
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            let bot = set_bot(c).await?;
            let mut form = edit_form(c, &bot).await?;
            if let Some(agent) = &mut form.agent {
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

pub async fn destroy(c: &mut Ctx) -> Result {
    before(c).await?;
    let mut bot = set_bot(c).await?;
    let context = audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let agent = Agent::for_user(tx.conn(), bot.id)?;
            bot.deactivate_with_audit(tx, &context)?;
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "agent.suspend".into(),
                    target: Some(audit_target(&bot, agent.as_ref())),
                    changes: Some(json!({"by":"bot removed"})),
                    ..Default::default()
                },
                &context,
            )?;
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    redirect_to_bots(c)
}

/// Rails permits administrators and the owner without a sudo prompt.
pub async fn kill_switch(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = set_bot(c).await?;
    ensure_can_manage_bot(c, &bot).await?;
    let id = bot.id;
    let agent = c
        .app()
        .db
        .read(move |conn| Agent::for_user(conn, id))
        .await
        .map_err(Error::internal)?;
    let Some(agent) = agent else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    let context = audit_context(c)?;
    let cancelled = c
        .app()
        .db
        .write(move |tx| campfire_db::models::agent_lifecycle::kill_switch(tx, agent.id, &context))
        .await
        .map_err(Error::internal)?;
    let plural = if cancelled == 1 { "" } else { "s" };
    c.flash().set_notice(format!(
        "Agent suspended; {cancelled} approval{plural} cancelled."
    ));
    c.redirect_to(&c.url_for(&campfire_routes::edit_account_bot(id)))
}

pub(super) async fn ensure_can_manage_bot(c: &mut Ctx, bot: &User) -> Result<()> {
    let viewer = concerns::require_current_user(c)?;
    if viewer.is_administrator() {
        return Ok(());
    }
    let (bot_id, viewer_id) = (bot.id, viewer.id);
    let manages = c
        .app()
        .db
        .read(move |conn| {
            Ok(Agent::for_user(conn, bot_id)?
                .is_some_and(|agent| agent.owner_id == Some(viewer_id)))
        })
        .await
        .map_err(Error::internal)?;
    if manages {
        Ok(())
    } else {
        campfire_kit::halt(c.head(StatusCode::FORBIDDEN))
    }
}

pub(super) fn audit_context(c: &Ctx) -> Result<Context> {
    Ok(Context {
        actor: Some(concerns::require_current_user(c)?.into()),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_owned),
    })
}

pub(super) fn audit_target(bot: &User, agent: Option<&Agent>) -> Target {
    agent
        .map(|agent| Target {
            record_type: "Agent".into(),
            id: agent.id,
            label: Some(format!("Agent {}", bot.name)),
        })
        .unwrap_or_else(|| Target::from(bot))
}

async fn edit_form(c: &Ctx, bot: &User) -> Result<accounts::BotForm> {
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
    let (app, base_url, bot) = (c.app().clone(), c.url_for(""), bot.clone());
    c.app()
        .db
        .read(move |conn| presenters::accounts::bot_form(conn, &app, &base_url, &bot, &zone))
        .await
        .map_err(Error::internal)
}

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
    let cap = |key| {
        params.get(key).map(|p| {
            p.to_s()
                .filter(|s| !campfire_richtext::ruby::is_blank(s))
                .map(|s| cast_integer(&s).unwrap_or(0))
        })
    };
    AgentChanges {
        provider: string("provider"),
        runtime: string("runtime"),
        description: string("description"),
        daily_message_cap: cap("daily_message_cap"),
        daily_board_post_cap: cap("daily_board_post_cap"),
        daily_external_action_cap: cap("daily_external_action_cap"),
        ..Default::default()
    }
}

fn apply_agent_form(form: &mut accounts::BotAgentForm, changes: &AgentChanges) {
    macro_rules! assign { ($($field:ident),*) => {$(if let Some(value) = &changes.$field { form.$field = value.clone(); })*}; }
    assign!(provider, runtime, description);
    if let Some(value) = changes.daily_message_cap {
        form.daily_message_cap = value;
    }
    if let Some(value) = changes.daily_board_post_cap {
        form.daily_board_post_cap = value;
    }
    if let Some(value) = changes.daily_external_action_cap {
        form.daily_external_action_cap = value;
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

pub(crate) async fn find_active_bot(c: &Ctx, key: &str) -> Result<User> {
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

/// `params.require(:user).permit(:name, :avatar, :webhook_url)`
fn bot_params(c: &Ctx) -> Result<ParamMap> {
    Ok(c.params.require("user")?.permit(&permit_keys(&[
        "name",
        "avatar",
        "icon_name",
        "webhook_url",
    ])))
}

fn redirect_to_bots(c: &mut Ctx) -> Result {
    let location = c.url_for(&campfire_routes::account_bots());
    c.redirect_to(&location)
}

/// Rails converts a legacy bot only after the caller has authorized this management visit.
pub(super) async fn ensure_agent(c: &Ctx, bot: &User) -> Result<Agent> {
    let (bot, owner_id, context) = (
        bot.clone(),
        concerns::require_current_user(c)?.id,
        audit_context(c)?,
    );
    c.app()
        .db
        .write(move |tx| {
            if let Some(agent) = Agent::for_user(tx.conn(), bot.id)? {
                return Ok(agent);
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
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "agent.create".into(),
                    target: Some(audit_target(&bot, Some(&agent))),
                    changes: Some(json!({"name":bot.name,"kind":"workspace"})),
                    ..Default::default()
                },
                &context,
            )?;
            Ok(agent)
        })
        .await
        .map_err(Error::internal)
}
pub(super) fn parse_datetime(
    param: Option<&Param>,
    zone: &campfire_views::time::Zone,
) -> Option<campfire_db::Timestamp> {
    let value = param?.to_s()?;
    if let Ok(at) = value.parse::<jiff::Timestamp>() {
        return Some(campfire_db::Timestamp::from_jiff(at));
    }
    value
        .parse::<jiff::civil::DateTime>()
        // ActiveModel::Type::DateTime falls back to Date._parse. This named-
        // month form is in the pinned HTTP corpus; the wider grammar and raw
        // non-time/multiparameter values still need WS11's input seam.
        .or_else(|_| jiff::civil::DateTime::strptime("%d %b %Y %H:%M:%S", &value))
        .ok()?
        .to_zoned(zone.tz().clone())
        .ok()
        .map(|t| campfire_db::Timestamp::from_jiff(t.timestamp()))
}
pub(super) fn error_sentence(errors: &campfire_db::Errors) -> String {
    campfire_views::helpers::to_sentence(&errors.full_messages(), " and ")
}

pub(super) async fn viewer_zone(c: &Ctx) -> Result<campfire_views::time::Zone> {
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
