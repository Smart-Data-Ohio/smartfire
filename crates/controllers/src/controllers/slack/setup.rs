use super::*;
use campfire_db::{
    audit_log::{AuditLog, NewAuditLog, Target},
    models::slack::SlackWorkspace,
};
use campfire_kit::StatusCode;
async fn before(c: &mut Ctx, sudo: bool) -> Result<User> {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    if sudo {
        concerns::require_sudo_mode(c).await?;
    }
    Ok(concerns::require_current_user(c)?.clone())
}
pub async fn update(c: &mut Ctx) -> Result {
    let user = before(c, true).await?;
    let client_id = campfire_richtext::ruby::strip(&param(c, "client_id")).to_owned();
    let secret = c
        .params
        .get("client_secret")
        .filter(|p| p.is_present())
        .map(|_| campfire_richtext::ruby::strip(&param(c, "client_secret")).to_owned());
    if let Err(_errors) = configure(c, &user, client_id.clone(), secret).await? {
        return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
    }
    redirect(
        c,
        "/account/slack_import",
        Some("Slack app credentials saved."),
        None,
    )
}
/// `update` once the administrator is known and their password confirmed: saves the app's
/// Client ID (stripped) and, when given, a new Client Secret, then the audit. The inner error is
/// the form's messages.
pub async fn configure(
    c: &Ctx,
    user: &User,
    client_id: String,
    secret: Option<String>,
) -> Result<std::result::Result<(), Vec<String>>> {
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
        Err(campfire_db::Error::RecordInvalid(errors)) => return Ok(Err(errors.full_messages())),
        Err(e) => return Err(Error::internal(e)),
    };
    let context = audit(c, user)?;
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
                        label: Some(format!("SlackWorkspace #{}", workspace.id)),
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
    Ok(Ok(()))
}
pub async fn destroy(c: &mut Ctx) -> Result {
    let user = before(c, true).await?;
    let removed = remove(c, &user).await?;
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
/// `destroy` once the administrator is known and their password confirmed: removes the app
/// credentials and every member's connection, then the audit; `false` (nothing removed) while a
/// run is active.
pub async fn remove(c: &Ctx, user: &User) -> Result<bool> {
    let context = audit(c, user)?;
    c.app().db.write(move|tx| {
  let active:bool=tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM slack_imports WHERE status IN ('queued','running','undoing'))",[],|r|r.get(0))?;
  if active {return Ok(false)}
  if let Some(w)=SlackWorkspace::current(tx.conn())? {w.remove_credentials(tx)?;AuditLog::record(tx,NewAuditLog{action:"slack.workspace.remove_credentials".into(),changes:Some(json!({"client_id":w.client_id})),..Default::default()},&context)?;}
  Ok(true)
 }).await.map_err(Error::internal)
}
