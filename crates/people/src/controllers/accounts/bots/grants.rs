//! Accounts::Bots::GrantsController. Validation and revocation remain WS11 domain calls.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
};
use campfire_db::models::audit_log::{AuditLog, NewAuditLog, Target};
use campfire_db::{AgentGrant, NewGrant, User};
use campfire_kit::{Ctx, Error, Param, Result, StatusCode, permit_keys};

pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    super::ensure_can_manage_bot(c, &bot).await?;
    concerns::ensure_can_administer(c)?;
    concerns::sudo::require_sudo_mode(c).await?;
    let agent = super::ensure_agent(c, &bot).await?;
    let params = c
        .params
        .require("agent_grant")?
        .permit(&permit_keys(&["capability", "room_id"]));
    let capability = params
        .get("capability")
        .and_then(Param::to_s)
        .unwrap_or_default();
    let room = params
        .get("room_id")
        .filter(|p| p.is_present())
        .and_then(Param::to_s);
    let room_id = room
        .as_deref()
        .map(|s| crate::concerns::cast_integer(s).unwrap_or(0));
    let result = grant(c, &bot, agent.id, capability, room_id).await?;
    match result {
        Ok(()) => redirect(c, bot.id),
        Err(campfire_db::Error::RecordInvalid(_errors)) => {
            Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY))
        }
        Err(error) if error.is_record_not_unique() => redirect(c, bot.id),
        Err(error) => Err(Error::internal(error)),
    }
}
/// `create`'s writes once the gates passed and the agent exists: the grant, unless an active
/// one is already there, then the audit. The inner result is the save's (`RecordInvalid` for
/// the form; a uniqueness race counts as done, as the classic rescue does).
pub async fn grant(
    c: &Ctx,
    bot: &User,
    agent_id: i64,
    capability: String,
    room_id: Option<i64>,
) -> Result<campfire_db::Result<()>> {
    let actor = concerns::require_current_user(c)?.id;
    let context = super::audit_context(c)?;
    let bot_name = bot.name.clone();
    Ok(async {
        let audit = c
            .app()
            .db
            .write(move |tx| {
                use rusqlite::OptionalExtension;
                let existing: Option<i64> = tx.conn().query_row(
                    "SELECT id FROM agent_grants WHERE agent_id=? AND capability=? AND room_id IS ? AND revoked_at IS NULL LIMIT 1",
                    rusqlite::params![agent_id, capability, room_id], |r| r.get(0)
                ).optional()?;
                if existing.is_none() {
                    let grant = AgentGrant::create(tx, NewGrant {
                        agent_id, capability, room_id, granted_by_id: actor, ..Default::default()
                    })?;
                    let room = grant.room_id.map(|id| campfire_db::Room::find_by_id(tx.conn(), id)).transpose()?.flatten().and_then(|r| r.name);
                    return Ok(Some(NewAuditLog {
                        action: "agent.grant.create".into(),
                        target: Some(target(grant.id, &grant.capability, &bot_name)),
                        changes: Some(serde_json::json!({"capability":grant.capability,"room":room})),
                        ..Default::default()
                    }));
                }
                Ok(None)
            })
            .await?;
        // The find/create transaction commits before Rails writes its audit.
        // The action's uniqueness rescue still covers both independent writes.
        if let Some(audit) = audit {
            c.app().db.write(move |tx| AuditLog::record(tx, audit, &context)).await?;
        }
        Ok::<_, campfire_db::Error>(())
    }.await)
}

pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    super::ensure_can_manage_bot(c, &bot).await?;
    concerns::sudo::require_sudo_mode(c).await?;
    let agent = super::ensure_agent(c, &bot).await?;
    let id = c
        .param_str("id")
        .and_then(crate::concerns::cast_integer)
        .ok_or(Error::NotFound)?;
    if !revoke(c, &bot, agent.id, id).await? {
        return Err(Error::NotFound);
    }
    redirect(c, bot.id)
}

/// `destroy`'s writes once the gates passed and the agent exists: grant `id` revoked (once),
/// then the audit. `false` when the agent has no such grant.
pub async fn revoke(c: &Ctx, bot: &User, agent_id: i64, id: i64) -> Result<bool> {
    let context = super::audit_context(c)?;
    let bot_name = bot.name.clone();
    let audit = c
        .app()
        .db
        .write(move |tx| {
            let Some(mut grant) =
                AgentGrant::find(tx.conn(), id)?.filter(|g| g.agent_id == agent_id)
            else {
                return Ok(None);
            };
            if grant.revoked_at.is_none() {
                grant.revoke(tx)?;
                let room = grant
                    .room_id
                    .map(|id| campfire_db::Room::find_by_id(tx.conn(), id))
                    .transpose()?
                    .flatten()
                    .and_then(|r| r.name);
                return Ok(Some(Some(NewAuditLog {
                    action: "agent.grant.revoke".into(),
                    target: Some(target(grant.id, &grant.capability, &bot_name)),
                    changes: Some(serde_json::json!({"capability":grant.capability,"room":room})),
                    ..Default::default()
                })));
            }
            Ok(Some(None))
        })
        .await
        .map_err(Error::internal)?;
    let Some(audit) = audit else {
        return Ok(false);
    };
    if let Some(audit) = audit {
        c.app()
            .db
            .write(move |tx| AuditLog::record(tx, audit, &context))
            .await
            .map_err(Error::internal)?;
    }
    Ok(true)
}
fn target(id: i64, capability: &str, bot: &str) -> Target {
    Target {
        record_type: "AgentGrant".into(),
        id,
        label: Some(format!("{capability} grant ({bot})")),
    }
}
fn redirect(c: &mut Ctx, bot_id: i64) -> Result {
    c.redirect_to(&c.url_for(&campfire_routes::account_bot_grants(bot_id)))
}
