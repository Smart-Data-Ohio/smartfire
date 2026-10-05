//! `SudoMode` and the WS14 hand-off for Google re-authentication.
use super::{require_current_user, session_keys};
use campfire_db::AuthAudit;
use campfire_kit::{Ctx, Response, Result, halt};
use std::sync::{Arc, RwLock};

/// WS14 installs its browser-bound GoogleSignInFlow adapter. `start` must stash a one-use
/// state/nonce/PKCE flow with purpose="sudo" and this user_id, and request prompt=login,
/// max_age=0. The callback must verify/consume that flow and verify the ID token before calling
/// `sudos::finish_google` with its verified subject and auth_time. No token is accepted here.
pub trait GoogleSudo: Send + Sync {
    fn configured(&self) -> bool;
    fn start(&self, c: &mut Ctx, user_id: i64) -> Result<Response>;
}

pub struct State {
    google: RwLock<Option<Arc<dyn GoogleSudo>>>,
    extra_verifiers: RwLock<Vec<String>>,
}
impl Default for State {
    fn default() -> Self {
        let state = Self {
            google: RwLock::new(None),
            extra_verifiers: RwLock::new(Vec::new()),
        };
        // config/initializers/sudo_mode_totp.rb registers TOTP at boot.
        state.register_verifier("totp");
        state
    }
}

impl State {
    #[cfg(test)]
    pub(crate) fn fixture_snapshot(&self) -> Self {
        Self {
            google: RwLock::new(self.google.read().unwrap_or_else(|p| p.into_inner()).clone()),
            extra_verifiers: RwLock::new(self.extra_verifiers()),
        }
    }

    /// SudoMode.register_verifier preserves order and ignores duplicate names.
    /// Registration alone does not supply a verifier implementation.
    pub fn register_verifier(&self, name: &str) {
        let mut names = self
            .extra_verifiers
            .write()
            .unwrap_or_else(|p| p.into_inner());
        if !names.iter().any(|n| n == name) {
            names.push(name.into());
        }
    }
    pub fn extra_verifiers(&self) -> Vec<String> {
        self.extra_verifiers
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    #[allow(dead_code)] // WS14's boot adapter installs this when its verifier lands.
    pub fn install_google(&self, google: Arc<dyn GoogleSudo>) {
        *self
            .google
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(google);
    }

    pub fn google(&self) -> Option<Arc<dyn GoogleSudo>> {
        self.google
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .filter(|google| google.configured())
    }
}

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
