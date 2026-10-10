//! `SudoMode` and the WS14 hand-off for Google re-authentication.
use super::{require_current_user, session_keys};
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
        let pending = serde_json::json!({
            "method": c.request.method.as_str(), "path": c.request.fullpath(),
            "params": null, "origin": origin
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
    // Approved divergence: don't reproduce Rails' oversized-cookie crash.
    // The storable-params check still counts Ruby Integer digits without quotes.
    if !c.session_cookie_fits()
        && let Some(mut request) = c.session().get(session_keys::SUDO_PENDING_KEY).cloned()
    {
        request["params"] = serde_json::Value::Null;
        c.session().insert(session_keys::SUDO_PENDING_KEY, request);
    }
    let location = c.url_for(&campfire_routes::new_sudo());
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

pub fn retry(c: &mut Ctx) -> Option<SudoRetry> {
    let pending = c.session().get(session_keys::SUDO_PENDING_KEY)?;
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
