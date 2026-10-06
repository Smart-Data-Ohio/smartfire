//! `SudoMode` and the WS14 hand-off for Google re-authentication.
use super::{require_current_user, session_keys};
use campfire_db::AuthAudit;
use campfire_kit::{Ctx, Result, halt};

pub use crate::state::sudo::State;

/// Run at the same point as Rails' controller callback: after authorization and any record or
/// configuration checks, before parsing permitted params, staging uploads or changing rows.
pub fn require_sudo_mode(c: &mut Ctx) -> Result<()> {
    let now = c.now();
    if session_keys::sudo_verified(c.session(), now) {
        return Ok(());
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

pub fn audit(c: &Ctx) -> Result<AuthAudit> {
    Ok(AuthAudit {
        actor: require_current_user(c)?.clone(),
        ip_address: c.request.remote_ip()?.to_string(),
        user_agent: c.request.user_agent().map(str::to_owned),
    })
}
