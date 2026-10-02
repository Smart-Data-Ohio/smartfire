//! Slack grant persistence and revocation from the OAuth and connection controllers.
//! HTTP calls stay outside the database writer; each Rails save keeps its commit boundary.
use super::oauth::{self, OAuth};
use campfire_db::{
    Database, Error, Result, User,
    audit_log::{AuditLog, Context, NewAuditLog},
    models::slack::{NewConnection, SlackConnection, SlackWorkspace},
};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::{Value, json};
const ALREADY_CONNECTED: &str =
    "That Slack account is already connected to another Smartfire user.";
pub struct Service<'a> {
    pub db: &'a Database,
    pub oauth: OAuth,
    pub encryption: std::sync::Arc<ArEncryption>,
}
fn dig<'a>(v: &'a Value, first: &str, second: &str) -> Result<&'a Value> {
    match &v[first] {
        Value::Null => Ok(&Value::Null),
        Value::Object(o) => Ok(o.get(second).unwrap_or(&Value::Null)),
        _ => Err(Error::Other("Slack OAuth response TypeError".into())),
    }
}
impl Service<'_> {
    pub async fn current_workspace(&self) -> Result<Option<SlackWorkspace>> {
        let crypto = self.encryption.clone();
        self.db
            .read(move |c| {
                let w = SlackWorkspace::current(c)?;
                match w {
                    Some(w) if w.app_configured(&crypto)? => Ok(Some(w)),
                    _ => Ok(None),
                }
            })
            .await
    }
    pub async fn connect(
        &self,
        user: User,
        workspace: SlackWorkspace,
        exchange: Value,
        context: Context,
    ) -> Result<Option<String>> {
        let team_id = oauth::string(dig(&exchange, "team", "id")?);
        let team_name = oauth::string(dig(&exchange, "team", "name")?);
        let authed = &exchange["authed_user"];
        let scope_string = oauth::string(&authed["scope"]);
        let mut scopes: Vec<_> = scope_string.split(',').collect();
        while scopes.last() == Some(&"") {
            scopes.pop();
        }
        if workspace
            .team_id
            .as_deref()
            .is_some_and(|t| !campfire_richtext::ruby::is_blank(t) && t != team_id)
        {
            return Ok(Some(format!(
                "That Slack account is in a different workspace ({}). Connect with an account in {}.",
                if !campfire_richtext::ruby::is_blank(&team_name) {
                    &team_name
                } else {
                    &team_id
                },
                workspace
                    .team_name
                    .as_deref()
                    .filter(|s| !campfire_richtext::ruby::is_blank(s))
                    .or(workspace.team_id.as_deref())
                    .unwrap_or("")
            )));
        }
        let missing: Vec<_> = oauth::USER_SCOPES
            .into_iter()
            .filter(|s| !scopes.contains(s))
            .collect();
        if !missing.is_empty() {
            return Ok(Some(format!(
                "Slack did not grant every required permission. Missing: {}. Reconnect and approve them all.",
                missing.join(", ")
            )));
        }
        let slack_id = (!authed["id"].is_null()).then(|| oauth::string(&authed["id"]));
        let token = if authed["access_token"].is_null() {
            None
        } else {
            Some(oauth::string(&authed["access_token"]))
        };
        let scopes = scopes.join(",");
        let uid = user.id;
        let wid = workspace.id;
        let claimed_id = slack_id.clone();
        if self
            .db
            .read(move |c| SlackConnection::claimed_by_another(c, wid, claimed_id.as_deref(), uid))
            .await?
        {
            return Ok(Some(ALREADY_CONNECTED.into()));
        }
        let crypto = self.encryption.clone();
        let t = token.clone();
        let connection = self
            .db
            .write(move |tx| {
                SlackConnection::relink(
                    tx,
                    &crypto,
                    NewConnection {
                        workspace_id: wid,
                        user_id: uid,
                        slack_user_id: slack_id.as_deref().unwrap_or(""),
                        access_token: t.as_deref(),
                        scopes: Some(&scopes),
                    },
                )
            })
            .await;
        let connection = match connection {
            Ok(c) => c,
            Err(e) if e.is_record_not_unique() => return Ok(Some(ALREADY_CONNECTED.into())),
            Err(e) => return Err(e),
        };
        if workspace
            .team_id
            .as_deref()
            .is_none_or(campfire_richtext::ruby::is_blank)
            && user.is_administrator()
        {
            let team = self
                .oauth
                .team_info(token.as_deref().unwrap_or(""))
                .await
                .unwrap_or(Value::Null);
            if !team.is_null() && !team.is_object() {
                return Err(Error::Other(
                    "Slack team.info response NoMethodError".into(),
                ));
            }
            let name = if !campfire_richtext::ruby::is_blank(&team_name) {
                Some(team_name)
            } else if oauth::present(&team["name"]) {
                Some(oauth::string(&team["name"]))
            } else {
                None
            };
            let domain = oauth::present(&team["domain"]).then(|| oauth::string(&team["domain"]));
            let id = team_id.clone();
            let crypto = self.encryption.clone();
            self.db
                .write(move |tx| {
                    workspace.name_team(tx, &crypto, &id, name.as_deref(), domain.as_deref())
                })
                .await?;
        }
        self.db
            .write(move |tx| {
                AuditLog::record(
                    tx,
                    NewAuditLog {
                        action: "slack.account.connect".into(),
                        target: Some((&user).into()),
                        changes: Some(
                            json!({"slack_user_id":connection.slack_user_id,"team_id":team_id}),
                        ),
                        ..Default::default()
                    },
                    &context,
                )?;
                Ok(())
            })
            .await?;
        Ok(None)
    }
    pub async fn disconnect(&self, user: User, context: Context) -> Result<bool> {
        let uid = user.id;
        if self.db.read(move|c|Ok(c.query_row("SELECT EXISTS(SELECT 1 FROM slack_imports WHERE user_id=? AND status IN ('queued','running','undoing'))",[uid],|r|r.get::<_,bool>(0))?)).await? {return Ok(false)}
        if let Some(c) = self
            .db
            .read(move |c| SlackConnection::for_user(c, uid))
            .await?
        {
            if let Some(token) = c
                .access_token(&self.encryption)
                .ok()
                .flatten()
                .filter(|s| !campfire_richtext::ruby::is_blank(s))
            {
                self.oauth.revoke(&token).await;
            }
            self.db
                .write(move |tx| {
                    SlackConnection::destroy(tx, c.id)?;
                    AuditLog::record(
                        tx,
                        NewAuditLog {
                            action: "slack.account.disconnect".into(),
                            target: Some((&user).into()),
                            changes: Some(json!({"slack_user_id":c.slack_user_id})),
                            ..Default::default()
                        },
                        &context,
                    )?;
                    Ok(())
                })
                .await?;
        }
        Ok(true)
    }
}
