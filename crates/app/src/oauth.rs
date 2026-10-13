//! OAuth authorization-server operations. Public protocol refusals commit replay revocation.
pub mod client;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use campfire_db::{
    AgentCredential, Connection, Result, Timestamp, Tx,
    models::{
        agent_access,
        audit_log::{AuditLog, Context, NewAuditLog, Target},
        oauth::{
            OAuthClient, OAuthConnection, OAuthToken, RefreshUse, TokenKind, canonical_resource,
        },
    },
};
use rusqlite::{OptionalExtension, Params, Row, params};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub fn secret() -> String {
    URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
}

#[derive(Debug, Clone)]
pub struct Authorization {
    pub id: i64,
    pub public_id: String,
    pub oauth_client_id: i64,
    pub user_id: i64,
    pub redirect_uri: String,
    pub state: Option<String>,
    pub code_challenge: String,
    pub resource: String,
    pub status: String,
    pub oauth_connection_id: Option<i64>,
    pub expires_at: Timestamp,
}
impl Authorization {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get("id")?,
            public_id: r.get("public_id")?,
            oauth_client_id: r.get("oauth_client_id")?,
            user_id: r.get("user_id")?,
            redirect_uri: r.get("redirect_uri")?,
            state: r.get("state")?,
            code_challenge: r.get("code_challenge")?,
            resource: r.get("resource")?,
            status: r.get("status")?,
            oauth_connection_id: r.get("oauth_connection_id")?,
            expires_at: r.get("expires_at")?,
        })
    }
    pub fn find(conn: &Connection, public_id: &str) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM oauth_authorizations WHERE public_id=?",
            [public_id],
            Self::from_row,
        )
    }
    pub fn pending(
        conn: &Connection,
        public_id: &str,
        user_id: i64,
        now: Timestamp,
    ) -> Result<Self> {
        let auth = Self::find(conn, public_id)?
            .filter(|a| a.user_id == user_id && a.status == "pending" && a.expires_at > now)
            .ok_or(campfire_db::Error::RecordNotFound("OAuthAuthorization"))?;
        OAuthClient::find(conn, auth.oauth_client_id)?
            .filter(|client| client.blocked_at.is_none() && client::allows_none(client))
            .ok_or(campfire_db::Error::RecordNotFound("OAuthClient"))?;
        Ok(auth)
    }
}

pub fn callback(
    redirect: &str,
    state: Option<&str>,
    issuer: &str,
    key: &str,
    value: &str,
) -> String {
    let mut url = url::Url::parse(redirect).expect("validated redirect");
    url.query_pairs_mut()
        .append_pair(key, value)
        .append_pair("iss", issuer);
    if let Some(state) = state {
        url.query_pairs_mut().append_pair("state", state);
    }
    url.into()
}

#[derive(Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: &'static str,
    pub scope: &'static str,
}
fn issue(tx: &Tx<'_>, connection: &OAuthConnection) -> Result<TokenPair> {
    let access = secret();
    let refresh = secret();
    for (value, kind, name) in [
        (&access, TokenKind::Access, "access"),
        (&refresh, TokenKind::Refresh, "refresh"),
    ] {
        tx.conn().execute("INSERT INTO oauth_tokens(oauth_connection_id,kind,token_digest,expires_at,created_at) VALUES(?,?,?,?,?)",params![connection.id,name,campfire_db::user::digest_bot_token(value),connection.token_expires_at(tx.now(),kind),tx.now()])?;
    }
    Ok(TokenPair {
        access_token: access,
        refresh_token: refresh,
        expires_in: connection
            .token_expires_at(tx.now(), TokenKind::Access)
            .as_second()
            - tx.now().as_second(),
        token_type: "Bearer",
        scope: "agent",
    })
}
fn live(tx: &Tx<'_>, connection: &OAuthConnection) -> Result<bool> {
    let Some(credential) = AgentCredential::find(tx.conn(), connection.agent_credential_id)?
        .filter(|c| c.active(tx.now()))
    else {
        return Ok(false);
    };
    Ok(connection.expires_at > tx.now()
        && connection.resource == canonical_resource(&tx.env().default_url_origin)
        && connection.scope == "agent"
        && exists(
            tx.conn(),
            &format!(
                "SELECT 1 FROM agents a WHERE a.id=? AND a.kind='personal' AND ({})",
                agent_access::agent_live("a")
            ),
            [credential.agent_id],
        )?)
}
fn client_valid(tx: &Tx<'_>, connection: &OAuthConnection, client_id: &str) -> Result<bool> {
    Ok(
        OAuthClient::find(tx.conn(), connection.oauth_client_id)?.is_some_and(|client| {
            client.client_id == client_id
                && client.blocked_at.is_none()
                && client::allows_none(&client)
        }),
    )
}
pub fn audit(
    tx: &Tx<'_>,
    connection: &OAuthConnection,
    action: &str,
    context: &Context,
) -> Result<()> {
    AuditLog::record(
        tx,
        NewAuditLog {
            action: action.into(),
            target: Some(Target {
                record_type: "AgentCredential".into(),
                id: connection.agent_credential_id,
                label: None,
            }),
            changes: Some(json!({"connection_id":connection.id})),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

pub struct CodeExchange<'a> {
    pub code: &'a str,
    pub client_id: &'a str,
    pub redirect_uri: &'a str,
    pub verifier: &'a str,
    pub resource: &'a str,
    pub method_none: bool,
}
pub fn exchange_code(tx: &mut Tx<'_>, input: CodeExchange<'_>) -> Result<Option<TokenPair>> {
    let auth = query_one(
        tx.conn(),
        "SELECT * FROM oauth_authorizations WHERE code_digest=?",
        [campfire_db::user::digest_bot_token(input.code)],
        Authorization::from_row,
    )?;
    let Some(auth) = auth else { return Ok(None) };
    let Some(connection) = auth
        .oauth_connection_id
        .map(|id| OAuthConnection::find(tx.conn(), id))
        .transpose()?
        .flatten()
    else {
        return Ok(None);
    };
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(input.verifier));
    let verifier_valid = (43..=128).contains(&input.verifier.len())
        && input
            .verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
        && bool::from(challenge.as_bytes().ct_eq(auth.code_challenge.as_bytes()));
    if !input.method_none
        || !client_valid(tx, &connection, input.client_id)?
        || auth.redirect_uri != input.redirect_uri
        || auth.resource != input.resource
        || !verifier_valid
    {
        return Ok(None);
    };
    if auth.status == "exchanged" {
        connection.revoke(tx)?;
        audit(tx, &connection, "oauth.code_reuse", &Context::default())?;
        return Ok(None);
    }
    if auth.status != "approved" || auth.expires_at <= tx.now() || !live(tx, &connection)? {
        return Ok(None);
    }
    tx.conn().execute("UPDATE oauth_authorizations SET status='exchanged',updated_at=? WHERE id=? AND status='approved'",params![tx.now(),auth.id])?;
    tx.conn().execute(
        "UPDATE oauth_connections SET auth_method='none' WHERE id=?",
        [connection.id],
    )?;
    issue(tx, &connection).map(Some)
}

pub fn refresh(
    tx: &mut Tx<'_>,
    secret: &str,
    client_id: &str,
    resource: Option<&str>,
    method_none: bool,
) -> Result<Option<TokenPair>> {
    let Some(token) = OAuthToken::find_by_secret(tx.conn(), secret)?
        .filter(|token| token.kind == TokenKind::Refresh)
    else {
        return Ok(None);
    };
    let Some(connection) = OAuthConnection::find(tx.conn(), token.oauth_connection_id)? else {
        return Ok(None);
    };
    // Validate all bindings before consumption, including on a replay.
    if !method_none
        || connection.auth_method.as_deref() != Some("none")
        || !client_valid(tx, &connection, client_id)?
        || resource.is_some_and(|resource| resource != connection.resource)
        || connection.resource != canonical_resource(&tx.env().default_url_origin)
    {
        return Ok(None);
    }
    if connection.expires_at <= tx.now() || (token.used_at.is_none() && !live(tx, &connection)?) {
        return Ok(None);
    }
    match OAuthToken::consume_refresh(tx, secret, connection.id)? {
        RefreshUse::Invalid => Ok(None),
        RefreshUse::Reused => {
            // Record and notify once, even if this tombstone is presented repeatedly.
            if !exists(
                tx.conn(),
                "SELECT 1 FROM audit_logs WHERE action='oauth.refresh_reuse' AND target_type='AgentCredential' AND target_id=?",
                [connection.agent_credential_id],
            )? {
                audit(tx, &connection, "oauth.refresh_reuse", &Context::default())?;
                let credential = AgentCredential::find(tx.conn(), connection.agent_credential_id)?
                    .ok_or(campfire_db::Error::RecordNotFound("AgentCredential"))?;
                let agent = campfire_db::Agent::find(tx.conn(), credential.agent_id)?
                    .ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
                if let Some(owner_id) = agent.owner_id {
                    tx.emit_after_commit(campfire_db::Event::job(
                        &campfire_mail::jobs::DeliveryJob {
                            notification: campfire_mail::jobs::Notification::OAuthRefreshReuse {
                                user_id: owner_id,
                            },
                        },
                    ));
                }
            }
            Ok(None)
        }
        RefreshUse::Consumed => {
            let pair = issue(tx, &connection)?;
            let replacement =
                OAuthToken::find_by_secret(tx.conn(), &pair.refresh_token)?.expect("issued token");
            tx.conn().execute(
                "UPDATE oauth_tokens SET replaced_by_id=? WHERE id=?",
                params![replacement.id, token.id],
            )?;
            Ok(Some(pair))
        }
    }
}

pub fn revoke(tx: &mut Tx<'_>, secret: &str, client_id: &str, method_none: bool) -> Result<()> {
    let Some(token) = OAuthToken::find_by_secret(tx.conn(), secret)? else {
        return Ok(());
    };
    let Some(connection) = OAuthConnection::find(tx.conn(), token.oauth_connection_id)? else {
        return Ok(());
    };
    if !method_none || !client_valid(tx, &connection, client_id)? {
        return Ok(());
    }
    if token.kind == TokenKind::Access {
        tx.conn().execute(
            "UPDATE oauth_tokens SET revoked_at=? WHERE id=?",
            params![tx.now(), token.id],
        )?;
    } else {
        connection.revoke(tx)?;
        audit(tx, &connection, "oauth.revoke", &Context::default())?;
    }
    Ok(())
}

fn query_one<T>(
    conn: &Connection,
    sql: &str,
    params: impl Params,
    map: impl FnOnce(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Option<T>> {
    Ok(conn.query_row(sql, params, map).optional()?)
}
fn exists(conn: &Connection, sql: &str, params: impl Params) -> Result<bool> {
    Ok(conn.query_row(&format!("SELECT EXISTS({sql})"), params, |r| r.get(0))?)
}
pub fn valid_challenge(value: &str) -> bool {
    URL_SAFE_NO_PAD
        .decode(value)
        .is_ok_and(|bytes| bytes.len() == 32 && URL_SAFE_NO_PAD.encode(bytes) == value)
}
pub fn form_pairs(bytes: &[u8]) -> Vec<(String, String)> {
    url::form_urlencoded::parse(bytes).into_owned().collect()
}
