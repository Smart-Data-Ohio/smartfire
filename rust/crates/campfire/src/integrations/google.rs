//! Google identity and opt-in Calendar/Drive integrations.
pub mod api;
pub mod calendar;
pub mod calendar_sync;
pub mod client;
pub mod drive;
pub mod entry_sync;
pub mod meeting_refresh;
pub mod sign_in;
use crate::app::AppCtx;
use campfire_kit::{Ctx, Response, Result};
use std::sync::{Arc, RwLock};
#[derive(Clone)]
pub struct State(
    Arc<RwLock<Arc<sign_in::SignIn>>>,
    Arc<RwLock<Arc<api::Api>>>,
    Arc<drive::State>,
);
impl Default for State {
    fn default() -> Self {
        Self(
            Arc::new(RwLock::new(Arc::new(sign_in::SignIn::new(
                sign_in::Config::from_env(),
                crate::integrations::net::Network::system(),
            )))),
            Arc::new(RwLock::new(Arc::new(api::Api::default()))),
            Arc::new(drive::State::default()),
        )
    }
}
impl State {
    pub fn drive(&self) -> &drive::State {
        &self.2
    }
    pub fn api(&self) -> Arc<api::Api> {
        self.1.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
    #[cfg(test)]
    pub fn install_api(&self, service: api::Api) {
        *self.1.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(service);
    }
    pub fn sign_in(&self) -> Arc<sign_in::SignIn> {
        self.0.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
    #[cfg(test)]
    pub fn install(&self, service: sign_in::SignIn) {
        *self.0.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(service);
    }
    pub fn start(&self, c: &mut Ctx, purpose: &str, user_id: Option<i64>) -> Result<Response> {
        let service = self.sign_in();
        let (flow, location) = service.start(
            &c.app().secrets,
            &c.url_for("/session/google/callback"),
            purpose,
            user_id,
            c.now(),
        );
        c.session().insert(sign_in::FLOW_SESSION_KEY, flow);
        c.redirect_to_with(
            &location,
            campfire_kit::Redirect {
                allow_other_host: true,
                ..Default::default()
            },
        )
    }
}
impl crate::concerns::sudo::GoogleSudo for State {
    fn configured(&self) -> bool {
        self.sign_in().config.configured()
    }
    fn start(&self, c: &mut Ctx, user_id: i64) -> Result<Response> {
        self.start(c, "sudo", Some(user_id))
    }
}
impl crate::concerns::two_factor::GoogleReauthentication for State {
    fn configured(&self) -> bool {
        self.sign_in().config.configured()
    }
    fn start(&self, c: &mut Ctx, user_id: i64) -> Result<Response> {
        self.start(c, "reauth", Some(user_id))
    }
}
