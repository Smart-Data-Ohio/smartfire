//! Transactional callbacks supplied by peer domains after their branches merge.
//! No adapter may perform network I/O while the SQLite writer is held.
use crate::{Result, Tx};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    MessageActivity,
    MessageGithubReferences,
    MessageFizzyReferences,
    MessageTwitterReferences,
    MessageEventReferences,
    MessageLinkReferences,
    UserHuddles,
    UserGoogleAccount,
    UserCalendarMeetingCache,
    UserGoogleIdentity,
    UserGithubAccount,
    UserFizzyAccount,
    UserSlackConnection,
    UserSlackImports,
    UserEventCalendarEntries,
    SessionHuddles,
}
#[derive(Clone, Copy, Debug)]
pub struct Callback {
    pub phase: Phase,
    /// Message, User or Session id, according to the phase.
    pub record_id: i64,
}
type Handler = dyn Fn(&mut Tx<'_>, Callback) -> Result<()> + Send + Sync;
#[derive(Default)]
pub struct Registry {
    handlers: RwLock<HashMap<Phase, Arc<Handler>>>,
}
impl Registry {
    pub fn install(
        &self,
        phase: Phase,
        handler: impl Fn(&mut Tx<'_>, Callback) -> Result<()> + Send + Sync + 'static,
    ) {
        self.handlers
            .write()
            .unwrap_or_else(|p| p.into_inner())
            .insert(phase, Arc::new(handler));
    }
    pub fn call(&self, tx: &mut Tx<'_>, callback: Callback) -> Result<()> {
        let handler = self
            .handlers
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .get(&callback.phase)
            .cloned();
        // FLAGGED STUB: WS12 activity, WS13 huddles, WS14 Google/events,
        // WS15 GitHub/Fizzy/embeds and WS16 Slack install their model adapters.
        // Leave their rows intact until installed; deletion FK failures roll back.
        match handler {
            Some(handler) => handler(tx, callback),
            None => Ok(()),
        }
    }
}
