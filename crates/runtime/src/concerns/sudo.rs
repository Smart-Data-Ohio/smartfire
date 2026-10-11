//! `SudoMode` and the WS14 hand-off for Google re-authentication.
use super::{current_session, require_current_user, session_keys};
use crate::app::AppCtx;
use campfire_api_types::{ApiError, ApiErrorResponse, SudoMethod, SudoRetry, SudoState};
use campfire_db::AuthAudit;
use campfire_kit::{Ctx, Error, Result, StatusCode, format, halt};
use rusqlite::OptionalExtension;

pub use crate::state::sudo::State;

/// Run at the same point as Rails' controller callback: after authorization and any record or
/// configuration checks, before parsing permitted params, staging uploads or changing rows.
pub async fn require_sudo_mode(c: &mut Ctx) -> Result<()> {
    let now = c.now();
    if session_keys::sudo_verified(c.session(), now) {
        return Ok(());
    }
    if json_request(c)? {
        let origin =
            session_keys::sudo_origin_path(c.request.referer(), &c.request.host(), "/app/");
        let origin = spa_origin(&origin).to_owned();
        // Bound to the signed-in session: no other session's answer names it.
        let pending = serde_json::json!({
            "method": c.request.method.as_str(), "path": c.request.fullpath(),
            "params": null, "origin": origin, "session_id": session_id(c)
        });
        c.session().insert(session_keys::SUDO_PENDING_KEY, pending);
        let reauthentication = state(c).await?;
        c.no_store();
        return halt(c.json(
            StatusCode::FORBIDDEN,
            &ApiErrorResponse {
                error: ApiError::SudoRequired {
                    message: "Confirm your password to continue".into(),
                    reauthentication,
                },
            },
        )?);
    }
    let pending = session_keys::SudoPendingRequest {
        method: c.request.method.as_str().into(),
        path: c.request.fullpath(),
        params: session_keys::sudo_storable_params(c.request.method.as_str(), &c.request_params),
        origin: session_keys::sudo_origin_path(
            c.request.referer(),
            &c.request.host(),
            &campfire_routes::root(),
        ),
    };
    session_keys::store_sudo_pending_request(c.session(), pending);
    // Bound to the signed-in session, as the JSON gate's: no other session's answer names it.
    let session = session_id(c);
    if let Some(mut request) = c.session().get(session_keys::SUDO_PENDING_KEY).cloned() {
        request["session_id"] = serde_json::json!(session);
        c.session().insert(session_keys::SUDO_PENDING_KEY, request);
    }
    // Approved divergence: don't reproduce Rails' oversized-cookie crash.
    // The storable-params check still counts Ruby Integer digits without quotes.
    if !c.session_cookie_fits()
        && let Some(mut request) = c.session().get(session_keys::SUDO_PENDING_KEY).cloned()
    {
        request["params"] = serde_json::Value::Null;
        c.session().insert(session_keys::SUDO_PENDING_KEY, request);
    }
    let location = c.url_for("/app/sudo/new");
    halt(c.redirect_to(&location)?)
}

pub fn json_request(c: &mut Ctx) -> Result<bool> {
    Ok(c.request.path().starts_with("/api/v1/")
        || c.format()? == Some(&format::JSON)
        || c.request
            .header("accept")
            .and_then(|value| format::parse_accept(value).ok())
            .is_some_and(|formats| formats.first() == Some(&&format::JSON)))
}

fn spa_origin(path: &str) -> &str {
    if path.starts_with("/app/") {
        path
    } else {
        "/app/"
    }
}

/// The request the gate holds for the signed-in session, if any. One held for another session
/// (it names a different one) is never named.
pub fn retry(c: &mut Ctx) -> Option<SudoRetry> {
    let current = session_id(c);
    let pending = c.session().get(session_keys::SUDO_PENDING_KEY)?;
    if pending
        .get("session_id")
        .is_some_and(|held| held.as_i64() != current)
    {
        return None;
    }
    let path = pending.get("path")?.as_str()?;
    if !path.starts_with('/') || path.starts_with("//") {
        return None;
    }
    Some(SudoRetry {
        method: pending.get("method")?.as_str()?.to_uppercase(),
        path: path.into(),
        return_to: spa_origin(
            pending
                .get("origin")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("/app/"),
        )
        .into(),
    })
}

/// Whether `attempt` can name an SPA Google confirmation: 16 to 64 letters, digits, `-` or `_`.
pub fn valid_attempt(attempt: &str) -> bool {
    (16..=64).contains(&attempt.len())
        && attempt
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// The signed-in session's id: SPA Google confirmations are bound to it.
pub fn session_id(c: &Ctx) -> Option<i64> {
    current_session(c).map(|session| session.id)
}

/// The SPA's Google confirmation for `attempt` finished in this session: keeps who confirmed, in
/// which session, when, and the pending request it answers (consuming it), for the continuation
/// to name.
pub fn record_attempt(c: &mut Ctx, attempt: &str) -> Result<()> {
    let user_id = require_current_user(c)?.id;
    let session_id = session_id(c);
    let retry = serde_json::to_value(retry(c)).map_err(Error::internal)?;
    let at = c.now().as_second();
    c.session().remove(session_keys::SUDO_PENDING_KEY);
    c.session().insert(
        session_keys::SUDO_ATTEMPT_KEY,
        serde_json::json!({
            "id": attempt, "user_id": user_id, "session_id": session_id, "at": at, "retry": retry
        }),
    );
    Ok(())
}

/// The pending request `attempt` answered, while this session's record of it belongs to the
/// signed-in member and session and is under `SUDO_ATTEMPT_TTL` old; `None` otherwise. Reading it again gives
/// the same answer until the SPA acknowledges it or it expires.
pub fn attempt_retry(c: &mut Ctx, attempt: &str) -> Result<Option<Option<SudoRetry>>> {
    let user_id = require_current_user(c)?.id;
    let now = c.now();
    let Some(record) = c.session().get(session_keys::SUDO_ATTEMPT_KEY).cloned() else {
        return Ok(None);
    };
    let fresh = record["at"]
        .as_i64()
        .and_then(|at| jiff::Timestamp::from_second(at).ok())
        .is_some_and(|at| at > now - session_keys::SUDO_ATTEMPT_TTL);
    if !fresh {
        c.session().remove(session_keys::SUDO_ATTEMPT_KEY);
        return Ok(None);
    }
    if !names(c, &record, attempt) || record["user_id"].as_i64() != Some(user_id) {
        return Ok(None);
    }
    Ok(Some(serde_json::from_value(record["retry"].clone()).unwrap_or(None)))
}

/// Whether `record` is `attempt`'s, finished in the signed-in session.
fn names(c: &Ctx, record: &serde_json::Value, attempt: &str) -> bool {
    let session_id = session_id(c);
    record["id"].as_str() == Some(attempt)
        && session_id.is_some()
        && record["session_id"].as_i64() == session_id
}

/// The SPA has what `attempt` confirmed: its record goes (only from the session that finished it).
pub fn acknowledge_attempt(c: &mut Ctx, attempt: &str) {
    let Some(record) = c.session().get(session_keys::SUDO_ATTEMPT_KEY).cloned() else {
        return;
    };
    if names(c, &record, attempt) {
        c.session().remove(session_keys::SUDO_ATTEMPT_KEY);
    }
}

pub async fn methods(c: &Ctx) -> Result<Vec<SudoMethod>> {
    let user = require_current_user(c)?;
    let mut methods = Vec::new();
    if user
        .password_digest
        .as_ref()
        .is_some_and(|digest| !digest.trim().is_empty())
    {
        methods.push(SudoMethod::Password);
    }
    let app = c.app().clone();
    let user_id = user.id;
    let (totp, google) = c
        .app()
        .db
        .read(move |conn| {
            let totp = app.sudo.verifier_available(conn, "totp", user_id)?;
            let google = app.sudo.google().is_some()
                && conn
                    .query_row(
                        "SELECT subject FROM google_identities WHERE user_id = ? LIMIT 1",
                        [user_id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()?
                    .is_some();
            Ok((totp, google))
        })
        .await
        .map_err(Error::internal)?;
    if totp {
        methods.push(SudoMethod::Totp);
    }
    if google {
        methods.push(SudoMethod::Google);
    }
    Ok(methods)
}

pub async fn state(c: &mut Ctx) -> Result<SudoState> {
    Ok(SudoState {
        methods: methods(c).await?,
        retry: retry(c),
    })
}

pub fn audit(c: &Ctx) -> Result<AuthAudit> {
    Ok(AuthAudit {
        actor: require_current_user(c)?.clone(),
        ip_address: c.request.remote_ip()?.to_string(),
        user_agent: c.request.user_agent().map(str::to_owned),
    })
}
