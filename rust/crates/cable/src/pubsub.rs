//! In-process replacement for the Redis subscription adapter: `tokio::sync::broadcast` channels
//! per broadcasting, created on first subscribe and dropped with their last subscriber.
//!
//! Payloads are already-encoded JSON, as they are on the Redis wire. Subscribers that would wrap
//! a payload identically (the same channel identifier) share one channel, and each broadcast
//! builds their `{"identifier":…,"message":…}` frame once for all of them. Each channel's ring
//! buffer is bounded; a subscriber that falls behind gets `Lagged` and its connection is closed
//! with `reconnect: true` rather than silently skipping messages.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures_util::stream::{Abortable, BoxStream, StreamExt};
use tokio::sync::broadcast;

use crate::protocol;

/// A frame ready for the socket, shared by every subscriber that receives it.
pub type Frame = crate::socket::Frame;

/// One subscription's stream of frames, as a connection reads it: it ends when the stream is
/// stopped.
pub(crate) type Deliveries = Abortable<BoxStream<'static, Result<Delivery, RecvError>>>;

/// Publication order is assigned under the hub lock, shared across broadcastings (including
/// the internal disconnect channel). It stays off the wire.
#[derive(Clone)]
pub(crate) struct Delivery {
    pub sequence: u64,
    pub frame: Frame,
}

pub struct Hub {
    capacity: usize,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    streams: HashMap<String, Vec<Group>>,
    sequence: u64,
}

/// The subscribers of one broadcasting that receive identical frames: the payload wrapped for
/// the same encoded channel identifier, or the raw payload when `identifier` is `None`.
struct Group {
    identifier: Option<Arc<str>>,
    sender: broadcast::Sender<Delivery>,
}

impl Group {
    fn frame(&self, payload: &str) -> Frame {
        match &self.identifier {
            Some(identifier) => protocol::message(identifier, payload).into(),
            None => payload.into(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RecvError {
    /// The subscriber missed messages; its connection must be closed.
    Lagged,
    Closed,
}

impl Hub {
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self { capacity, state: Mutex::new(State::default()) })
    }

    /// Publishes to every current subscriber of `broadcasting`. Returns how many received it.
    pub fn broadcast(&self, broadcasting: &str, payload: &str) -> usize {
        let mut state = self.state.lock().unwrap();
        state.sequence += 1;
        let sequence = state.sequence;
        let Some(groups) = state.streams.get_mut(broadcasting) else {
            return 0;
        };
        let mut receivers = 0;
        groups.retain(|group| match group.sender.send(Delivery { sequence, frame: group.frame(payload) }) {
            Ok(count) => {
                receivers += count;
                true
            }
            Err(_) => false,
        });
        if groups.is_empty() {
            state.streams.remove(broadcasting);
        }
        receivers
    }

    /// Subscribes to `broadcasting`, receiving each payload wrapped as a message frame for the
    /// encoded channel `identifier`, or raw when it's `None`.
    pub fn subscribe(self: &Arc<Self>, broadcasting: &str, identifier: Option<Arc<str>>) -> Subscriber {
        let mut state = self.state.lock().unwrap();
        let groups = state.streams.entry(broadcasting.to_string()).or_default();
        let receiver = match groups.iter().find(|group| group.identifier == identifier) {
            Some(group) => group.sender.subscribe(),
            None => {
                let (sender, receiver) = broadcast::channel(self.capacity);
                groups.push(Group { identifier: identifier.clone(), sender });
                receiver
            }
        };
        Subscriber { hub: self.clone(), broadcasting: broadcasting.to_string(), identifier, receiver: Some(receiver) }
    }

    /// Number of broadcastings with at least one live subscriber channel.
    pub fn stream_count(&self) -> usize {
        self.state.lock().unwrap().streams.len()
    }

    fn release(&self, broadcasting: &str, identifier: &Option<Arc<str>>) {
        let mut state = self.state.lock().unwrap();
        let Some(groups) = state.streams.get_mut(broadcasting) else {
            return;
        };
        groups.retain(|group| group.identifier != *identifier || group.sender.receiver_count() > 0);
        if groups.is_empty() {
            state.streams.remove(broadcasting);
        }
    }
}

pub struct Subscriber {
    hub: Arc<Hub>,
    broadcasting: String,
    identifier: Option<Arc<str>>,
    receiver: Option<broadcast::Receiver<Delivery>>,
}

impl Subscriber {
    pub fn broadcasting(&self) -> &str {
        &self.broadcasting
    }

    pub async fn recv(&mut self) -> Result<Frame, RecvError> {
        self.recv_delivery().await.map(|delivery| delivery.frame)
    }

    async fn recv_delivery(&mut self) -> Result<Delivery, RecvError> {
        match self.receiver.as_mut().expect("receiver is present until drop").recv().await {
            Ok(frame) => Ok(frame),
            Err(broadcast::error::RecvError::Lagged(_)) => Err(RecvError::Lagged),
            Err(broadcast::error::RecvError::Closed) => Err(RecvError::Closed),
        }
    }

    /// The frames as a stream, which ends if the hub goes away. It holds the subscription until
    /// it's dropped.
    pub fn deliveries(self) -> BoxStream<'static, Result<Frame, RecvError>> {
        self.ordered_deliveries().map(|result| result.map(|delivery| delivery.frame)).boxed()
    }

    pub(crate) fn ordered_deliveries(self) -> BoxStream<'static, Result<Delivery, RecvError>> {
        futures_util::stream::unfold(self, |mut subscriber| async move {
            match subscriber.recv_delivery().await {
                Err(RecvError::Closed) => None,
                result => Some((result, subscriber)),
            }
        })
        .boxed()
    }
}

impl Drop for Subscriber {
    fn drop(&mut self) {
        drop(self.receiver.take());
        self.hub.release(&self.broadcasting, &self.identifier);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn delivers_to_every_subscriber_and_cleans_up() {
        let hub = Hub::new(8);
        let mut a = hub.subscribe("room", None);
        let mut b = hub.subscribe("room", None);
        assert_eq!(hub.broadcast("room", "1"), 2);
        assert_eq!(a.recv().await.unwrap(), "1");
        assert_eq!(b.recv().await.unwrap(), "1");
        drop(a);
        assert_eq!(hub.stream_count(), 1);
        drop(b);
        assert_eq!(hub.stream_count(), 0);
        assert_eq!(hub.broadcast("room", "2"), 0);
    }

    #[tokio::test]
    async fn wraps_payloads_once_per_identifier() {
        let hub = Hub::new(8);
        let identifier: Arc<str> = r#""{\"channel\":\"RoomChannel\"}""#.into();
        let mut a = hub.subscribe("room", Some(identifier.clone()));
        let mut b = hub.subscribe("room", Some(identifier.clone()));
        let mut other = hub.subscribe("room", Some(r#""other""#.into()));
        let mut raw = hub.subscribe("room", None);
        assert_eq!(hub.broadcast("room", r#"{"id":1}"#), 4);

        let (a, b) = (a.recv().await.unwrap(), b.recv().await.unwrap());
        assert_eq!(a, r#"{"identifier":"{\"channel\":\"RoomChannel\"}","message":{"id":1}}"#);
        assert_eq!(a.as_str().as_ptr(), b.as_str().as_ptr(), "one frame shared by both subscribers");
        assert_eq!(other.recv().await.unwrap(), r#"{"identifier":"other","message":{"id":1}}"#);
        assert_eq!(raw.recv().await.unwrap(), r#"{"id":1}"#);

        drop(other);
        drop(raw);
        assert_eq!(hub.state.lock().unwrap().streams["room"].len(), 1);
    }

    #[tokio::test]
    async fn slow_subscribers_see_lag_instead_of_gaps() {
        let hub = Hub::new(2);
        let mut slow = hub.subscribe("room", None);
        for i in 0..5 {
            hub.broadcast("room", &i.to_string());
        }
        assert_eq!(slow.recv().await, Err(RecvError::Lagged));
    }
}
