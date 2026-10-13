//! MCP OAuth storage. Endpoint validation and token issuance belong to the authorization server.
use crate::sql::{exists, query_one};
use crate::{AgentCredential, Connection, NewCredential, Result, Timestamp, Tx};
use jiff::SignedDuration;
use rand::RngCore;
use rusqlite::{Row, params};

pub const CONNECTION_LIFETIME: SignedDuration = SignedDuration::from_hours(180 * 24);
pub const ACCESS_LIFETIME: SignedDuration = SignedDuration::from_hours(1);
pub const REFRESH_LIFETIME: SignedDuration = SignedDuration::from_hours(30 * 24);

pub fn canonical_resource(app_url: &str) -> String {
    format!("{}/agents/mcp", app_url.trim_end_matches('/'))
}

#[derive(Debug, Clone)]
pub struct OAuthClient {
    pub id: i64,
    pub client_id: String,
    pub registration: String,
    pub client_name: String,
    pub client_uri: Option<String>,
    pub redirect_uris: String,
    pub auth_methods: String,
    pub jwks_uri: Option<String>,
    pub metadata: Option<String>,
    pub fetched_at: Option<Timestamp>,
    pub refetch_after: Option<Timestamp>,
    pub blocked_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl OAuthClient {
    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM oauth_clients WHERE id=?",
            [id],
            Self::from_row,
        )
    }

    pub fn find_by_client_id(conn: &Connection, client_id: &str) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM oauth_clients WHERE client_id=?",
            [client_id],
            Self::from_row,
        )
    }

    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            client_id: r.get("client_id")?,
            registration: r.get("registration")?,
            client_name: r.get("client_name")?,
            client_uri: r.get("client_uri")?,
            redirect_uris: r.get("redirect_uris")?,
            auth_methods: r.get("auth_methods")?,
            jwks_uri: r.get("jwks_uri")?,
            metadata: r.get("metadata")?,
            fetched_at: r.get("fetched_at")?,
            refetch_after: r.get("refetch_after")?,
            blocked_at: r.get("blocked_at")?,
            created_at: r.get("created_at")?,
            updated_at: r.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OAuthConnection {
    pub id: i64,
    pub agent_credential_id: i64,
    pub oauth_client_id: i64,
    pub resource: String,
    pub scope: String,
    pub auth_method: Option<String>,
    pub expires_at: Timestamp,
    pub created_at: Timestamp,
}

impl OAuthConnection {
    /// Store an owner-approved personal connection with a credential that cannot authenticate
    /// as a static token. Consent/access replacement is the caller's transaction.
    pub fn create(tx: &Tx<'_>, agent_id: i64, owner_id: i64, client_id: i64) -> Result<Self> {
        super::personal_agents::owned(tx.conn(), owner_id, agent_id)?;
        let client = OAuthClient::find(tx.conn(), client_id)?
            .filter(|client| client.blocked_at.is_none())
            .ok_or(crate::Error::RecordNotFound("OAuthClient"))?;
        let mut random = [0; 32];
        rand::rng().fill_bytes(&mut random);
        let expires_at = tx.now().since(CONNECTION_LIFETIME);
        let credential = AgentCredential::create(
            tx,
            NewCredential {
                agent_id,
                created_by_id: owner_id,
                name: client.client_name,
                token_digest: format!("oauth:{}", hex::encode(random)),
                token_last_four: "oauth".into(),
                expires_at: Some(expires_at),
                ..Default::default()
            },
        )?;
        let id = tx.conn().query_row(
            "INSERT INTO oauth_connections(agent_credential_id,oauth_client_id,resource,scope,expires_at,created_at) VALUES(?,?,?,'agent',?,?) RETURNING id",
            params![credential.id, client_id, canonical_resource(&tx.env().default_url_origin), expires_at, tx.now()],
            |r| r.get(0),
        )?;
        Ok(Self::find(tx.conn(), id)?.expect("inserted connection"))
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM oauth_connections WHERE id=?",
            [id],
            |r| {
                Ok(Self {
                    id: r.get("id")?,
                    agent_credential_id: r.get("agent_credential_id")?,
                    oauth_client_id: r.get("oauth_client_id")?,
                    resource: r.get("resource")?,
                    scope: r.get("scope")?,
                    auth_method: r.get("auth_method")?,
                    expires_at: r.get("expires_at")?,
                    created_at: r.get("created_at")?,
                })
            },
        )
    }

    pub fn revoke(&self, tx: &Tx<'_>) -> Result<()> {
        tx.conn().execute(
            "UPDATE agent_credentials SET revoked_at=?,updated_at=? WHERE id=? AND revoked_at IS NULL",
            params![tx.now(), tx.now(), self.agent_credential_id],
        )?;
        tx.conn().execute(
            "UPDATE oauth_tokens SET revoked_at=? WHERE oauth_connection_id=? AND revoked_at IS NULL",
            params![tx.now(), self.id],
        )?;
        Ok(())
    }

    /// Cap both access and refresh expirations at the absolute connection deadline.
    pub fn token_expires_at(&self, now: Timestamp, kind: TokenKind) -> Timestamp {
        now.since(match kind {
            TokenKind::Access => ACCESS_LIFETIME,
            TokenKind::Refresh => REFRESH_LIFETIME,
        })
        .min(self.expires_at)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Access,
    Refresh,
}

#[derive(Debug, Clone)]
pub struct OAuthToken {
    pub id: i64,
    pub oauth_connection_id: i64,
    pub kind: TokenKind,
    pub token_digest: String,
    pub expires_at: Timestamp,
    pub used_at: Option<Timestamp>,
    pub replaced_by_id: Option<i64>,
    pub revoked_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshUse {
    Consumed,
    Reused,
    Invalid,
}

impl OAuthToken {
    pub fn find_by_secret(conn: &Connection, secret: &str) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM oauth_tokens WHERE token_digest=?",
            [crate::user::digest_bot_token(secret)],
            |r| {
                let kind: String = r.get("kind")?;
                Ok(Self {
                    id: r.get("id")?,
                    oauth_connection_id: r.get("oauth_connection_id")?,
                    kind: if kind == "access" {
                        TokenKind::Access
                    } else {
                        TokenKind::Refresh
                    },
                    token_digest: r.get("token_digest")?,
                    expires_at: r.get("expires_at")?,
                    used_at: r.get("used_at")?,
                    replaced_by_id: r.get("replaced_by_id")?,
                    revoked_at: r.get("revoked_at")?,
                    created_at: r.get("created_at")?,
                })
            },
        )
    }

    pub fn authenticate(
        conn: &Connection,
        secret: &str,
        now: Timestamp,
        app_url: &str,
    ) -> Result<Option<AgentCredential>> {
        let Some(token) = Self::find_by_secret(conn, secret)? else {
            return Ok(None);
        };
        if token.kind != TokenKind::Access || token.expires_at <= now || token.revoked_at.is_some()
        {
            return Ok(None);
        }
        let Some(connection) = OAuthConnection::find(conn, token.oauth_connection_id)? else {
            return Ok(None);
        };
        if connection.expires_at <= now
            || connection.resource != canonical_resource(app_url)
            || connection.scope != "agent"
            || !exists(
                conn,
                "SELECT 1 FROM oauth_clients WHERE id=? AND blocked_at IS NULL",
                [connection.oauth_client_id],
            )?
        {
            return Ok(None);
        }
        Ok(AgentCredential::find(conn, connection.agent_credential_id)?
            .filter(|credential| credential.active(now)))
    }

    /// The caller validates client, auth method and resource before consumption. A used digest
    /// remains a replay even after its expiry; the writer transaction serializes concurrent uses.
    pub fn consume_refresh(tx: &Tx<'_>, secret: &str, connection_id: i64) -> Result<RefreshUse> {
        let Some(token) = Self::find_by_secret(tx.conn(), secret)? else {
            return Ok(RefreshUse::Invalid);
        };
        if token.kind != TokenKind::Refresh || token.oauth_connection_id != connection_id {
            return Ok(RefreshUse::Invalid);
        }
        let Some(connection) = OAuthConnection::find(tx.conn(), connection_id)? else {
            return Ok(RefreshUse::Invalid);
        };
        if token.used_at.is_some() {
            connection.revoke(tx)?;
            return Ok(RefreshUse::Reused);
        }
        if token.revoked_at.is_some()
            || token.expires_at <= tx.now()
            || connection.expires_at <= tx.now()
            || AgentCredential::find(tx.conn(), connection.agent_credential_id)?
                .is_none_or(|credential| !credential.active(tx.now()))
        {
            return Ok(RefreshUse::Invalid);
        }
        let changed = tx.conn().execute(
            "UPDATE oauth_tokens SET used_at=? WHERE id=? AND used_at IS NULL",
            params![tx.now(), token.id],
        )?;
        if changed == 0 {
            connection.revoke(tx)?;
            return Ok(RefreshUse::Reused);
        }
        Ok(RefreshUse::Consumed)
    }
}

/// A dead connection no longer authenticates, but retains its family until every token expires.
fn dead_connection_sql(connection: &str) -> String {
    format!(
        "({connection}.expires_at<=?1 OR EXISTS (SELECT 1 FROM agent_credentials sweep_credential WHERE sweep_credential.id={connection}.agent_credential_id AND sweep_credential.revoked_at IS NOT NULL))"
    )
}

pub fn sweep(tx: &Tx<'_>) -> Result<()> {
    // Bot-user hard destruction deliberately preserves the legacy credential rows.
    tx.conn().execute("DELETE FROM oauth_connections WHERE NOT EXISTS (SELECT 1 FROM agent_credentials credential JOIN agents agent ON agent.id=credential.agent_id WHERE credential.id=oauth_connections.agent_credential_id)", [])?;
    let dead = dead_connection_sql("connection");
    tx.conn().execute(
        &format!("DELETE FROM oauth_tokens WHERE oauth_connection_id IN (SELECT connection.id FROM oauth_connections connection WHERE {dead} AND NOT EXISTS (SELECT 1 FROM oauth_tokens family WHERE family.oauth_connection_id=connection.id AND family.expires_at>?1))"),
        [tx.now()],
    )?;
    // Linked code digests are replay tombstones as well. Pending/denied requests have no family.
    tx.conn().execute(
        &format!("DELETE FROM oauth_authorizations WHERE expires_at<=?1 AND (oauth_connection_id IS NULL OR oauth_connection_id IN (SELECT connection.id FROM oauth_connections connection WHERE {dead} AND NOT EXISTS (SELECT 1 FROM oauth_tokens family WHERE family.oauth_connection_id=connection.id AND family.expires_at>?1)))"),
        [tx.now()],
    )?;
    Ok(())
}
