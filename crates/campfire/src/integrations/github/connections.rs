//! Connection lifecycle from GitHub's human/App/bot controllers; no request or HTML state.
use super::{
    accounts::{Account, AccountInput, Accounts},
    client::{AppClient, Error, ErrorKind, ruby_to_i},
};
use campfire_db::{
    Database, Timestamp, User,
    audit_log::{AuditLog, Context, NewAuditLog, Target},
};
use rails_compat::ar_encryption::ArEncryption;
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
use std::sync::Arc;

pub struct Credentials {
    pub login: String,
    pub access: String,
    pub refresh: Option<String>,
    pub expires: Option<Timestamp>,
    pub source: &'static str,
}
pub struct ConnectionService<'a> {
    pub db: &'a Database,
    pub accounts: &'a Accounts,
    pub app: &'a AppClient,
    pub crypto: Arc<ArEncryption>,
}
impl ConnectionService<'_> {
    pub async fn pat(&self, token: &str) -> Result<Credentials, Error> {
        Ok(Credentials {
            login: self
                .accounts
                .write_client(token.into())
                .authenticated_login()
                .await?,
            access: token.into(),
            refresh: None,
            expires: None,
            source: "pat",
        })
    }
    pub async fn exchange(&self, code: &str, redirect: &str) -> Result<Credentials, Error> {
        let tokens = self.app.exchange_code(code, redirect).await?;
        let access = super::client::ruby_string(&tokens["access_token"]);
        let login = self
            .accounts
            .write_client(access.clone())
            .authenticated_login()
            .await?;
        let expires = if tokens["expires_in"].is_null() || tokens["expires_in"] == false {
            None
        } else {
            let seconds = match &tokens["expires_in"] {
                Value::String(s) => ruby_to_i(s),
                Value::Number(n) => n
                    .as_i64()
                    .or_else(|| n.as_f64().map(|v| v.trunc() as i64))
                    .unwrap_or(0),
                _ => {
                    return Err(Error {
                        kind: ErrorKind::TypeError,
                        message: "GitHub returned an invalid expiry".into(),
                    });
                }
            };
            Some(Timestamp::from_jiff(
                self.db
                    .env()
                    .now()
                    .jiff()
                    .checked_add(jiff::SignedDuration::from_secs(seconds))
                    .map_err(|_| Error {
                        kind: ErrorKind::TypeError,
                        message: "GitHub returned an invalid expiry".into(),
                    })?,
            ))
        };
        Ok(Credentials {
            login,
            access,
            refresh: tokens
                .get("refresh_token")
                .filter(|v| !v.is_null())
                .map(super::client::ruby_string),
            expires,
            source: "app",
        })
    }
    pub async fn link(
        &self,
        user: User,
        credentials: Credentials,
        bot: bool,
        context: Context,
    ) -> campfire_db::Result<(Account, bool)> {
        let user_id = user.id;
        let old = if !bot {
            match self
                .db
                .read(move |conn| Account::for_user(conn, user_id))
                .await?
            {
                Some(a) => self.accounts.app_token_for_revoke(a.id).await?,
                None => None,
            }
        } else {
            None
        };
        let access = credentials.access.clone();
        let crypto = self.crypto.clone();
        let result = self
            .db
            .write(move |tx| {
                let input = AccountInput {
                    user_id: user.id,
                    github_login: &credentials.login,
                    access_token: &credentials.access,
                    refresh_token: credentials.refresh.as_deref(),
                    token_expires_at: credentials.expires,
                    token_source: credentials.source,
                };
                let account = if bot {
                    Account::relink_without_touch(tx, &crypto, &input)?
                } else {
                    Account::relink(tx, &crypto, &input)?
                };
                let matched = tx
                    .conn()
                    .query_row(
                        "SELECT github_login FROM users WHERE id=?",
                        [user.id],
                        |r| r.get::<_, Option<String>>(0),
                    )?
                    .as_deref()
                    == Some(&account.github_login.to_lowercase());
                let target = target(tx.conn(), &user, bot)?;
                AuditLog::record(
                    tx,
                    NewAuditLog {
                        action: if bot {
                            "agent.github.connect"
                        } else {
                            "github.account.connect"
                        }
                        .into(),
                        target: Some(target),
                        changes: Some(json!({"github_login":account.github_login})),
                        ..Default::default()
                    },
                    &context,
                )?;
                Ok((account, matched))
            })
            .await?;
        if let Some(old) = old.filter(|s| s != &access) {
            self.app.revoke_token(&old).await;
        }
        Ok(result)
    }
    pub async fn disconnect(
        &self,
        user: User,
        bot: bool,
        context: Context,
    ) -> campfire_db::Result<()> {
        let user_id = user.id;
        if let Some(account) = self
            .db
            .read(move |conn| Account::for_user(conn, user_id))
            .await?
        {
            self.accounts.revoke_remote_token(account.id).await?;
            self.db
                .write(move |tx| {
                    tx.conn().execute(
                        "DELETE FROM github_connected_accounts WHERE id=?",
                        [account.id],
                    )?;
                    AuditLog::record(
                        tx,
                        NewAuditLog {
                            action: if bot {
                                "agent.github.disconnect"
                            } else {
                                "github.account.disconnect"
                            }
                            .into(),
                            target: Some(target(tx.conn(), &user, bot)?),
                            changes: Some(json!({"github_login":account.github_login})),
                            ..Default::default()
                        },
                        &context,
                    )?;
                    Ok(())
                })
                .await?;
        }
        Ok(())
    }
}
fn target(conn: &rusqlite::Connection, user: &User, bot: bool) -> campfire_db::Result<Target> {
    if bot
        && let Some(id) = conn
            .query_row("SELECT id FROM agents WHERE user_id=?", [user.id], |r| {
                r.get::<_, i64>(0)
            })
            .optional()?
    {
        return Ok(Target {
            record_type: "Agent".into(),
            id,
            label: Some(format!("Agent {}", user.name)),
        });
    }
    Ok(Target::from(user))
}
