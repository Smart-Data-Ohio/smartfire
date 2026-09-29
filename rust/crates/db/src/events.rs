//! Side effects that leave the database. Models emit these at the point Rails performs
//! them; the caller supplies the [`EventSink`] that turns them into jobs, cable
//! disconnects and so on. The sink runs on the writer thread, so it must hand work off
//! (push onto a queue) rather than do it inline.
//!
//! The variants below are upstream's core. Everything a later domain adds (our app has 34 job
//! classes) goes through [`Event::Job`] instead of a new variant: the domain defines its job's
//! arguments as a [`Job`] in its own module and emits `Event::job(&args)`, and the runner finds
//! the handler by [`Job::CLASS`], as Active Job finds a class by name. So no central list is
//! edited per domain, and parallel workstreams don't collide here.

use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde::de::DeserializeOwned;

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

    /// `SomeJob.perform_later(*arguments)` for a job a domain module defines (see [`Job`]).
    Job(JobRequest),
}

/// A job's arguments, serializable like Active Job's. Implemented by each domain's job type.
pub trait Job: Serialize + DeserializeOwned {
    /// The Rails job class (`"Calendar::SyncEntryJob"`): the name the runner dispatches on, and
    /// what its logs say.
    const CLASS: &'static str;
}

/// A [`Job`] ready to enqueue: its class and its serialized arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRequest {
    pub class: &'static str,
    pub arguments: serde_json::Value,
}

impl JobRequest {
    /// The arguments as `J`, or `None` if this is another class's job.
    pub fn decode<J: Job>(&self) -> Option<serde_json::Result<J>> {
        (self.class == J::CLASS).then(|| serde_json::from_value(self.arguments.clone()))
    }
}

impl Event {
    /// `J.perform_later(arguments)`
    pub fn job<J: Job>(arguments: &J) -> Self {
        Event::Job(JobRequest {
            class: J::CLASS,
            arguments: serde_json::to_value(arguments).expect("job arguments serialize to JSON"),
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct SyncEntry {
        event_id: i64,
        user_id: i64,
    }

    impl Job for SyncEntry {
        const CLASS: &'static str = "Calendar::SyncEntryJob";
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct Other {}

    impl Job for Other {
        const CLASS: &'static str = "OtherJob";
    }

    /// A domain's job goes through the open variant and back without a central list.
    #[test]
    fn domain_jobs_round_trip_through_the_open_variant() {
        let args = SyncEntry { event_id: 7, user_id: 9 };
        let Event::Job(request) = Event::job(&args) else { panic!("not a job") };
        assert_eq!(request.class, "Calendar::SyncEntryJob");
        assert_eq!(request.decode::<SyncEntry>().unwrap().unwrap(), args);
        assert!(request.decode::<Other>().is_none(), "another class's job");
    }
}
