use super::*;
use crate::controllers::presenters::page::framed_page;
use campfire_db::{
    audit_log::{AuditLog, NewAuditLog, Target},
    models::slack::{SlackConnection, SlackWorkspace},
};
use campfire_kit::StatusCode;
use campfire_views::slack::{RunSummary, Setup, SetupData};
async fn before(c: &mut Ctx, sudo: bool) -> Result<User> {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    if sudo {
        concerns::require_sudo_mode(c)?;
    }
    Ok(concerns::require_current_user(c)?.clone())
}
async fn data(c: &Ctx, uid: i64) -> Result<SetupData> {
    let crypto = service(c).encryption;
    let mut data=c.app().db.read(move|conn| {
  let w=SlackWorkspace::current(conn)?;let connection=SlackConnection::for_user(conn,uid)?;
  let active=conn.query_row("SELECT id,kind,mode,status FROM slack_imports WHERE status IN ('queued','running','undoing') ORDER BY created_at DESC, id DESC LIMIT 1",[],|r|Ok(RunSummary{id:r.get(0)?,kind:r.get(1)?,mode:r.get(2)?,status:r.get(3)?})).optional()?;
  Ok(SetupData{client_id:w.as_ref().and_then(|w|w.client_id.clone()),configured:w.as_ref().map(|w|w.app_configured(&crypto)).transpose()?.unwrap_or(false),configured_by:w.as_ref().and_then(|w|w.configured_by_id).map(|id|User::find_by_id(conn,id)).transpose()?.flatten().map(|u|u.name),team_name:w.as_ref().and_then(|w|w.team_name.clone()).filter(|s|!campfire_richtext::ruby::is_blank(s)),team_known:w.as_ref().and_then(|w|w.team_id.as_deref()).is_some_and(|s|!campfire_richtext::ruby::is_blank(s)),connection_exists:connection.is_some(),connected:connection.as_ref().map(|c|c.connected(&crypto)).transpose()?.unwrap_or(false),disconnected_reason:connection.and_then(|c|c.disconnected_reason).filter(|s|!campfire_richtext::ruby::is_blank(s)),active_run:active,..Default::default()})
 }).await.map_err(Error::internal)?;
    data.manifest = oauth::manifest(&c.url_for(""));
    Ok(data)
}
use rusqlite::OptionalExtension;
pub async fn show(c: &mut Ctx) -> Result {
    let user = before(c, false).await?;
    let data = data(c, user.id).await?;
    framed_page!(c, StatusCode::OK, |ctx| Setup { ctx, data: &data }).await
}
pub async fn update(c: &mut Ctx) -> Result {
    let user = before(c, true).await?;
    let client_id = campfire_richtext::ruby::strip(&param(c, "client_id")).to_owned();
    let secret = c
        .params
        .get("client_secret")
        .filter(|p| p.is_present())
        .map(|_| campfire_richtext::ruby::strip(&param(c, "client_secret")).to_owned());
    let crypto = service(c).encryption;
    let uid = user.id;
    let id = client_id.clone();
    let result = c
        .app()
        .db
        .write(move |tx| SlackWorkspace::configure(tx, &crypto, &id, secret.as_deref(), uid))
        .await;
    let workspace = match result {
        Ok(w) => w,
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            let mut data = data(c, user.id).await?;
            data.client_id = Some(client_id.clone());
            data.configured = false;
            data.errors = errors.full_messages();
            return framed_page!(c, StatusCode::UNPROCESSABLE_ENTITY, |ctx| Setup {
                ctx,
                data: &data
            })
            .await;
        }
        Err(e) => return Err(Error::internal(e)),
    };
    let context = audit(c, &user)?;
    c.app()
        .db
        .write(move |tx| {
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "slack.workspace.configure".into(),
                    target: Some(Target {
                        record_type: "SlackWorkspace".into(),
                        id: workspace.id,
                        label: None,
                    }),
                    changes: Some(json!({"client_id":client_id})),
                    ..Default::default()
                },
                &context,
            )?;
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    redirect(
        c,
        "/account/slack_import",
        Some("Slack app credentials saved."),
        None,
    )
}
pub async fn destroy(c: &mut Ctx) -> Result {
    let user = before(c, true).await?;
    let context = audit(c, &user)?;
    let removed=c.app().db.write(move|tx| {
  let active:bool=tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM slack_imports WHERE status IN ('queued','running','undoing'))",[],|r|r.get(0))?;
  if active {return Ok(false)}
  if let Some(w)=SlackWorkspace::current(tx.conn())? {w.remove_credentials(tx)?;AuditLog::record(tx,NewAuditLog{action:"slack.workspace.remove_credentials".into(),changes:Some(json!({"client_id":w.client_id})),..Default::default()},&context)?;}
  Ok(true)
 }).await.map_err(Error::internal)?;
    if removed {
        redirect(
            c,
            "/account/slack_import",
            Some("Slack credentials removed."),
            None,
        )
    } else {
        redirect(
            c,
            "/account/slack_import",
            None,
            Some("Finish or cancel the running import first."),
        )
    }
}
