//! MCP OAuth routes, mounted only while the authorization server is available.
use axum::{
    Router,
    routing::{get, post},
};
use campfire_api_types as api;
use campfire_app::{
    app::AppCtx,
    oauth::{self, Authorization, assertion, client, registration::Registration},
};
use campfire_db::{
    NewUser, Timestamp, User,
    models::{
        oauth::{OAuthClient, OAuthConnection, canonical_resource},
        personal_agents,
    },
};
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode, action, unparsed_action};
use campfire_runtime::concerns::{self, Before};
use rusqlite::{OptionalExtension, params};
use serde_json::json;
use std::collections::BTreeMap;

pub fn routes(enabled: bool) -> Router<Kit> {
    if !enabled {
        return Router::new();
    }
    Router::new()
        .route(
            "/.well-known/oauth-authorization-server",
            get(action(metadata)).options(action(preflight)),
        )
        .route(
            "/.well-known/oauth-protected-resource",
            get(action(resource_metadata)).options(action(preflight)),
        )
        .route(
            "/.well-known/oauth-protected-resource/agents/mcp",
            get(action(resource_metadata)).options(action(preflight)),
        )
        .route(
            "/oauth/register",
            post(unparsed_action(register)).options(action(preflight)),
        )
        .route("/oauth/authorize", get(action(authorize)))
        .route(
            "/oauth/token",
            post(unparsed_action(token)).options(action(preflight)),
        )
        .route(
            "/oauth/revoke",
            post(unparsed_action(revoke)).options(action(preflight)),
        )
        .route(
            "/api/v1/oauth/authorizations/{public_id}",
            get(action(consent)),
        )
        .route(
            "/api/v1/oauth/authorizations/{public_id}/approve",
            post(unparsed_action(approve)),
        )
        .route(
            "/api/v1/oauth/authorizations/{public_id}/deny",
            post(action(deny)),
        )
}
/// The administrators' list of connected clients and their block switch. Mounted whatever the
/// gate says, so a client can still be blocked while new connections are off.
pub fn admin_routes() -> Router<Kit> {
    Router::new()
        .route("/api/v1/admin/oauth_clients", get(action(clients)))
        .route(
            "/api/v1/admin/oauth_clients/{id}/block",
            axum::routing::put(action(block)).delete(action(unblock)),
        )
}
endpoint!(
    /// `GET /api/v1/admin/oauth_clients`
    clients => index_clients
);
endpoint!(
    /// `PUT /api/v1/admin/oauth_clients/:id/block`
    block => block_client
);
endpoint!(
    /// `DELETE /api/v1/admin/oauth_clients/:id/block`
    unblock => unblock_client
);
const CLIENT_SQL: &str = "SELECT client.id, client.client_name, client.client_id, client.blocked_at, (SELECT COUNT(*) FROM oauth_connections connection JOIN agent_credentials credential ON credential.id=connection.agent_credential_id WHERE connection.oauth_client_id=client.id AND credential.revoked_at IS NULL AND connection.expires_at>?1) AS live, (SELECT MAX(connection.created_at) FROM oauth_connections connection WHERE connection.oauth_client_id=client.id) AS last_connected_at FROM oauth_clients client";
fn admin_client(r: &rusqlite::Row<'_>) -> rusqlite::Result<api::AdminOAuthClient> {
    let client_id: String = r.get(2)?;
    Ok(api::AdminOAuthClient {
        id: r.get(0)?,
        client_name: r.get(1)?,
        client_host: client::client_host(&client_id),
        client_id,
        blocked_at: r.get::<_, Option<Timestamp>>(3)?.map(crate::dto::time),
        live_connections: r.get(4)?,
        last_connected_at: r.get::<_, Option<Timestamp>>(5)?.map(crate::dto::time),
    })
}
async fn index_clients(c: &mut Ctx) -> Result {
    crate::admin::administrator(c).await?;
    let oauth_available = c.app().config.oauth_issuer().is_some();
    let now = Timestamp::from_jiff(c.now());
    let clients = c
        .app()
        .db
        .read(move |conn| {
            let sql = format!("{CLIENT_SQL} WHERE client.blocked_at IS NOT NULL OR EXISTS (SELECT 1 FROM oauth_connections connection WHERE connection.oauth_client_id=client.id) ORDER BY client.blocked_at IS NULL, last_connected_at DESC, client.id DESC");
            let mut statement = conn.prepare(&sql)?;
            let rows = statement.query_map([now], admin_client)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
        .map_err(campfire_runtime::context::db_error)?;
    c.json(
        StatusCode::OK,
        &api::AdminOAuthClientList {
            clients,
            oauth_available,
        },
    )
}
async fn block_client(c: &mut Ctx) -> Result {
    set_blocked(c, true).await
}
async fn unblock_client(c: &mut Ctx) -> Result {
    set_blocked(c, false).await
}
/// Blocking stops new authorizations and every token of the client's connections; unblocking
/// lets the unexpired, unrevoked ones work again. Neither revokes anything.
async fn set_blocked(c: &mut Ctx, blocked: bool) -> Result {
    crate::admin::administrator(c).await?;
    let id = c
        .param_str("id")
        .and_then(|id| id.parse::<i64>().ok())
        .ok_or(Error::NotFound)?;
    let context = campfire_people::controllers::two_factor::audit_context(c)?;
    let client = c
        .app()
        .db
        .write(move |tx| {
            let client = OAuthClient::find(tx.conn(), id)?
                .ok_or(campfire_db::Error::RecordNotFound("OAuthClient"))?;
            if client.blocked_at.is_some() != blocked {
                let at = blocked.then(|| tx.now());
                tx.conn().execute(
                    "UPDATE oauth_clients SET blocked_at=?,updated_at=? WHERE id=?",
                    params![at, tx.now(), id],
                )?;
                campfire_db::models::audit_log::AuditLog::record(
                    tx,
                    campfire_db::models::audit_log::NewAuditLog {
                        action: if blocked { "oauth.client.block" } else { "oauth.client.unblock" }.into(),
                        target: Some(campfire_db::models::audit_log::Target {
                            record_type: "OAuthClient".into(),
                            id,
                            label: Some(client.client_name),
                        }),
                        changes: Some(json!({"client_id":client.client_id})),
                        ..Default::default()
                    },
                    &context,
                )?;
            }
            let sql = format!("{CLIENT_SQL} WHERE client.id=?2");
            Ok(tx.conn().query_row(&sql, params![tx.now(), id], admin_client)?)
        })
        .await
        .map_err(campfire_runtime::context::db_error)?;
    c.json(StatusCode::OK, &client)
}
fn issuer(c: &Ctx) -> String {
    c.app().config.oauth_issuer().expect("gated route")
}
fn public(c: &mut Ctx) {
    c.no_store();
    c.set_header("access-control-allow-origin", "*");
}
async fn preflight(c: &mut Ctx) -> Result {
    public(c);
    c.set_header("access-control-allow-methods", "GET, POST, OPTIONS");
    c.set_header(
        "access-control-allow-headers",
        "Content-Type, Authorization",
    );
    Ok(c.head(StatusCode::NO_CONTENT))
}
async fn metadata(c: &mut Ctx) -> Result {
    public(c);
    let issuer = issuer(c);
    c.json(StatusCode::OK,&json!({"issuer":issuer,"authorization_endpoint":format!("{issuer}/oauth/authorize"),"token_endpoint":format!("{issuer}/oauth/token"),"revocation_endpoint":format!("{issuer}/oauth/revoke"),"registration_endpoint":format!("{issuer}/oauth/register"),"response_types_supported":["code"],"grant_types_supported":["authorization_code","refresh_token"],"code_challenge_methods_supported":["S256"],"token_endpoint_auth_methods_supported":["none","private_key_jwt"],"token_endpoint_auth_signing_alg_values_supported":["RS256","ES256"],"revocation_endpoint_auth_methods_supported":["none","private_key_jwt"],"revocation_endpoint_auth_signing_alg_values_supported":["RS256","ES256"],"client_id_metadata_document_supported":true,"authorization_response_iss_parameter_supported":true,"scopes_supported":["agent"]}))
}
async fn resource_metadata(c: &mut Ctx) -> Result {
    public(c);
    let issuer = issuer(c);
    c.json(StatusCode::OK,&json!({"resource":canonical_resource(&issuer),"authorization_servers":[issuer],"scopes_supported":["agent"],"bearer_methods_supported":["header"],"resource_name":"Smartfire agents"}))
}
fn throttle(c: &mut Ctx, key: &str, limit: u64) -> Result<()> {
    throttle_window(c, key, limit, 60)
}
fn throttle_window(c: &mut Ctx, key: &str, limit: u64, seconds: i64) -> Result<()> {
    let now = c.now();
    let bucket = now.as_second().div_euclid(seconds);
    let key = format!(
        "oauth:{bucket}:{}",
        campfire_db::user::digest_bot_token(key)
    );
    if c.kit()
        .rate_limits()
        .increment(&key, jiff::SignedDuration::from_secs(seconds + 5), now)
        > limit
    {
        c.set_header(
            "retry-after",
            &(((bucket + 1) * seconds - now.as_second()).max(1)).to_string(),
        );
        return campfire_kit::halt(c.json(
            StatusCode::TOO_MANY_REQUESTS,
            &json!({"error":"rate_limited"}),
        )?);
    }
    Ok(())
}
fn error_page(c: &mut Ctx, error: &str) -> Result {
    c.no_store();
    c.set_header("content-security-policy", "frame-ancestors 'none'");
    // Only a server-owned error code enters this URL; no untrusted redirect or remote logo.
    c.redirect_to(&format!("/app/oauth/error?error={error}"))
}
fn authorization_error(
    c: &mut Ctx,
    redirect: &str,
    state: Option<&str>,
    error: &str,
    description: Option<&str>,
) -> Result {
    c.redirect_to_with(
        &oauth::error_callback(redirect, state, &issuer(c), error, description),
        campfire_kit::Redirect {
            allow_other_host: true,
            ..Default::default()
        },
    )
}
async fn authorize(c: &mut Ctx) -> Result {
    c.no_store();
    let overflow = error_page(c, "invalid_request")?;
    c.on_session_cookie_overflow(overflow);
    let client_id = c.param_str("client_id").unwrap_or_default().to_string();
    let app = c.app().clone();
    let client = match client::identify(&app, &client_id, || throttle(c, "cimd-global", 30)).await {
        Ok(client) => client,
        Err(Error::Halt(response)) => return Err(Error::Halt(response)),
        Err(_) => return error_page(c, "invalid_client"),
    };
    if !client::has_supported_method(&client) {
        return error_page(c, "unsupported_client_authentication");
    }
    let redirect = c.param_str("redirect_uri").unwrap_or_default().to_string();
    let registered: Vec<String> =
        serde_json::from_str(&client.redirect_uris).map_err(Error::internal)?;
    if !registered
        .iter()
        .any(|uri| client::redirect_matches(uri, &redirect))
    {
        return error_page(c, "invalid_redirect_uri");
    }
    let state = c.param_str("state").map(str::to_owned);
    let overflow = authorization_error(c, &redirect, state.as_deref(), "invalid_request", None)?;
    c.on_session_cookie_overflow(overflow);
    let resource = canonical_resource(&issuer(c));
    let challenge = c
        .param_str("code_challenge")
        .unwrap_or_default()
        .to_string();
    let bad = if c.param_str("response_type") != Some("code") {
        Some("unsupported_response_type")
    } else if c.param_str("code_challenge_method") != Some("S256")
        || !oauth::valid_challenge(&challenge)
    {
        Some("invalid_request")
    } else if c
        .param_str("resource")
        .is_some_and(|value| value.trim_end_matches('/') != resource)
    {
        Some("invalid_target")
    } else if c
        .param_str("scope")
        .is_some_and(|scope| scope.split_whitespace().any(|scope| scope != "agent"))
    {
        Some("invalid_scope")
    } else {
        None
    };
    if let Some(error) = bad {
        return authorization_error(c, &redirect, state.as_deref(), error, None);
    }
    let return_url = oauth::AuthorizationReturn {
        client_id: &client_id,
        redirect_uri: &redirect,
        code_challenge: &challenge,
        resource: &resource,
        state: state.as_deref(),
    }
    .url(&c.url_for("/oauth/authorize"));
    let Ok(return_url) = return_url else {
        return authorization_error(c, &redirect, state.as_deref(), "invalid_request", None);
    };
    // Count exactly the Rails JSON string the session serializer will encode,
    // including HTML escaping, before encryption and the metadata's Base64 layers.
    if rails_compat::json_encode(&json!(&return_url)).len() > 1024 {
        return authorization_error(c, &redirect, state.as_deref(), "invalid_request", None);
    }
    c.set_current(concerns::AuthenticationReturnUrl(return_url));
    let before = concerns::before_actions(c, Before::default()).await;
    if matches!(&before, Err(Error::Halt(_))) {
        // Include the sign-in page's CSRF now and reserve wire bytes for flash and
        // pending second-factor fields. The final write still has an overflow refusal.
        c.form_authenticity_token();
        if !c.session_cookie_fits_with_headroom(1024) {
            c.reset_session();
            return authorization_error(c, &redirect, state.as_deref(), "invalid_request", None);
        }
    }
    before?;
    let owner = concerns::require_current_user(c)?.clone();
    if !owner.is_active() || owner.is_bot() {
        return authorization_error(c, &redirect, state.as_deref(), "access_denied", None);
    }
    throttle(c, &format!("authorize:{}", owner.id), 30)?;
    let public_id = oauth::secret();
    let id = public_id.clone();
    let error_redirect = redirect.clone();
    let error_state = state.clone();
    let authorization = c.app().db.write(move |tx|{
        crate::me_agents::enabled(tx.conn())?;
        let client=OAuthClient::find(tx.conn(),client.id)?.filter(|client|client.blocked_at.is_none()).ok_or(campfire_db::Error::RecordNotFound("OAuthClient"))?;
        let sql = "INSERT INTO oauth_authorizations(public_id,oauth_client_id,user_id,redirect_uri,state,code_challenge,scope,resource,expires_at,created_at,updated_at) VALUES(?,?,?,?,?,?,'agent',?,?,?,?)";
        tx.conn().execute(sql, params![id,client.id,owner.id,redirect,state,challenge,resource,tx.now().since(jiff::SignedDuration::from_secs(600)),tx.now(),tx.now()])?;Ok(())
    }).await;
    match authorization {
        Ok(()) => {}
        Err(campfire_db::Error::Other(message)) if message == crate::me_agents::DISABLED => {
            return authorization_error(
                c,
                &error_redirect,
                error_state.as_deref(),
                "access_denied",
                Some(crate::me_agents::DISABLED_MESSAGE),
            );
        }
        Err(error) => return Err(crate::me_agents::saved_error(error)),
    }

    c.redirect_to(&format!("/app/oauth/consent/{public_id}"))
}
fn public_id(c: &Ctx) -> String {
    c.param_str("public_id").unwrap_or_default().to_owned()
}
endpoint!(consent=>show_consent);
endpoint!(approve=>approve_consent);
endpoint!(deny=>deny_consent);
fn protect_consent(c: &mut Ctx) {
    c.no_store();
    c.set_header("content-security-policy", "frame-ancestors 'none'");
}
async fn show_consent(c: &mut Ctx) -> Result {
    protect_consent(c);
    let owner = crate::me_agents::member(c).await?;
    let id = public_id(c);
    let now = Timestamp::from_jiff(c.now());
    let response = c.app().db.read(move |conn| {
        let auth = Authorization::pending(conn, &id, owner.id, now)?;
        let client = OAuthClient::find(conn, auth.oauth_client_id)?.expect("authorization client");
        let url = client::redirect_url(&auth.redirect_uri).expect("validated redirect");
        let settings = campfire_db::Account::first(conn)?
            .ok_or(campfire_db::Error::RecordNotFound("Account"))?.settings();
        let agents = personal_agents::for_owner(conn, owner.id)?
            .into_iter().map(|agent| crate::me_agents::present(conn, agent, now))
            .collect::<campfire_db::Result<Vec<_>>>()?;
        let mut room_access = Vec::new();
        let mut hidden_rooms = Vec::new();
        for agent in &agents {
            let (access, count) = self::room_access(conn, owner.id, agent)?;
            room_access.extend(access);
            if count > 0 {
                hidden_rooms.push(api::OAuthAgentHiddenRooms { agent_id: agent.id, count });
            }
        }
        let last_sql = "SELECT credential.agent_id FROM oauth_connections connection JOIN agent_credentials credential ON credential.id=connection.agent_credential_id JOIN agents agent ON agent.id=credential.agent_id WHERE connection.oauth_client_id=? AND agent.owner_id=? ORDER BY connection.created_at DESC,connection.id DESC LIMIT 1";
        let last: Option<i64> = conn.query_row(last_sql, params![client.id, owner.id], |r| r.get(0)).optional()?;
        let selected_agent_id = last.filter(|id| agents.iter().any(|agent| agent.id == *id));
        let rooms = personal_agents::grantable_rooms(conn, owner.id)?
            .into_iter().map(|room| api::PersonalAgentRoom {
                id: room.id, name: room.name, private: room.private,
            }).collect();
        Ok(api::OAuthConsent {
            client_name: client.client_name,
            redirect_hostname: url.host_str().expect("redirect host").into(),
            badge: if client::loopback(&url) {
                api::OAuthClientBadge::AppOnThisComputer
            } else {
                api::OAuthClientBadge::UnverifiedApp
            },
            agents, selected_agent_id, rooms,
            capabilities: campfire_db::models::agent_access::PERSONAL_CAPABILITIES.map(str::to_string).to_vec(),
            enabled: settings.member_agents_enabled(),
            limit: (!owner.is_administrator()).then(|| settings.member_agent_limit()),
            access_applies_to_all_connections: true,
            room_access,
            hidden_rooms,
        })
    }).await.map_err(campfire_runtime::context::db_error)?;
    c.json(StatusCode::OK, &response)
}
/// Each room `agent` is in or holds capabilities in, with what it can do there: everything
/// approval would replace. Direct and deleted rooms are left alone by approval, so they're skipped.
/// Rooms `owner_id` can't see any more (`user.rooms`) are only counted: their names and ids stay
/// private, though approval takes the agent out of them too.
fn room_access(
    conn: &rusqlite::Connection,
    owner_id: i64,
    agent: &api::PersonalAgent,
) -> campfire_db::Result<(Vec<api::OAuthAgentRoomAccess>, i64)> {
    let mut rooms: BTreeMap<i64, Vec<String>> = campfire_db::Membership::for_user(conn, agent.user_id)?
        .into_iter()
        .map(|membership| (membership.room_id, Vec::new()))
        .collect();
    for grant in campfire_db::AgentGrant::effective_for_agent(conn, agent.id)? {
        if let Some(room_id) = grant.room_id {
            let capabilities = rooms.entry(room_id).or_default();
            if !capabilities.contains(&grant.capability) {
                capabilities.push(grant.capability);
            }
        }
    }
    let mut access = Vec::new();
    let mut hidden = 0;
    for (room_id, mut capabilities) in rooms {
        let Some(room) = campfire_db::Room::find_by_id(conn, room_id)? else { continue };
        if room.direct() || room.deleted_at.is_some() {
            continue;
        }
        if campfire_db::Room::find_for_user(conn, owner_id, room_id)?.is_none() {
            hidden += 1;
            continue;
        }
        capabilities.sort();
        access.push(api::OAuthAgentRoomAccess {
            agent_id: agent.id, room_id, room_name: room.name.unwrap_or_default(), capabilities,
        });
    }
    Ok((access, hidden))
}
async fn approve_consent(c: &mut Ctx) -> Result {
    protect_consent(c);
    let owner = crate::me_agents::member(c).await?;
    let id = public_id(c);
    let owner_id = owner.id;
    let now = Timestamp::from_jiff(c.now());
    c.app()
        .db
        .read(move |conn| Authorization::pending(conn, &id, owner_id, now))
        .await
        .map_err(campfire_runtime::context::db_error)?;
    throttle(c, &format!("authorize:{}", owner.id), 30)?;
    crate::admin::require_sudo(c).await?;
    let input: api::ApproveOAuthAuthorization = crate::admin::body(c).await?;
    let id = public_id(c);
    let issuer = issuer(c);
    let context = campfire_people::controllers::two_factor::audit_context(c)?;
    let redirect = c.app().db.write(move |tx| {
        crate::me_agents::enabled(tx.conn())?;
        let owner = User::find(tx.conn(), owner.id)?;
        if !owner.is_active() || owner.is_bot() {
            return Err(campfire_db::Error::RecordNotFound("User"));
        }
        let auth = Authorization::pending(tx.conn(), &id, owner.id, tx.now())?;
        if auth.resource != canonical_resource(&issuer) {
            return Err(campfire_db::Error::RecordNotFound("OAuthAuthorization"));
        }
        let agent = match input.agent_id {
            Some(id) => personal_agents::owned(tx.conn(), owner.id, id)?,
            None => {
                let settings = campfire_db::Account::first_in(tx)?.settings();
                if !owner.is_administrator()
                    && personal_agents::for_owner(tx.conn(), owner.id)?.len() as i64 >= settings.member_agent_limit()
                {
                    return Err(campfire_db::Error::Other(crate::me_agents::LIMIT.into()));
                }
                let agent = personal_agents::create(tx, NewUser {
                    name: input.name.unwrap_or_default(), icon_name: input.icon_name,
                    ..Default::default()
                }, owner.id)?;
                crate::me_agents::audit(tx, &agent, "agent.create", json!({"owner_id":owner.id}), &context)?;
                agent
            }
        };
        personal_agents::replace_access(tx, &agent, owner.id, &input.room_ids, &input.capabilities)?;
        crate::me_agents::audit(tx, &agent, "agent.access.update", json!({"room_ids":input.room_ids,"capabilities":input.capabilities}), &context)?;
        let connection = OAuthConnection::create(tx, agent.id, owner.id, auth.oauth_client_id)?;
        let code = oauth::secret();
        let sql = "UPDATE oauth_authorizations SET status='approved',code_digest=?,oauth_connection_id=?,expires_at=?,updated_at=? WHERE id=? AND status='pending'";
        tx.conn().execute(sql, params![
            campfire_db::user::digest_bot_token(&code), connection.id,
            tx.now().since(jiff::SignedDuration::from_secs(60)), tx.now(), auth.id,
        ])?;
        oauth::audit(tx, &connection, "oauth.connect", &context)?;
        Ok(oauth::callback(&auth.redirect_uri, auth.state.as_deref(), &issuer, "code", &code))
    }).await.map_err(crate::me_agents::saved_error)?;
    c.json(
        StatusCode::OK,
        &api::OAuthRedirect {
            redirect_to: redirect,
        },
    )
}
async fn deny_consent(c: &mut Ctx) -> Result {
    protect_consent(c);
    let owner = crate::me_agents::member(c).await?;
    let id = public_id(c);
    let issuer = issuer(c);
    let redirect = c
        .app()
        .db
        .write(move |tx| {
            let auth = Authorization::pending(tx.conn(), &id, owner.id, tx.now())?;
            tx.conn().execute(
                "UPDATE oauth_authorizations SET status='denied',updated_at=? WHERE id=?",
                params![tx.now(), auth.id],
            )?;
            Ok(oauth::callback(
                &auth.redirect_uri,
                auth.state.as_deref(),
                &issuer,
                "error",
                "access_denied",
            ))
        })
        .await
        .map_err(campfire_runtime::context::db_error)?;
    c.json(
        StatusCode::OK,
        &api::OAuthRedirect {
            redirect_to: redirect,
        },
    )
}
async fn register(c: &mut Ctx) -> Result {
    public(c);
    let ip = c.request.remote_ip()?.to_string();
    throttle_window(c, &format!("register:{ip}"), 10, 3600)?;
    let invalid = "invalid_client_metadata";
    if c.request.content_type().is_none_or(|value| {
        !value
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .eq_ignore_ascii_case("application/json")
    }) {
        return c.json(StatusCode::BAD_REQUEST, &json!({"error": invalid}));
    }
    let bytes = c.read_body(16385).await;
    let registration = if bytes.len() > 16384 {
        Err(invalid)
    } else {
        serde_json::from_slice(&bytes)
            .map_err(|_| invalid)
            .and_then(Registration::parse)
    };
    let mut registration = match registration {
        Ok(registration) => registration,
        Err(error) => return c.json(StatusCode::BAD_REQUEST, &json!({"error": error})),
    };
    let response = c.app().db.write(move |tx| {
        let client_id = oauth::secret();
        registration.metadata["client_id"] = json!(client_id);
        registration.metadata["client_id_issued_at"] = json!(tx.now().as_second());
        tx.conn().execute(
            "INSERT INTO oauth_clients(client_id,registration,client_name,client_uri,redirect_uris,auth_methods,metadata,created_at,updated_at) VALUES(?,'dcr',?,?,?,'[\"none\"]',?,?,?)",
            params![client_id, registration.client_name, registration.client_uri,
                json!(registration.redirect_uris).to_string(), registration.metadata.to_string(),
                tx.now(), tx.now()],
        )?;
        Ok(registration.metadata)
    }).await.map_err(Error::internal)?;
    c.json(StatusCode::CREATED, &response)
}
async fn form(c: &mut Ctx) -> Result<Option<BTreeMap<String, String>>> {
    if c.request.content_type().is_none_or(|value| {
        value.split(';').next().unwrap_or("").trim() != "application/x-www-form-urlencoded"
    }) {
        return Ok(None);
    }
    let bytes = c.read_body(16385).await;
    if bytes.len() > 16384 {
        return Ok(None);
    }
    let mut input = BTreeMap::new();
    for (key, value) in campfire_app::oauth::form_pairs(&bytes) {
        if input.insert(key, value).is_some() {
            return Ok(None);
        }
    }
    Ok(Some(input))
}
fn none_method(c: &Ctx, input: &BTreeMap<String, String>) -> bool {
    c.request.header("authorization").is_none()
        && !["client_secret", "client_assertion", "client_assertion_type"]
            .iter()
            .any(|key| input.contains_key(*key))
        && input
            .get("token_endpoint_auth_method")
            .is_none_or(|method| method == "none")
}
struct ClientAuthentication {
    client_id: String,
    method: Option<client::AuthMethod>,
    verified: Option<assertion::Verified>,
}

async fn client_authentication(
    c: &mut Ctx,
    input: &BTreeMap<String, String>,
) -> Result<ClientAuthentication> {
    let mut client_id = input.get("client_id").cloned().unwrap_or_default();
    let ip = c.request.remote_ip()?.to_string();
    let has_assertion =
        input.contains_key("client_assertion") || input.contains_key("client_assertion_type");
    let has_header = c.request.header("authorization").is_some();
    let has_secret = input.contains_key("client_secret");
    if (has_assertion && (has_header || has_secret)) || (has_header && has_secret) {
        throttle(c, &format!("token:{client_id}:{ip}"), 60)?;
        return campfire_kit::halt(
            c.json(StatusCode::BAD_REQUEST, &json!({"error":"invalid_request"}))?,
        );
    }
    let assertion = if has_assertion {
        if input.get("client_assertion_type").map(String::as_str) != Some(assertion::TYPE)
            || input
                .get("token_endpoint_auth_method")
                .is_some_and(|method| method != "private_key_jwt")
        {
            throttle(c, &format!("token:{client_id}:{ip}"), 60)?;
            return campfire_kit::halt(
                c.json(StatusCode::BAD_REQUEST, &json!({"error":"invalid_client"}))?,
            );
        }
        let parsed = assertion::parse(
            input
                .get("client_assertion")
                .map(String::as_str)
                .unwrap_or_default(),
            input.get("client_id").map(String::as_str),
            &issuer(c),
            Timestamp::from_jiff(c.now()),
        );
        let Ok(parsed) = parsed else {
            throttle(c, &format!("token:{client_id}:{ip}"), 60)?;
            return campfire_kit::halt(
                c.json(StatusCode::BAD_REQUEST, &json!({"error":"invalid_client"}))?,
            );
        };
        client_id = parsed.client_id().to_owned();
        Some(parsed)
    } else {
        None
    };
    throttle(c, &format!("token:{client_id}:{ip}"), 60)?;
    let app = c.app().clone();
    let verified = if let Some(assertion) = assertion {
        match assertion::verify(&app, assertion, || throttle(c, "cimd-global", 30)).await {
            Ok(verified) => Some(verified),
            Err(Error::Halt(response)) => return Err(Error::Halt(response)),
            Err(Error::Internal(error)) => return Err(Error::Internal(error)),
            Err(_) => {
                return campfire_kit::halt(
                    c.json(StatusCode::BAD_REQUEST, &json!({"error":"invalid_client"}))?,
                );
            }
        }
    } else {
        None
    };
    let method = if verified.is_some() {
        Some(client::AuthMethod::PrivateKeyJwt)
    } else if none_method(c, input) {
        Some(client::AuthMethod::None)
    } else {
        None
    };
    Ok(ClientAuthentication {
        client_id,
        method,
        verified,
    })
}

async fn token(c: &mut Ctx) -> Result {
    public(c);
    let Some(input) = form(c).await? else {
        return c.json(StatusCode::BAD_REQUEST, &json!({"error":"invalid_request"}));
    };
    let ClientAuthentication {
        client_id,
        method,
        verified,
    } = client_authentication(c, &input).await?;
    let missing_credentials = c.request.header("authorization").is_none()
        && !input.contains_key("client_secret")
        && !input.contains_key("client_assertion");
    let result = c
        .app()
        .db
        .write(move |tx| {
            if let Some(verified) = verified
                && !verified.consume(tx)?
            {
                return Ok(("invalid_client", None));
            }
            let value = |key: &str| input.get(key).map(String::as_str).unwrap_or_default();
            match value("grant_type") {
                "authorization_code" => {
                    if missing_credentials
                        && OAuthClient::find_by_client_id(tx.conn(), &client_id)?.is_some_and(
                            |client| !client::allows(&client, client::AuthMethod::None),
                        )
                    {
                        return Ok(("invalid_client", None));
                    }
                    oauth::exchange_code(
                        tx,
                        oauth::CodeExchange {
                            code: value("code"),
                            client_id: &client_id,
                            redirect_uri: input.get("redirect_uri").map(String::as_str),
                            verifier: value("code_verifier"),
                            resource: value("resource"),
                            method,
                        },
                    )
                    .map(|pair| ("invalid_grant", pair))
                }
                "refresh_token" => oauth::refresh(
                    tx,
                    value("refresh_token"),
                    &client_id,
                    input.get("resource").map(String::as_str),
                    method,
                )
                .map(|pair| ("invalid_grant", pair)),
                _ => Ok(("unsupported_grant_type", None)),
            }
        })
        .await
        .map_err(Error::internal)?;
    if let Some(pair) = result.1 {
        c.json(StatusCode::OK, &pair)
    } else {
        c.json(StatusCode::BAD_REQUEST, &json!({"error":result.0}))
    }
}

async fn revoke(c: &mut Ctx) -> Result {
    public(c);
    if let Some(input) = form(c).await? {
        let ClientAuthentication {
            client_id,
            method,
            verified,
        } = client_authentication(c, &input).await?;
        let accepted = c
            .app()
            .db
            .write(move |tx| {
                if let Some(verified) = verified
                    && !verified.consume(tx)?
                {
                    return Ok(false);
                }
                oauth::revoke(
                    tx,
                    input.get("token").map(String::as_str).unwrap_or_default(),
                    &client_id,
                    method,
                )?;
                Ok(true)
            })
            .await
            .map_err(Error::internal)?;
        if !accepted {
            return c.json(StatusCode::BAD_REQUEST, &json!({"error":"invalid_client"}));
        }
    }
    Ok(c.head(StatusCode::OK))
}
