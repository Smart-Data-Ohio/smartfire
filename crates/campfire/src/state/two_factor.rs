//! WS14's live OAuth adapter plugs into this self-service seam separately from sudo.
use campfire_kit::{Ctx, Response, Result};
use std::sync::{Arc, RwLock};
/// Create a one-use browser-bound state/nonce/PKCE flow with purpose="reauth", this user_id,
/// prompt=login and max_age=0. Verify and consume the flow and ID token before calling
/// `controllers::two_factor::finish_google_reauthentication`; request params are never proof.
pub trait GoogleReauthentication: Send + Sync {
    fn configured(&self) -> bool;
    fn start(&self, c: &mut Ctx, user_id: i64) -> Result<Response>;
}
#[derive(Default)]
pub struct State {
    google: RwLock<Option<Arc<dyn GoogleReauthentication>>>,
}
impl State {
    #[cfg(test)]
    pub(crate) fn fixture_snapshot(&self) -> Self {
        Self {
            google: RwLock::new(self.google.read().unwrap_or_else(|p| p.into_inner()).clone()),
        }
    }

    #[allow(dead_code)] // WS14 installs its adapter at boot.
    pub fn install_google(&self, adapter: Arc<dyn GoogleReauthentication>) {
        *self.google.write().unwrap_or_else(|e| e.into_inner()) = Some(adapter);
    }
    pub fn google(&self) -> Option<Arc<dyn GoogleReauthentication>> {
        self.google
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .filter(|a| a.configured())
    }
}
