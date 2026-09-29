//! Side effects that leave the database. Models emit these at the point Rails performs
//! them; the caller supplies the [`EventSink`] that turns them into jobs, cable
//! disconnects and so on. The sink runs on the writer thread, so it must hand work off
//! (push onto a queue) rather than do it inline.

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// `Room::PushMessageJob.perform_later(room, message)`, from `Room#receive` after a
    /// message's create commits (`reference/app/models/room.rb`).
    PushMessage { room_id: i64, message_id: i64 },

    /// `ActionCable.server.remote_connections.where(current_user: user).disconnect(reconnect:)`
    /// (`reference/app/models/user.rb`). `reconnect: true` comes from
    /// `reset_remote_connections` (membership destroyed, sign out); `false` from
    /// deactivate and ban, which emit it inside their transaction, before anything is deleted.
    DisconnectUser { user_id: i64, reconnect: bool },

    /// `RemoveBannedContentJob.perform_later(user)` (`User::Bannable#apply_ban`).
    RemoveBannedContent { user_id: i64 },

    /// `Bot::WebhookJob.perform_later(bot, message)` (`User::Bot#deliver_webhook_later`).
    DeliverWebhook { bot_id: i64, message_id: i64 },

    /// `ActiveStorage::Blob#purge_later`: the blob lost its attachment when its record was
    /// destroyed (`has_one_attached` defaults to `dependent: :purge_later`).
    PurgeBlob { blob_id: i64 },
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: Event);
}

/// Drops every event.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _event: Event) {}
}

/// Records events, for tests (`assert_enqueued_jobs` and friends).
#[derive(Debug, Default, Clone)]
pub struct RecordingSink {
    events: Arc<Mutex<Vec<Event>>>,
}

impl RecordingSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    pub fn take(&self) -> Vec<Event> {
        std::mem::take(&mut *self.events.lock().unwrap())
    }
}

impl EventSink for RecordingSink {
    fn emit(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }
}

impl<T: EventSink + ?Sized> EventSink for Arc<T> {
    fn emit(&self, event: Event) {
        (**self).emit(event)
    }
}
