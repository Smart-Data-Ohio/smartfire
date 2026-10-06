//! `app/models/github_connected_account.rb`: encrypted rows, verified-login claim, optimistic
//! rotating refresh, and per-viewer repository access. Network calls never hold SQLite locks.
use campfire_db::{Database, Errors, Result, Timestamp, Tx};
use jiff::SignedDuration;
use rails_compat::ar_encryption::ArEncryption;
use rails_compat::unicode;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::{
    blank,
    client::{AppClient, ErrorKind, WriteClient, ruby_strip, ruby_to_i},
};
use crate::net::Network;

pub const UNREADABLE_TOKEN_REASON: &str = "The stored token could not be read; link it again";
pub const REJECTED_TOKEN_REASON: &str = "GitHub rejected the linked token (401)";

/// Ciphertexts are kept private and this type deliberately has no `Debug`/`Serialize`.
#[derive(Clone)]
pub struct Account {
    pub id: i64,
    pub user_id: i64,
    pub github_login: String,
    pub token_source: String,
    pub token_expires_at: Option<Timestamp>,
    pub disconnected_reason: Option<String>,
    pub last_error: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    access_token: String,
    refresh_token: Option<String>,
}

pub struct AccountInput<'a> {
    pub user_id: i64,
    pub github_login: &'a str,
    pub access_token: &'a str,
    pub refresh_token: Option<&'a str>,
    pub token_expires_at: Option<Timestamp>,
    pub token_source: &'a str,
}

impl Account {
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        Self::query(conn, "id", id)
    }
    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        Self::query(conn, "user_id", user_id)
    }
    fn query(conn: &Connection, column: &str, value: i64) -> Result<Option<Self>> {
        Ok(conn.query_row(&format!("SELECT id, user_id, github_login, token_source, token_expires_at, disconnected_reason, last_error, created_at, updated_at, access_token, refresh_token FROM github_connected_accounts WHERE {column} = ?"), [value], |r| Ok(Self {
            id: r.get(0)?, user_id: r.get(1)?, github_login: r.get(2)?, token_source: r.get(3)?, token_expires_at: r.get(4)?, disconnected_reason: r.get(5)?, last_error: r.get(6)?, created_at: r.get(7)?, updated_at: r.get(8)?, access_token: r.get(9)?, refresh_token: r.get(10)?,
        })).optional()?)
    }
    pub fn connected(&self) -> bool {
        self.disconnected_reason.as_deref().is_none_or(blank)
    }
    pub fn app_token(&self) -> bool {
        self.token_source == "app"
    }
    pub fn app_token_expired(&self, now: Timestamp) -> bool {
        self.app_token()
            && self
                .token_expires_at
                .is_some_and(|t| t <= now.since(SignedDuration::from_secs(60)))
    }
    pub fn create(
        tx: &mut Tx<'_>,
        crypto: &ArEncryption,
        input: &AccountInput<'_>,
    ) -> Result<Self> {
        Self::save(tx, crypto, None, input)
    }
    /// Controller relinks replace all credential fields and always touch `updated_at`.
    pub fn relink(
        tx: &mut Tx<'_>,
        crypto: &ArEncryption,
        input: &AccountInput<'_>,
    ) -> Result<Self> {
        let id = Self::for_user(tx.conn(), input.user_id)?.map(|a| a.id);
        Self::save(tx, crypto, id, input)
    }
    pub fn relink_without_touch(
        tx: &mut Tx<'_>,
        crypto: &ArEncryption,
        input: &AccountInput<'_>,
    ) -> Result<Self> {
        if let Some(account) = Self::for_user(tx.conn(), input.user_id)? {
            Self::validate(tx, Some(account.id), input)?;
            if account.github_login == input.github_login
                && account.token_source == input.token_source
                && account.token_expires_at == input.token_expires_at
                && account.disconnected_reason.is_none()
                && account.last_error.is_none()
                && crypto.decrypt(&account.access_token).ok().as_deref() == Some(input.access_token)
                && account
                    .refresh_token
                    .as_deref()
                    .map(|s| crypto.decrypt(s))
                    .transpose()
                    .ok()
                    .flatten()
                    .as_deref()
                    == input.refresh_token
            {
                account.claim_verified_login(tx)?;
                return Ok(account);
            }
        }
        Self::relink(tx, crypto, input)
    }
    fn validate(tx: &Tx<'_>, id: Option<i64>, input: &AccountInput<'_>) -> Result<()> {
        let mut errors = Errors::default();
        if !tx.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = ?)",
            [input.user_id],
            |r| r.get::<_, bool>(0),
        )? {
            errors.add("user", "must exist");
        }
        if tx.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM github_connected_accounts WHERE user_id = ? AND id != ?)",
            params![input.user_id, id.unwrap_or(0)],
            |r| r.get::<_, bool>(0),
        )? {
            errors.add("user_id", "has already been taken");
        }
        if blank(input.github_login) {
            errors.add("github_login", "can't be blank");
        }
        if !["pat", "app"].contains(&input.token_source) {
            errors.add("token_source", "is not included in the list");
        }
        errors.into_result()
    }
    fn validate_current(&self, tx: &Tx<'_>) -> Result<()> {
        Self::validate(
            tx,
            Some(self.id),
            &AccountInput {
                user_id: self.user_id,
                github_login: &self.github_login,
                access_token: "",
                refresh_token: None,
                token_expires_at: self.token_expires_at,
                token_source: &self.token_source,
            },
        )
    }
    fn save(
        tx: &mut Tx<'_>,
        crypto: &ArEncryption,
        id: Option<i64>,
        input: &AccountInput<'_>,
    ) -> Result<Self> {
        Self::validate(tx, id, input)?;
        let access = crypto.encrypt(input.access_token);
        let refresh = input.refresh_token.map(|token| crypto.encrypt(token));
        let now = tx.now();
        let id = if let Some(id) = id {
            tx.conn().execute("UPDATE github_connected_accounts SET github_login = ?, access_token = ?, refresh_token = ?, token_expires_at = ?, token_source = ?, disconnected_reason = NULL, last_error = NULL, updated_at = ? WHERE id = ?", params![input.github_login, access, refresh, input.token_expires_at, input.token_source, now, id])?;
            id
        } else {
            tx.conn().execute("INSERT INTO github_connected_accounts (user_id, github_login, access_token, refresh_token, token_expires_at, token_source, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", params![input.user_id, input.github_login, access, refresh, input.token_expires_at, input.token_source, now, now])?;
            tx.conn().last_insert_rowid()
        };
        let account = Self::find(tx.conn(), id)?
            .ok_or(campfire_db::Error::RecordNotFound("GithubConnectedAccount"))?;
        account.claim_verified_login(tx)?;
        Ok(account)
    }
    fn claim_verified_login(&self, tx: &Tx<'_>) -> Result<()> {
        if !self.connected() {
            return Ok(());
        }
        let login = unicode::downcase(ruby_strip(&self.github_login));
        if blank(&login) {
            return Ok(());
        }
        let own: Option<String> = tx.conn().query_row(
            "SELECT github_login FROM users WHERE id = ?",
            [self.user_id],
            |r| r.get(0),
        )?;
        if own.as_deref() == Some(&login) {
            return Ok(());
        }
        let claimants: Vec<i64> = tx
            .conn()
            .prepare("SELECT id FROM users WHERE LOWER(github_login) = ? AND id != ?")?
            .query_map(params![login, self.user_id], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        for claimant in &claimants {
            if let Some(account) = Self::for_user(tx.conn(), *claimant)?
                && account.connected()
                && unicode::fold(&account.github_login) == unicode::fold(&login)
            {
                return Ok(());
            }
        }
        for claimant in claimants {
            tx.conn().execute(
                "UPDATE users SET github_login = NULL WHERE id = ?",
                [claimant],
            )?;
        }
        tx.conn().execute(
            "UPDATE users SET github_login = ? WHERE id = ?",
            params![login, self.user_id],
        )?;
        Ok(())
    }
    /// Marking disconnected uses `update!`: dirty saves update the stamp; no-op saves keep it.
    pub fn mark_disconnected(tx: &Tx<'_>, id: i64, reason: &str) -> Result<()> {
        let account = Self::find(tx.conn(), id)?
            .ok_or(campfire_db::Error::RecordNotFound("GithubConnectedAccount"))?;
        account.validate_current(tx)?;
        if account.disconnected_reason.as_deref() == Some(reason) {
            return account.claim_verified_login(tx);
        }
        tx.conn().execute("UPDATE github_connected_accounts SET disconnected_reason = ?, updated_at = ? WHERE id = ?", params![reason, tx.now(), id])?;
        Self::find(tx.conn(), id)?
            .ok_or(campfire_db::Error::RecordNotFound("GithubConnectedAccount"))?
            .claim_verified_login(tx)
    }
}

type RepositoryKey = (i64, Timestamp, String, String);
#[derive(Clone)]
pub struct Accounts {
    db: Database,
    crypto: Arc<ArEncryption>,
    app: AppClient,
    network: Network,
    repository_cache: Arc<Mutex<HashMap<RepositoryKey, (Timestamp, bool)>>>,
}
impl Accounts {
    pub fn new(db: Database, crypto: Arc<ArEncryption>, app: AppClient) -> Self {
        Self::with_network(db, crypto, app, Network::system())
    }
    pub fn with_network(
        db: Database,
        crypto: Arc<ArEncryption>,
        app: AppClient,
        network: Network,
    ) -> Self {
        Self {
            db,
            crypto,
            app,
            network,
            repository_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub fn write_client(&self, token: String) -> WriteClient {
        WriteClient::with_network(token, self.network.clone())
    }
    fn find(&self, id: i64) -> Result<Option<Account>> {
        self.db.read_blocking(|conn| Account::find(conn, id))
    }
    async fn disconnect(&self, id: i64, reason: &'static str) -> Result<()> {
        self.db
            .write(move |tx| Account::mark_disconnected(tx, id, reason))
            .await
    }
    async fn decrypt_access(&self, account: &Account) -> Result<Option<String>> {
        match self.crypto.decrypt(&account.access_token) {
            Ok(token) => Ok((!blank(&token)).then_some(token)),
            Err(_) => {
                self.disconnect(account.id, UNREADABLE_TOKEN_REASON).await?;
                Ok(None)
            }
        }
    }
    pub async fn usable(&self, id: i64) -> Result<bool> {
        let Some(account) = self.find(id)?.filter(Account::connected) else {
            return Ok(false);
        };
        Ok(self.decrypt_access(&account).await?.is_some())
    }
    pub async fn access_token_for_use(&self, id: i64) -> Result<Option<String>> {
        Ok(self
            .account_token_for_use(id)
            .await?
            .map(|(_, token)| token))
    }
    // The token and cache version must come from one snapshot. Reloading only updated_at after
    // reading a token could cache an old identity's denial under a concurrently relinked account.
    async fn account_token_for_use(&self, id: i64) -> Result<Option<(Account, String)>> {
        let Some(account) = self.find(id)?.filter(Account::connected) else {
            return Ok(None);
        };
        let Some(token) = self.decrypt_access(&account).await? else {
            return Ok(None);
        };
        if !account.app_token_expired(self.db.env().now()) {
            return Ok(Some((account, token)));
        }
        let refresh = match account
            .refresh_token
            .as_deref()
            .map(|t| self.crypto.decrypt(t))
            .transpose()
        {
            Ok(Some(token)) if !blank(&token) => token,
            Ok(_) => return Ok(None),
            Err(_) => {
                self.disconnect(id, UNREADABLE_TOKEN_REASON).await?;
                return Ok(None);
            }
        };
        // The snapshot read is finished. No read connection or writer transaction crosses this await.
        match self.app.refresh_access_token(&refresh).await {
            Ok(tokens) => {
                let crypto = self.crypto.clone();
                let refreshed = self
                    .db
                    .write(move |tx| store_refreshed_tokens(tx, &crypto, id, &refresh, &tokens))
                    .await;
                let refreshed = match refreshed {
                    Err(campfire_db::Error::Other(reason)) if reason == UNREADABLE_TOKEN_REASON => {
                        self.disconnect(id, UNREADABLE_TOKEN_REASON).await?;
                        return Ok(None);
                    }
                    result => result?,
                };
                if !refreshed {
                    return Ok(None);
                }
            }
            Err(error) if error.kind == ErrorKind::Unauthorized => {
                // Check and disconnect in the same short transaction so a concurrent winner survives.
                let won = self
                    .db
                    .write(move |tx| {
                        let Some(current) = Account::find(tx.conn(), id)? else {
                            return Ok(false);
                        };
                        if current.connected() && !current.app_token_expired(tx.now()) {
                            return Ok(true);
                        }
                        Account::mark_disconnected(tx, id, REJECTED_TOKEN_REASON)?;
                        Ok(false)
                    })
                    .await?;
                if !won {
                    return Ok(None);
                }
            }
            Err(error) => {
                let mut text: String = error.message.chars().take(250).collect();
                if error.message.chars().count() > 250 {
                    text = error.message.chars().take(247).collect::<String>() + "...";
                }
                // Rails `update_column`: no timestamp or after_save callback on transient failures.
                self.db
                    .write(move |tx| {
                        tx.conn().execute(
                            "UPDATE github_connected_accounts SET last_error = ? WHERE id = ?",
                            params![text, id],
                        )?;
                        Ok(())
                    })
                    .await?;
                return Ok(None);
            }
        }
        match self.find(id)? {
            Some(account) => Ok(self
                .decrypt_access(&account)
                .await?
                .map(|token| (account, token))),
            None => Ok(None),
        }
    }
    pub async fn app_token_for_revoke(&self, id: i64) -> Result<Option<String>> {
        let Some(account) = self.find(id)?.filter(Account::app_token) else {
            return Ok(None);
        };
        // Relink does not refresh or mark an unreadable old token disconnected.
        Ok(self
            .crypto
            .decrypt(&account.access_token)
            .ok()
            .filter(|t| !blank(t)))
    }
    pub async fn revoke_remote_token(&self, id: i64) -> Result<()> {
        if self.find(id)?.is_some_and(|a| a.app_token())
            && let Some(token) = self.access_token_for_use(id).await?
        {
            self.app.revoke_grant(&token).await;
        }
        Ok(())
    }
    pub async fn can_read_repository(&self, id: i64, owner: &str, repo: &str) -> Result<bool> {
        let Some((account, token)) = self.account_token_for_use(id).await? else {
            return Ok(false);
        };
        let now = self.db.env().now();
        let (owner, repo) = (owner.to_lowercase(), repo.to_lowercase());
        let key = (
            account.user_id,
            account.updated_at,
            owner.clone(),
            repo.clone(),
        );
        {
            let mut cache = self.repository_cache.lock().expect("repository cache lock");
            cache.retain(|_, (expiry, _)| *expiry > now);
            if let Some((_, readable)) = cache.get(&key) {
                return Ok(*readable);
            }
        }
        let readable = match WriteClient::with_network(token, self.network.clone())
            .repository_readable(&owner, &repo)
            .await
        {
            Ok(readable) => readable,
            Err(error) if error.kind == ErrorKind::Unauthorized => {
                self.disconnect(id, REJECTED_TOKEN_REASON).await?;
                false
            }
            Err(_) => return Ok(false),
        };
        self.repository_cache
            .lock()
            .expect("repository cache lock")
            .insert(
                key,
                (
                    self.db.env().now().since(SignedDuration::from_mins(10)),
                    readable,
                ),
            );
        Ok(readable)
    }
    /// `Github::AgentIdentity.resolve`: prefers an owner's usable App identity without refreshing.
    pub async fn agent_identity(
        &self,
        owner_user_id: Option<i64>,
        agent_user_id: i64,
    ) -> Result<Option<Account>> {
        if let Some(owner_id) = owner_user_id
            && let Some(account) = self
                .db
                .read_blocking(|conn| Account::for_user(conn, owner_id))?
            && self.usable(account.id).await?
            && account.app_token()
        {
            return Ok(Some(account));
        }
        self.db
            .read_blocking(|conn| Account::for_user(conn, agent_user_id))
    }
}
fn store_refreshed_tokens(
    tx: &mut Tx<'_>,
    crypto: &ArEncryption,
    id: i64,
    used: &str,
    tokens: &Value,
) -> Result<bool> {
    let Some(current) = Account::find(tx.conn(), id)? else {
        return Ok(false);
    };
    let stored_refresh = current
        .refresh_token
        .as_deref()
        .map(|value| crypto.decrypt(value))
        .transpose()
        .map_err(|_| campfire_db::Error::Other(UNREADABLE_TOKEN_REASON.into()))?;
    if stored_refresh.as_deref() != Some(used) || !current.app_token_expired(tx.now()) {
        return Ok(current.connected() && !current.app_token_expired(tx.now()));
    }
    current.validate_current(tx)?;
    let access = tokens["access_token"].as_str().ok_or_else(|| {
        campfire_db::Error::Other("GitHub returned a non-string access token".into())
    })?;
    let refresh = tokens["refresh_token"]
        .as_str()
        .filter(|token| !blank(token))
        .unwrap_or(used);
    let expiry = tx
        .now()
        .jiff()
        .checked_add(SignedDuration::from_secs(expires_in(
            &tokens["expires_in"],
        )?))
        .map(Timestamp::from_jiff)
        .map_err(|_| {
            campfire_db::Error::Other(
                "GitHub expiry is outside the supported timestamp range".into(),
            )
        })?;
    tx.conn().execute("UPDATE github_connected_accounts SET access_token = ?, refresh_token = ?, token_expires_at = ?, last_error = NULL, updated_at = ? WHERE id = ?", params![crypto.encrypt(access), crypto.encrypt(refresh), expiry, tx.now(), id])?;
    current.claim_verified_login(tx)?;
    Ok(true)
}

fn expires_in(value: &Value) -> Result<i64> {
    match value {
        Value::Null => Ok(0),
        Value::String(s) => Ok(ruby_to_i(s)),
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_f64().map(|v| v.trunc() as i64))
            .ok_or_else(|| {
                campfire_db::Error::Other(
                    "GitHub expiry is outside the supported timestamp range".into(),
                )
            }),
        _ => Err(campfire_db::Error::Other(
            "GitHub returned an invalid expiry".into(),
        )),
    }
}

/// User#deactivate marks the linked account disconnected without revoking its remote grant.
pub fn on_user_deactivation(tx: &mut Tx<'_>, user: &campfire_db::User) -> Result<()> {
    if let Some(account) = Account::for_user(tx.conn(), user.id)? {
        Account::mark_disconnected(tx, account.id, "Account deactivated")?;
    }
    Ok(())
}
