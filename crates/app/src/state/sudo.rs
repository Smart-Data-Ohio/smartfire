//! `SudoMode`'s share of the app state: the registered verifiers and the WS14 Google hand-off.
//! The `require_sudo_mode` callback is [`crate::concerns::sudo`].
use campfire_db::{Connection, TwoFactorCredential, Tx};
use campfire_kit::{Ctx, Response, Result};
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
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_snapshot(&self) -> Self {
        Self {
            google: RwLock::new(self.google.read().unwrap_or_else(|p| p.into_inner()).clone()),
            extra_verifiers: RwLock::new(self.extra_verifiers()),
        }
    }

    /// The registered verifier's model API, shared by the prompt and confirmation action.
    pub fn verifier_available(
        &self,
        conn: &Connection,
        name: &str,
        user_id: i64,
    ) -> campfire_db::Result<bool> {
        Ok(name == "totp"
            && self
                .extra_verifiers()
                .iter()
                .any(|registered| registered == name)
            && TwoFactorCredential::for_user(conn, user_id)?
                .is_some_and(|credential| credential.enabled()))
    }

    /// None is Rails' :unsupported; a rejected credential is Some(false).
    pub fn verify_totp(
        &self,
        tx: &mut Tx<'_>,
        user_id: i64,
        secrets: &rails_compat::Secrets,
        code: &str,
    ) -> campfire_db::Result<Option<bool>> {
        let Some(mut credential) = TwoFactorCredential::for_user(tx.conn(), user_id)?
            .filter(|credential| credential.enabled())
        else {
            return Ok(None);
        };
        if credential.locked_out(tx.now()) {
            return Ok(Some(false));
        }
        if credential.verify_code(
            tx,
            &rails_compat::ar_encryption::ArEncryption::new(secrets),
            code,
        )? {
            credential.register_challenge_success(tx)?;
            Ok(Some(true))
        } else {
            credential.register_challenge_failure(tx)?;
            Ok(Some(false))
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
