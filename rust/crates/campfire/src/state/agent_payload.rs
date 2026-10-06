//! The slot for the agent message payload adapter in the app state. The adapter, and the
//! payload, are the presenters' ([`crate::controllers::presenters::agent_payload`]); it takes a
//! `Presenter`, which sits above the app state, so the slot holds it type-erased.
use std::any::Any;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct State {
    adapter: RwLock<Option<Arc<dyn Any + Send + Sync>>>,
}
impl State {
    #[cfg(test)]
    pub(crate) fn fixture_snapshot(&self) -> Self {
        Self {
            adapter: RwLock::new(self.adapter.read().unwrap_or_else(|p| p.into_inner()).clone()),
        }
    }

    pub fn install_erased(&self, adapter: Arc<dyn Any + Send + Sync>) {
        *self.adapter.write().unwrap_or_else(|p| p.into_inner()) = Some(adapter);
    }
    pub fn erased(&self) -> Option<Arc<dyn Any + Send + Sync>> {
        self.adapter.read().unwrap_or_else(|p| p.into_inner()).clone()
    }
}
