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
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::database::Tx;
use crate::error::Result;

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

    /// A broadcast a model makes from a callback (`broadcast_*_to`, `Turbo::StreamsChannel.
    /// broadcast_*`, `ActionCable.server.broadcast`): the model describes it as a [`Broadcast`] in
    /// its own module, and the app's cable sink finds the handler by [`Broadcast::KIND`] and
    /// sends it, in emit order, from the thread that committed (as Rails renders and publishes
    /// in the committing thread). Emit it with `tx.emit_after_commit` for `after_*_commit`
    /// callbacks. Open like [`Event::Job`], so domains don't edit a central list.
    Broadcast(BroadcastRequest),
}

/// A job's arguments, serializable like Active Job's. Implemented by each domain's job type.
pub trait Job: Serialize + DeserializeOwned {
    /// The Rails job class (`"Calendar::SyncEntryJob"`): the name the runner dispatches on, and
    /// what its logs say.
    const CLASS: &'static str;
}

/// A [`Job`] ready to enqueue: its class, its serialized arguments, and how long to wait before
/// performing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRequest {
    pub class: &'static str,
    pub arguments: serde_json::Value,
    /// `set(wait:)`: performed no sooner than this long after the enqueueing write.
    pub wait: Option<Duration>,
}

impl JobRequest {
    /// `J.perform_later(arguments)`
    pub fn new<J: Job>(arguments: &J) -> Self {
        Self {
            class: J::CLASS,
            arguments: serde_json::to_value(arguments).expect("job arguments serialize to JSON"),
            wait: None,
        }
    }

    /// `J.set(wait:)`
    pub fn wait(mut self, wait: Duration) -> Self {
        self.wait = Some(wait);
        self
    }

    /// The arguments as `J`, or `None` if this is another class's job.
    pub fn decode<J: Job>(&self) -> Option<serde_json::Result<J>> {
        (self.class == J::CLASS).then(|| serde_json::from_value(self.arguments.clone()))
    }
}

/// What a model broadcasts: the arguments the app's handler for [`Broadcast::KIND`] needs to
/// name the stream and render the content. Implemented by each domain's broadcast type.
pub trait Broadcast: Serialize + DeserializeOwned {
    /// The Rails method that broadcasts (`"Membership#broadcast_room_removal_to_user"`): what the
    /// handler is registered under, and what the logs say.
    const KIND: &'static str;
}

/// A [`Broadcast`] ready to send: its kind and its serialized arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BroadcastRequest {
    pub kind: &'static str,
    pub arguments: serde_json::Value,
}

impl BroadcastRequest {
    /// The arguments as `B`, or `None` if this is another kind's broadcast.
    pub fn decode<B: Broadcast>(&self) -> Option<serde_json::Result<B>> {
        (self.kind == B::KIND).then(|| serde_json::from_value(self.arguments.clone()))
    }
}

impl Event {
    /// A model's broadcast (see [`Event::Broadcast`]).
    pub fn broadcast<B: Broadcast>(arguments: &B) -> Self {
        Event::Broadcast(BroadcastRequest {
            kind: B::KIND,
            arguments: serde_json::to_value(arguments).expect("broadcast arguments serialize to JSON"),
        })
    }

    /// `J.perform_later(arguments)`
    pub fn job<J: Job>(arguments: &J) -> Self {
        Event::Job(JobRequest::new(arguments))
    }

    /// `J.set(wait:).perform_later(arguments)`
    pub fn job_in<J: Job>(wait: Duration, arguments: &J) -> Self {
        Event::Job(JobRequest::new(arguments).wait(wait))
    }

    /// The job request, if this is a domain job of class `J`.
    pub fn as_job<J: Job>(&self) -> Option<J> {
        match self {
            Event::Job(request) => request.decode::<J>().and_then(|decoded| decoded.ok()),
            _ => None,
        }
    }

    /// The WS8 messaging description, if this is its broadcast kind.
    pub fn as_broadcast(&self) -> Option<crate::broadcasts::Broadcast> {
        match self {
            Event::Broadcast(request) => request.decode::<crate::broadcasts::Broadcast>().and_then(|value| value.ok()),
            _ => None,
        }
    }
}

pub trait EventSink: Send + Sync {
    /// Rails attaches a daily digest claim only after its explicit broadcast succeeds.
    /// The app returns the successfully published note IDs; recorder-only sinks retain
    /// their ordinary event behavior. Run outside the writer so rendering can read.
    fn broadcast_digest_notes(&self, notes: &crate::models::board_automations::DigestNotes) -> Result<Vec<i64>> {
        self.emit(Event::broadcast(notes));
        Ok(notes.message_ids.clone())
    }
    /// Peer model adapter. The default is a flagged uninstalled stub, as with
    /// `callbacks::Registry`; production Jobs dispatches its installed handlers.
    fn model_callback(&self, _tx: &mut Tx<'_>, _callback: crate::callbacks::Callback) -> Result<()> { Ok(()) }
    /// Domain connected-account callbacks in User#deactivate. SQL only, on the triggering write.
    fn disconnect_user_accounts(&self, _tx: &mut Tx<'_>, _user_id: i64) -> Result<()> {
        Ok(())
    }

    /// Domain reference callbacks run on the message writer's transaction. No network or
    /// HTML belongs here; durable fetch requests must commit with their triggering message.
    fn sync_message_references(&self, _tx: &mut Tx<'_>, _message: &crate::Message, _enqueue: bool) -> Result<()> {
        Ok(())
    }

    /// A real reference adapter at its position in Message#sync_all_references.
    /// Existing custom sinks retain their combined hook at the link phase.
    fn sync_message_reference_phase(&self, tx: &mut Tx<'_>, message: &crate::Message, phase: crate::callbacks::Phase, enqueue: bool) -> Result<()> {
        if phase == crate::callbacks::Phase::MessageLinkReferences {
            self.sync_message_references(tx, message, enqueue)
        } else { Ok(()) }
    }

    /// Hands the event off, once its write has committed (or right away, for [`Tx::emit_now`]).
    fn emit(&self, event: Event);

    /// Writes what the event needs in the database, on the emitting write's connection, when it's
    /// emitted: a sink backed by the durable job queue inserts a job's row here, so the job
    /// commits or rolls back with the write that enqueued it (and its runner, woken by
    /// [`EventSink::emit`] after the commit, can't see it before). An error fails that write. The
    /// default writes nothing.
    fn persist(&self, _tx: &Tx<'_>, _event: &Event) -> Result<()> {
        Ok(())
    }
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
    fn model_callback(&self, tx: &mut Tx<'_>, callback: crate::callbacks::Callback) -> Result<()> {
        (**self).model_callback(tx, callback)
    }

    fn disconnect_user_accounts(&self, tx: &mut Tx<'_>, user_id: i64) -> Result<()> {
        (**self).disconnect_user_accounts(tx, user_id)
    }

    fn sync_message_references(&self, tx: &mut Tx<'_>, message: &crate::Message, enqueue: bool) -> Result<()> {
        (**self).sync_message_references(tx, message, enqueue)
    }

    fn sync_message_reference_phase(&self, tx: &mut Tx<'_>, message: &crate::Message, phase: crate::callbacks::Phase, enqueue: bool) -> Result<()> {
        (**self).sync_message_reference_phase(tx, message, phase, enqueue)
    }

    fn emit(&self, event: Event) {
        (**self).emit(event)
    }

    fn persist(&self, tx: &Tx<'_>, event: &Event) -> Result<()> {
        (**self).persist(tx, event)
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
