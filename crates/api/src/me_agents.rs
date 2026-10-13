//! Session-authenticated management of the current member's personal agents.
use crate::{
    admin::{body, require_sudo, viewer},
    dto::{UserExtras, time},
    error::{fail, validation},
};
use axum::{
    Router,
    routing::{delete, get, post, put},
};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::models::{
    agent_access,
    audit_log::{self, AuditLog, Context, NewAuditLog, Target},
    personal_agents,
};
use campfire_db::{
    Agent, AgentChanges, AgentCredential, Connection, NewUser, Timestamp, Tx, User, UserChanges,
};
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode, action, unparsed_action};
use serde_json::json;

pub fn routes() -> Router<Kit> {
    Router::new()
        .route(
            "/api/v1/me/agents",
            get(action(index)).post(unparsed_action(create)),
        )
        .route(
            "/api/v1/me/agents/{id}",
            get(action(show))
                .patch(unparsed_action(update))
                .delete(action(remove)),
        )
        .route(
            "/api/v1/me/agents/{id}/access",
            put(unparsed_action(access)),
        )
        .route(
            "/api/v1/me/agents/{id}/credentials",
            get(action(credentials)).post(unparsed_action(issue)),
        )
        .route(
            "/api/v1/me/agents/{id}/credentials/{cid}/rotate",
            post(unparsed_action(rotate)),
        )
        .route(
            "/api/v1/me/agents/{id}/credentials/{cid}",
            delete(action(revoke)),
        )
}
endpoint!(index => list_agents);
endpoint!(create => create_agent);
endpoint!(show => show_agent);
endpoint!(update => update_agent);
endpoint!(remove => remove_agent);
endpoint!(access => update_access);
endpoint!(credentials => list_credentials);
endpoint!(issue => issue_credential);
endpoint!(rotate => rotate_credential);
endpoint!(revoke => revoke_credential);

pub(crate) async fn member(c: &mut Ctx) -> Result<User> {
    let user = viewer(c).await?;
    if !user.is_active() || user.is_bot() {
        return Err(Error::Halt(Box::new(campfire_kit::Response::new(
            StatusCode::FORBIDDEN,
        ))));
    }
    Ok(user)
}
fn id(c: &Ctx, key: &str) -> Result<i64> {
    c.param_str(key)
        .and_then(|s| s.parse().ok())
        .filter(|id| *id > 0)
        .ok_or(Error::NotFound)
}
async fn owned(c: &Ctx, owner_id: i64) -> Result<Agent> {
    let id = id(c, "id")?;
    c.app()
        .db
        .read(move |conn| personal_agents::owned(conn, owner_id, id))
        .await
        .map_err(saved_error)
}
const DISABLED: &str = "MemberAgentsDisabled";
const DISABLED_MESSAGE: &str = "Personal agents are turned off for this workspace.";
pub(crate) const LIMIT: &str = "MemberAgentLimitReached";
const LIMIT_MESSAGE: &str = "personal agent limit reached";
pub(crate) fn enabled(conn: &Connection) -> campfire_db::Result<()> {
    if !campfire_db::Account::first(conn)?
        .ok_or(campfire_db::Error::RecordNotFound("Account"))?
        .settings()
        .member_agents_enabled()
    {
        return Err(campfire_db::Error::Other(DISABLED.into()));
    }
    Ok(())
}
pub(crate) fn saved_error(error: campfire_db::Error) -> Error {
    match &error {
        campfire_db::Error::Other(message) if message == DISABLED => refuse(api::ApiError::Forbidden {
            message: DISABLED_MESSAGE.into(),
        }),
        campfire_db::Error::Other(message) if message == LIMIT => refuse(api::ApiError::Validation {
            message: "Personal agent limit reached".into(),
            fields: [("base".to_string(), vec![LIMIT_MESSAGE.to_string()])].into(),
        }),
        _ => Error::internal(error),
    }
}
/// A refusal found inside a database closure, as the API's usual error body.
fn refuse(error: api::ApiError) -> Error {
    let status = StatusCode::from_u16(error.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    match serde_json::to_vec(&api::ApiErrorResponse { error }) {
        Ok(body) => Error::Halt(Box::new(campfire_kit::Response::with_body(
            status,
            campfire_kit::response::JSON_UTF8,
            body,
        ))),
        Err(error) => Error::internal(error),
    }
}
fn credential(row: AgentCredential, now: Timestamp) -> api::PersonalAgentCredential {
    let state = if row.revoked_at.is_some() {
        api::CredentialState::Revoked
    } else if !row.active(now) {
        api::CredentialState::Expired
    } else {
        api::CredentialState::Active
    };
    api::PersonalAgentCredential {
        id: row.id,
        kind: api::PersonalCredentialKind::Token,
        name: row.name,
        last_four: row.token_last_four,
        created_at: time(row.created_at),
        expires_at: row.expires_at.map(time),
        last_used_at: row.last_used_at.map(time),
        last_used_ip: row.last_used_ip,
        state,
    }
}
pub(crate) fn present(
    conn: &Connection,
    agent: Agent,
    now: Timestamp,
) -> campfire_db::Result<api::PersonalAgent> {
    let bot = User::find(conn, agent.user_id)?;
    let icon_url = UserExtras::load(conn, [&bot])?
        .avatar_icon(bot.id)
        .and_then(|icon| icon.image_url.clone());
    let credentials = AgentCredential::for_agent(conn, agent.id)?
        .into_iter()
        .map(|r| credential(r, now))
        .collect::<Vec<_>>();
    let grants = campfire_db::AgentGrant::effective_for_agent(conn, agent.id)?;
    let grantable =
        personal_agents::grantable_rooms(conn, agent.owner_id.expect("personal owner"))?;
    let room_ids = campfire_db::Membership::for_user(conn, bot.id)?
        .into_iter()
        .map(|m| m.room_id)
        .filter(|id| grantable.iter().any(|room| room.id == *id))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let capabilities = grants
        .into_iter()
        .map(|g| g.capability)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(api::PersonalAgent {
        id: agent.id,
        user_id: bot.id,
        name: bot.name,
        icon_name: bot.icon_name,
        icon_url,
        provider: agent.provider,
        runtime: agent.runtime,
        description: agent.description,
        daily_message_cap: agent.daily_message_cap,
        suspended: agent.suspended_at.is_some(),
        room_ids,
        capabilities,
        last_used_at: credentials
            .iter()
            .filter_map(|c| c.last_used_at.clone())
            .max(),
        created_at: time(agent.created_at),
        credentials,
    })
}
pub(crate) fn audit(
    tx: &Tx<'_>,
    agent: &Agent,
    action: &str,
    changes: serde_json::Value,
    context: &Context,
) -> campfire_db::Result<()> {
    AuditLog::record(
        tx,
        NewAuditLog {
            action: action.into(),
            target: Some(Target {
                record_type: "Agent".into(),
                id: agent.id,
                label: Some(User::find(tx.conn(), agent.user_id)?.name),
            }),
            changes: Some(changes),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}
fn expiry(c: &mut Ctx, value: Option<&str>) -> Result<Option<Timestamp>> {
    let now = Timestamp::from_jiff(c.now());
    match value {
        None => Ok(Some(now.since(jiff::SignedDuration::from_hours(90 * 24)))),
        Some("never") => Ok(None),
        Some(value) => value
            .parse::<jiff::Timestamp>()
            .ok()
            .map(Timestamp::from_jiff)
            .filter(|at| *at > now)
            .map(Some)
            .ok_or_else(|| {
                fail(
                    c,
                    validation("expiresAt", "must be a future timestamp or never"),
                )
            }),
    }
}
async fn list_agents(c: &mut Ctx) -> Result {
    let owner = member(c).await?;
    let now = Timestamp::from_jiff(c.now());
    let mcp_url = c
        .app()
        .config
        .mail
        .validated_app_url()
        .map(|origin| format!("{origin}/agents/mcp"));
    let oauth_available = c.app().config.oauth_issuer().is_some();
    let list = c
        .app()
        .db
        .read(move |conn| {
            let settings = campfire_db::Account::first(conn)?
                .ok_or(campfire_db::Error::RecordNotFound("Account"))?
                .settings();
            Ok(api::PersonalAgentList {
                agents: personal_agents::for_owner(conn, owner.id)?
                    .into_iter()
                    .map(|a| present(conn, a, now))
                    .collect::<campfire_db::Result<_>>()?,
                enabled: settings.member_agents_enabled(),
                limit: (!owner.is_administrator()).then(|| settings.member_agent_limit()),
                capabilities: agent_access::PERSONAL_CAPABILITIES
                    .map(str::to_string)
                    .to_vec(),
                rooms: personal_agents::grantable_rooms(conn, owner.id)?
                    .into_iter()
                    .map(|room| api::PersonalAgentRoom {
                        id: room.id,
                        name: room.name,
                        private: room.private,
                    })
                    .collect(),
                mcp_url,
                oauth_available,
            })
        })
        .await
        .map_err(saved_error)?;
    c.json(StatusCode::OK, &list)
}
async fn show_agent(c: &mut Ctx) -> Result {
    let owner = member(c).await?;
    let agent = owned(c, owner.id).await?;
    reply_agent(c, agent.id, owner.id).await
}
async fn reply_agent(c: &mut Ctx, id: i64, owner_id: i64) -> Result {
    let now = Timestamp::from_jiff(c.now());
    let agent = c
        .app()
        .db
        .read(move |conn| present(conn, personal_agents::owned(conn, owner_id, id)?, now))
        .await
        .map_err(saved_error)?;
    c.json(StatusCode::OK, &agent)
}
async fn create_agent(c: &mut Ctx) -> Result {
    let owner = member(c).await?;
    require_sudo(c).await?;
    let input: api::CreatePersonalAgent = body(c).await?;
    let expires_at = expiry(c, input.expires_at.as_deref())?;
    let context = campfire_people::controllers::two_factor::audit_context(c)?;
    let response = c
        .app()
        .db
        .write(move |tx| {
            enabled(tx.conn())?;
            let owner = User::find(tx.conn(), owner.id)?;
            let settings = campfire_db::Account::first_in(tx)?.settings();
            if !owner.is_administrator()
                && personal_agents::for_owner(tx.conn(), owner.id)?.len() as i64
                    >= settings.member_agent_limit()
            {
                return Err(campfire_db::Error::Other(LIMIT.into()));
            }
            let mut agent = personal_agents::create(
                tx,
                NewUser {
                    name: input.name,
                    icon_name: input.icon_name,
                    ..Default::default()
                },
                owner.id,
            )?;
            agent.update(
                tx,
                AgentChanges {
                    provider: input.provider.map(Some),
                    runtime: input.runtime.map(Some),
                    ..Default::default()
                },
            )?;
            personal_agents::replace_access(
                tx,
                &agent,
                owner.id,
                &input.room_ids,
                &input.capabilities,
            )?;
            let (row, secret) = AgentCredential::create_personal_with_secret(
                tx,
                agent.id,
                input.token_name.as_deref().unwrap_or("Primary token"),
                owner.id,
                expires_at,
            )?;
            audit(
                tx,
                &agent,
                "agent.create",
                json!({"owner_id":owner.id}),
                &context,
            )?;
            audit(
                tx,
                &agent,
                "agent.access.update",
                json!({"room_ids":input.room_ids,"capabilities":input.capabilities}),
                &context,
            )?;
            audit(
                tx,
                &agent,
                "agent.credential.issue",
                json!({"id":row.id,"name":row.name,"expires_at":row.expires_at.map(time)}),
                &context,
            )?;
            Ok(api::PersonalAgentCreated {
                agent: present(tx.conn(), agent, tx.now())?,
                secret,
            })
        })
        .await
        .map_err(saved_error)?;
    c.json(StatusCode::CREATED, &response)
}
async fn update_agent(c: &mut Ctx) -> Result {
    let owner = member(c).await?;
    let agent = owned(c, owner.id).await?;
    let input: api::UpdatePersonalAgent = body(c).await?;
    let agent_id = agent.id;
    let context = campfire_people::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let mut agent = personal_agents::owned(tx.conn(), owner.id, agent_id)?;
            let before_agent = agent.clone();
            let mut bot = User::find(tx.conn(), agent.user_id)?;
            let before_bot = bot.clone();
            bot.update(
                tx,
                UserChanges {
                    name: input.name,
                    icon_name: input.icon_name.map(Some),
                    ..Default::default()
                },
            )?;
            agent.update(
                tx,
                AgentChanges {
                    provider: input.provider.map(Some),
                    runtime: input.runtime.map(Some),
                    description: input.description.map(Some),
                    daily_message_cap: input.daily_message_cap,
                    ..Default::default()
                },
            )?;
            let mut changes = serde_json::Map::new();
            for (field, before, after) in [
                ("name", json!(before_bot.name), json!(bot.name)),
                (
                    "icon_name",
                    json!(before_bot.icon_name),
                    json!(bot.icon_name),
                ),
                (
                    "provider",
                    json!(before_agent.provider),
                    json!(agent.provider),
                ),
                ("runtime", json!(before_agent.runtime), json!(agent.runtime)),
                (
                    "description",
                    json!(before_agent.description),
                    json!(agent.description),
                ),
                (
                    "daily_message_cap",
                    json!(before_agent.daily_message_cap),
                    json!(agent.daily_message_cap),
                ),
            ] {
                if before != after {
                    changes.insert(field.into(), audit_log::pair(before, after));
                }
            }
            if !changes.is_empty() {
                audit(
                    tx,
                    &agent,
                    "agent.update",
                    serde_json::Value::Object(changes),
                    &context,
                )?;
            }
            Ok(())
        })
        .await
        .map_err(saved_error)?;
    reply_agent(c, agent_id, owner.id).await
}
async fn remove_agent(c: &mut Ctx) -> Result {
    let owner = member(c).await?;
    let agent = owned(c, owner.id).await?;
    let bot = c
        .app()
        .db
        .read(move |conn| User::find(conn, agent.user_id))
        .await
        .map_err(saved_error)?;
    let id = bot.id;
    campfire_people::controllers::accounts::bots::remove_bot(c, bot).await?;
    c.json(StatusCode::OK, &api::BotRemoved { id })
}
async fn update_access(c: &mut Ctx) -> Result {
    let owner = member(c).await?;
    let agent = owned(c, owner.id).await?;
    let input: api::PersonalAgentAccess = body(c).await?;
    let context = campfire_people::controllers::two_factor::audit_context(c)?;
    let agent_id = agent.id;
    c.app()
        .db
        .write(move |tx| {
            let agent = personal_agents::owned(tx.conn(), owner.id, agent_id)?;
            enabled(tx.conn())?;
            personal_agents::replace_access(
                tx,
                &agent,
                owner.id,
                &input.room_ids,
                &input.capabilities,
            )?;
            audit(
                tx,
                &agent,
                "agent.access.update",
                json!({"room_ids":input.room_ids,"capabilities":input.capabilities}),
                &context,
            )
        })
        .await
        .map_err(saved_error)?;
    reply_agent(c, agent_id, owner.id).await
}
async fn list_credentials(c: &mut Ctx) -> Result {
    let owner = member(c).await?;
    let agent = owned(c, owner.id).await?;
    reply_credentials(c, agent.id, owner.id).await
}
async fn reply_credentials(c: &mut Ctx, agent_id: i64, owner_id: i64) -> Result {
    let now = Timestamp::from_jiff(c.now());
    let list = c
        .app()
        .db
        .read(move |conn| {
            personal_agents::owned(conn, owner_id, agent_id)?;
            Ok(api::PersonalCredentialList {
                credentials: AgentCredential::for_agent(conn, agent_id)?
                    .into_iter()
                    .map(|row| credential(row, now))
                    .collect(),
            })
        })
        .await
        .map_err(saved_error)?;
    c.json(StatusCode::OK, &list)
}
fn find_credential(
    conn: &Connection,
    agent_id: i64,
    id: i64,
) -> campfire_db::Result<AgentCredential> {
    AgentCredential::find(conn, id)?
        .filter(|row| row.agent_id == agent_id)
        .ok_or(campfire_db::Error::RecordNotFound("AgentCredential"))
}
async fn issue_credential(c: &mut Ctx) -> Result {
    let owner = member(c).await?;
    let agent = owned(c, owner.id).await?;
    require_sudo(c).await?;
    let input: api::IssuePersonalCredential = body(c).await?;
    let expires_at = expiry(c, input.expires_at.as_deref())?;
    let context = campfire_people::controllers::two_factor::audit_context(c)?;
    let response = c
        .app()
        .db
        .write(move |tx| {
            let agent = personal_agents::owned(tx.conn(), owner.id, agent.id)?;
            enabled(tx.conn())?;
            let (row, secret) = AgentCredential::create_personal_with_secret(
                tx,
                agent.id,
                &input.name,
                owner.id,
                expires_at,
            )?;
            audit(
                tx,
                &agent,
                "agent.credential.issue",
                json!({"id":row.id,"name":row.name,"expires_at":row.expires_at.map(time)}),
                &context,
            )?;
            Ok(api::PersonalCredentialCreated {
                credential: credential(row, tx.now()),
                secret,
            })
        })
        .await
        .map_err(saved_error)?;
    c.json(StatusCode::CREATED, &response)
}
async fn rotate_credential(c: &mut Ctx) -> Result {
    let owner = member(c).await?;
    let agent = owned(c, owner.id).await?;
    let cid = id(c, "cid")?;
    c.app()
        .db
        .read(move |conn| find_credential(conn, agent.id, cid))
        .await
        .map_err(saved_error)?;
    require_sudo(c).await?;
    let input: api::RotatePersonalCredential = body(c).await?;
    let expires_at = expiry(c, input.expires_at.as_deref())?;
    let context = campfire_people::controllers::two_factor::audit_context(c)?;
    let response = c.app().db.write(move |tx| {
        let agent = personal_agents::owned(tx.conn(),owner.id,agent.id)?;
        enabled(tx.conn())?;
        let mut old = find_credential(tx.conn(),agent.id,cid)?;
        if old.revoked_at.is_some() {return Err(campfire_db::Error::RecordNotFound("AgentCredential"));}
        let (row,secret) = AgentCredential::create_personal_with_secret(tx,agent.id,input.name.as_deref().unwrap_or(&old.name),owner.id,expires_at)?;
        old.revoke(tx)?;
        audit(tx,&agent,"agent.credential.rotate",json!({"id":row.id,"previous_id":old.id,"name":row.name,"expires_at":row.expires_at.map(time)}),&context)?;
        Ok(api::PersonalCredentialCreated {credential:credential(row,tx.now()),secret})
    }).await.map_err(saved_error)?;
    c.json(StatusCode::CREATED, &response)
}
async fn revoke_credential(c: &mut Ctx) -> Result {
    let owner = member(c).await?;
    let agent = owned(c, owner.id).await?;
    let cid = id(c, "cid")?;
    let context = campfire_people::controllers::two_factor::audit_context(c)?;
    c.app()
        .db
        .write(move |tx| {
            let agent = personal_agents::owned(tx.conn(), owner.id, agent.id)?;
            let mut row = find_credential(tx.conn(), agent.id, cid)?;
            if row.revoked_at.is_none() {
                row.revoke(tx)?;
                audit(
                    tx,
                    &agent,
                    "agent.credential.revoke",
                    json!({"id":row.id,"name":row.name}),
                    &context,
                )?;
            }
            Ok(())
        })
        .await
        .map_err(saved_error)?;
    reply_credentials(c, agent.id, owner.id).await
}
