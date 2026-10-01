//! `app/models/slack_workspace.rb` and `slack_connection.rb`. Ciphertext stays private,
//! and neither model implements Debug: user tokens and app secrets must never reach logs.
use rails_compat::ar_encryption::ArEncryption;
use rusqlite::{Connection, Row, params};

use crate::sql::query_one;
use crate::{Error, Errors, Result, Timestamp, Tx};

#[derive(Clone)]
pub struct SlackWorkspace {
    pub id: i64,
    pub client_id: Option<String>,
    encrypted_secret: Option<String>,
    pub configured_by_id: Option<i64>,
    pub team_id: Option<String>,
    pub team_name: Option<String>,
    pub team_domain: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
impl SlackWorkspace {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            client_id: row.get("client_id")?,
            encrypted_secret: row.get("client_secret")?,
            configured_by_id: row.get("configured_by_id")?,
            team_id: row.get("team_id")?,
            team_name: row.get("team_name")?,
            team_domain: row.get("team_domain")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn current(conn: &Connection) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM slack_workspaces ORDER BY id LIMIT 1",
            [],
            Self::from_row,
        )
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM slack_workspaces WHERE id = ?",
            [id],
            Self::from_row,
        )
    }
    pub fn client_secret(&self, encryption: &ArEncryption) -> Result<Option<String>> {
        decrypt(encryption, self.encrypted_secret.as_deref())
    }
    pub fn app_configured(&self, encryption: &ArEncryption) -> Result<bool> {
        Ok(!blank(self.client_id.as_deref()) && !blank(self.client_secret(encryption)?.as_deref()))
    }
    pub fn create(
        tx: &mut Tx<'_>,
        encryption: &ArEncryption,
        client_id: &str,
        secret: &str,
        configured_by_id: Option<i64>,
    ) -> Result<Self> {
        require_transaction(tx)?;
        let mut errors = Errors::default();
        if blank(Some(client_id)) {
            errors.add("client_id", "can't be blank");
        }
        if blank(Some(secret)) {
            errors.add("client_secret", "can't be blank");
        }
        // Rails marks configured_by optional, including a stale configured_by_id.
        errors.into_result()?;
        let ciphertext = encryption.encrypt(secret);
        let id = tx.conn().query_row("INSERT INTO slack_workspaces (client_id, client_secret, configured_by_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?) RETURNING id", params![client_id, ciphertext, configured_by_id, tx.now(), tx.now()], |r| r.get(0))?;
        Self::find(tx.conn(), id)?.ok_or(Error::RecordNotFound("SlackWorkspace"))
    }
}

#[derive(Clone)]
pub struct SlackConnection {
    pub id: i64,
    pub slack_workspace_id: i64,
    pub user_id: i64,
    pub slack_user_id: String,
    encrypted_token: Option<String>,
    pub scopes: Option<String>,
    pub disconnected_reason: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
pub struct NewConnection<'a> {
    pub workspace_id: i64,
    pub user_id: i64,
    pub slack_user_id: &'a str,
    pub access_token: Option<&'a str>,
    pub scopes: Option<&'a str>,
}
impl SlackConnection {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            slack_workspace_id: row.get("slack_workspace_id")?,
            user_id: row.get("user_id")?,
            slack_user_id: row.get("slack_user_id")?,
            encrypted_token: row.get("access_token")?,
            scopes: row.get("scopes")?,
            disconnected_reason: row.get("disconnected_reason")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM slack_connections WHERE id = ?",
            [id],
            Self::from_row,
        )
    }
    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM slack_connections WHERE user_id = ?",
            [user_id],
            Self::from_row,
        )
    }
    pub fn access_token(&self, encryption: &ArEncryption) -> Result<Option<String>> {
        decrypt(encryption, self.encrypted_token.as_deref())
    }
    pub fn connected(&self, encryption: &ArEncryption) -> Result<bool> {
        Ok(blank(self.disconnected_reason.as_deref())
            && !blank(self.access_token(encryption)?.as_deref()))
    }
    pub fn create(
        tx: &mut Tx<'_>,
        encryption: &ArEncryption,
        new: NewConnection<'_>,
    ) -> Result<Self> {
        require_transaction(tx)?;
        let mut errors = Errors::default();
        if blank(Some(new.slack_user_id)) {
            errors.add("slack_user_id", "can't be blank");
        }
        if SlackWorkspace::find(tx.conn(), new.workspace_id)?.is_none() {
            errors.add("slack_workspace", "must exist");
        }
        let user_exists: bool = tx.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = ?)",
            [new.user_id],
            |r| r.get(0),
        )?;
        if !user_exists {
            errors.add("user", "must exist");
        }
        errors.into_result()?;
        let token = new.access_token.map(|token| encryption.encrypt(token));
        let id = tx.conn().query_row("INSERT INTO slack_connections (slack_workspace_id, user_id, slack_user_id, access_token, scopes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id", params![new.workspace_id, new.user_id, new.slack_user_id, token, new.scopes, tx.now(), tx.now()], |r| r.get(0))?;
        Self::find(tx.conn(), id)?.ok_or(Error::RecordNotFound("SlackConnection"))
    }
    pub fn destroy(tx: &mut Tx<'_>, id: i64) -> Result<()> {
        require_transaction(tx)?;
        // dependent: :nullify uses update_all (does not touch each run's updated_at).
        tx.conn().execute(
            "UPDATE slack_imports SET slack_connection_id = NULL WHERE slack_connection_id = ?",
            [id],
        )?;
        tx.conn()
            .execute("DELETE FROM slack_connections WHERE id = ?", [id])?;
        Ok(())
    }
}
fn decrypt(encryption: &ArEncryption, ciphertext: Option<&str>) -> Result<Option<String>> {
    ciphertext
        .map(|s| {
            encryption
                .decrypt(s)
                .map_err(|e| Error::Other(e.to_string()))
        })
        .transpose()
}
fn blank(value: Option<&str>) -> bool {
    value.is_none_or(|s| s.chars().all(char::is_whitespace))
}
fn require_transaction(tx: &Tx<'_>) -> Result<()> {
    if tx.in_transaction() {
        Ok(())
    } else {
        Err(Error::Other(
            "Slack credential mutations require a transaction".into(),
        ))
    }
}
