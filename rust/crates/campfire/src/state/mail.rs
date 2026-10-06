//! Mail's share of the app state: its configuration, the inbound throttle, and the renderer and
//! webhook fanout installed at boot. The jobs and relay that use it are [`crate::mail`].
use campfire_db::Message;
use campfire_mail::config::Config;
use campfire_mail::inbound::{Renderer, Throttle};
use std::sync::{Arc, RwLock};

pub(crate) type Fanout =
    Arc<dyn Fn(&mut campfire_db::Tx<'_>, &Message) -> campfire_db::Result<()> + Send + Sync>;
pub struct State {
    pub config: Config,
    pub(crate) throttle: Throttle,
    pub(crate) renderer: RwLock<Option<Arc<dyn Renderer>>>,
    pub(crate) fanout: RwLock<Option<Fanout>>,
}
impl State {
    #[cfg(test)]
    pub(crate) fn fixture_snapshot(&self) -> Self {
        Self {
            config: self.config.clone(),
            throttle: self.throttle.clone(),
            renderer: RwLock::new(self.renderer.read().unwrap_or_else(|p| p.into_inner()).clone()),
            fanout: RwLock::new(self.fanout.read().unwrap_or_else(|p| p.into_inner()).clone()),
        }
    }

    pub fn new(config: Config) -> Self {
        Self {
            config,
            throttle: Throttle::default(),
            renderer: RwLock::new(None),
            fanout: RwLock::new(None),
        }
    }
    /// Called at boot by WS5/WS8 after the shared Markdown/mention renderer has landed.
    pub fn install_renderer(&self, renderer: Arc<dyn Renderer>) {
        *self.renderer.write().unwrap_or_else(|e| e.into_inner()) = Some(renderer);
    }
    /// WS11's Message::BotWebhookFanout hook, called inside a write after the broadcast.
    #[allow(dead_code)]
    pub fn install_fanout(&self, fanout: Fanout) {
        *self.fanout.write().unwrap_or_else(|e| e.into_inner()) = Some(fanout);
    }
}
