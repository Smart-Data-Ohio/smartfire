//! Accounts::Bots::GrantsController. Validation and revocation remain WS11 domain calls.
use crate::controllers::presenters::page::framed_page;
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    controllers::presenters,
};
use campfire_db::models::audit_log::{AuditLog, NewAuditLog, Target};
use campfire_db::{AgentGrant, NewGrant, User};
use campfire_kit::{Ctx, Error, Param, Result, StatusCode, format, permit_keys};
use campfire_views::accounts::bot_access::GrantForm;

pub async fn index(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    super::ensure_can_manage_bot(c, &bot).await?;
    let agent = super::ensure_agent(c, &bot).await?;
    render_index(c, &bot, agent.id, GrantForm::default(), StatusCode::OK).await
}
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    super::ensure_can_manage_bot(c, &bot).await?;
    concerns::ensure_can_administer(c)?;
    concerns::sudo::require_sudo_mode(c)?;
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
    let actor = concerns::require_current_user(c)?.id;
    let context = super::audit_context(c)?;
    let form_capability = capability.clone();
    let bot_name = bot.name.clone();
    let result = async {
        let audit = c
            .app()
            .db
            .write(move |tx| {
                use rusqlite::OptionalExtension;
                let existing: Option<i64> = tx.conn().query_row(
                    "SELECT id FROM agent_grants WHERE agent_id=? AND capability=? AND room_id IS ? AND revoked_at IS NULL LIMIT 1",
                    rusqlite::params![agent.id, capability, room_id], |r| r.get(0)
                ).optional()?;
                if existing.is_none() {
                    let grant = AgentGrant::create(tx, NewGrant {
                        agent_id: agent.id, capability, room_id, granted_by_id: actor, ..Default::default()
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
    }.await;
    match result {
        Ok(()) => redirect(c, bot.id),
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            render_index(
                c,
                &bot,
                agent.id,
                GrantForm {
                    capability: Some(form_capability),
                    room_id: room_id.map(|id| id.to_string()),
                    errors: Some(super::error_sentence(&errors)),
                    error_fields: errors.0.iter().map(|(f, _)| f.to_string()).collect(),
                },
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await
        }
        Err(error) if error.is_record_not_unique() => redirect(c, bot.id),
        Err(error) => Err(Error::internal(error)),
    }
}
pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let bot = super::find_active_bot(c, "bot_id").await?;
    super::ensure_can_manage_bot(c, &bot).await?;
    concerns::sudo::require_sudo_mode(c)?;
    let agent = super::ensure_agent(c, &bot).await?;
    let id = c
        .param_str("id")
        .and_then(crate::concerns::cast_integer)
        .ok_or(Error::NotFound)?;
    let context = super::audit_context(c)?;
    let bot_name = bot.name.clone();
    let audit = c
        .app()
        .db
        .write(move |tx| {
            let Some(mut grant) =
                AgentGrant::find(tx.conn(), id)?.filter(|g| g.agent_id == agent.id)
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
    let audit = audit.ok_or(Error::NotFound)?;
    if let Some(audit) = audit {
        c.app()
            .db
            .write(move |tx| AuditLog::record(tx, audit, &context))
            .await
            .map_err(Error::internal)?;
    }
    redirect(c, bot.id)
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
async fn render_index(
    c: &mut Ctx,
    bot: &User,
    agent_id: i64,
    form: GrantForm,
    status: StatusCode,
) -> Result {
    c.respond_to(&[&format::HTML])?;
    let (bot_id, viewer) = (bot.id, concerns::require_current_user(c)?.clone());
    let (legacy, grants, rooms) = c
        .app()
        .db
        .read(move |conn| presenters::accounts::bot_access::grants(conn, agent_id, bot_id, &viewer))
        .await
        .map_err(Error::internal)?;
    let bot_name = bot.name.clone();
    presenters::view_context::omit_unused_room_back_link(c);
    framed_page!(c, status, |ctx| {
        campfire_views::accounts::bot_access::Grants {
            ctx,
            bot_id,
            bot_name: bot_name.clone(),
            legacy,
            grants: grants.clone(),
            grant: form.clone(),
            rooms: rooms.clone(),
        }
    })
    .await
}
