//! Shared request-specific message payload adapter.
//! WS8b-m owns MessagePayloadHelper; WS11-api calls the stable Presenter method.
#![allow(dead_code)] // WS11-api calls the stable entry point after its HTTP branch merges.
use super::{Presenter, Result};
use campfire_db::{Message, User};
use serde_json::Value;
use std::sync::Arc;

pub trait MessagePayload: Send + Sync {
    fn message(
        &self,
        presenter: &Presenter<'_>,
        message: &Message,
        viewer: &User,
        base_url: &str,
    ) -> Result<Value>;
}
struct SharedPayload;
impl MessagePayload for SharedPayload {
    fn message(
        &self,
        presenter: &Presenter<'_>,
        message: &Message,
        viewer: &User,
        base_url: &str,
    ) -> Result<Value> {
        crate::controllers::presenters::message_payload::message(presenter, message, viewer, base_url)
    }
}
pub use crate::state::agent_payload::State;

/// The payload adapter in the app's [`State`] slot, which holds it type-erased: the adapter
/// takes a [`Presenter`], above the app state.
pub trait StateExt {
    fn live() -> State;
    fn install(&self, adapter: Arc<dyn MessagePayload>);
    fn message(&self, presenter: &Presenter<'_>, message: &Message) -> Result<Value>;
}
impl StateExt for State {
    fn live() -> State {
        let state = State::default();
        state.install(Arc::new(SharedPayload));
        state
    }
    fn install(&self, adapter: Arc<dyn MessagePayload>) {
        self.install_erased(Arc::new(adapter));
    }
    fn message(&self, presenter: &Presenter<'_>, message: &Message) -> Result<Value> {
        let viewer = presenter.current_user_id.ok_or_else(|| {
            campfire_db::Error::Other("message payload requires Current.user".into())
        })?;
        let viewer = presenter.user(viewer)?;
        let base = presenter.cache_base_url.as_deref().ok_or_else(|| {
            campfire_db::Error::Other("message payload requires request base URL".into())
        })?;
        // Only `install` fills the slot, so it always holds an adapter.
        let adapter = self
            .erased()
            .and_then(|adapter| adapter.downcast_ref::<Arc<dyn MessagePayload>>().cloned());
        // Explicitly uninstalled custom states fail instead of returning cached stock JSON.
        adapter
            .ok_or_else(|| {
                campfire_db::Error::Other(
                    "WS8b-m MessagePayloadHelper adapter is not installed".into(),
                )
            })?
            .message(presenter, message, &viewer, base)
    }
}
impl Presenter<'_> {
    pub fn agent_message_payloads(&self, messages: &[Message]) -> Result<Vec<Value>> {
        self.agent_message_payloads_with_users(messages)
            .map(|(values, _)| values)
    }
    pub fn agent_message_payloads_with_users(
        &self,
        messages: &[Message],
    ) -> Result<(Vec<Value>, std::collections::HashMap<i64, User>)> {
        let p = self.preload_payload(messages)?;
        let users = p
            .search_preloads
            .as_ref()
            .expect("payload preloads")
            .users
            .iter()
            .map(|(id, record)| (*id, record.user.clone()))
            .collect();
        let values = messages
            .iter()
            .map(|m| p.agent_message_payload(m))
            .collect::<Result<_>>()?;
        Ok((values, users))
    }
    /// Matches WS11-api's seam. Its Current.user is the authenticated bot.
    pub fn agent_message_payload(&self, message: &Message) -> Result<Value> {
        self.agent_payload.message(self, message)
    }
}
