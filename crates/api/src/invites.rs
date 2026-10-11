use axum::{
    Router,
    routing::{delete, get},
};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::models::audit_log::{AuditLog, NewAuditLog, Target};
use campfire_db::models::workspace_invite::{InviteExpiry, InviteState, WorkspaceInvite};
use campfire_db::{Timestamp, Tx};
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode, action, unparsed_action};
use campfire_people::controllers::two_factor::audit_context;

use crate::admin::{administrator, body};
use crate::dto::time;

pub fn routes() -> Router<Kit> {
    Router::new()
        .route(
            "/api/v1/admin/invites",
            get(action(index)).post(unparsed_action(create)),
        )
        .route("/api/v1/admin/invites/{id}", delete(action(revoke)))
}

endpoint!(index => list_invites);
endpoint!(create => create_invite);
endpoint!(revoke => revoke_invite);

fn dto(invite: WorkspaceInvite, now: Timestamp) -> api::WorkspaceInvite {
    let state = match invite.state(now) {
        InviteState::Active => api::WorkspaceInviteState::Active,
        InviteState::Expired => api::WorkspaceInviteState::Expired,
        InviteState::Exhausted => api::WorkspaceInviteState::Exhausted,
        InviteState::Revoked => api::WorkspaceInviteState::Revoked,
    };
    api::WorkspaceInvite {
        id: invite.id,
        creator: api::WorkspaceInviteCreator {
            id: invite.creator_id,
            name: invite.creator_name,
        },
        expires_at: invite.expires_at.map(time),
        max_uses: invite.max_uses,
        uses: invite.uses,
        revoked_at: invite.revoked_at.map(time),
        created_at: time(invite.created_at),
        updated_at: time(invite.updated_at),
        state,
    }
}

fn expiry(choice: api::WorkspaceInviteExpiry) -> InviteExpiry {
    match choice {
        api::WorkspaceInviteExpiry::ThirtyMinutes => InviteExpiry::ThirtyMinutes,
        api::WorkspaceInviteExpiry::OneHour => InviteExpiry::OneHour,
        api::WorkspaceInviteExpiry::SixHours => InviteExpiry::SixHours,
        api::WorkspaceInviteExpiry::TwelveHours => InviteExpiry::TwelveHours,
        api::WorkspaceInviteExpiry::OneDay => InviteExpiry::OneDay,
        api::WorkspaceInviteExpiry::SevenDays => InviteExpiry::SevenDays,
        api::WorkspaceInviteExpiry::Never => InviteExpiry::Never,
    }
}

fn audit(
    tx: &Tx<'_>,
    invite: &WorkspaceInvite,
    action: &str,
    context: &campfire_db::models::audit_log::Context,
) -> campfire_db::Result<()> {
    AuditLog::record(
        tx,
        NewAuditLog {
            action: action.into(),
            target: Some(Target {
                record_type: "WorkspaceInvite".into(),
                id: invite.id,
                label: Some(format!("Invite {}", invite.id)),
            }),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

async fn list_invites(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let now = Timestamp::from_jiff(c.now());
    let invites = c
        .app()
        .db
        .read(WorkspaceInvite::list)
        .await
        .map_err(Error::internal)?;
    c.json(
        StatusCode::OK,
        &api::WorkspaceInviteList {
            invites: invites.into_iter().map(|invite| dto(invite, now)).collect(),
        },
    )
}

async fn create_invite(c: &mut Ctx) -> Result {
    let creator = administrator(c).await?;
    let input: api::CreateWorkspaceInvite = body(c).await?;
    let context = audit_context(c)?;
    let (invite, token) = c
        .app()
        .db
        .write(move |tx| {
            let (invite, token) =
                WorkspaceInvite::create(tx, creator.id, expiry(input.expiry), input.max_uses)?;
            audit(tx, &invite, "workspace_invite.create", &context)?;
            Ok((invite, token))
        })
        .await
        .map_err(Error::internal)?;
    let url = c.url_for(&format!("/invite/{token}"));
    c.json(
        StatusCode::CREATED,
        &api::WorkspaceInviteCreated {
            invite: dto(invite, Timestamp::from_jiff(c.now())),
            token,
            url,
        },
    )
}

async fn revoke_invite(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let id = c
        .param_str("id")
        .and_then(|id| id.parse::<i64>().ok())
        .ok_or(Error::NotFound)?;
    let context = audit_context(c)?;
    let invite = c
        .app()
        .db
        .write(move |tx| {
            let previous = WorkspaceInvite::find(tx.conn(), id)?
                .ok_or(campfire_db::Error::RecordNotFound("WorkspaceInvite"))?;
            let invite = WorkspaceInvite::revoke(tx, id)?
                .ok_or(campfire_db::Error::RecordNotFound("WorkspaceInvite"))?;
            if previous.revoked_at.is_none() {
                audit(tx, &invite, "workspace_invite.revoke", &context)?;
            }
            Ok(invite)
        })
        .await
        .map_err(Error::internal)?;
    c.json(StatusCode::OK, &dto(invite, Timestamp::from_jiff(c.now())))
}
