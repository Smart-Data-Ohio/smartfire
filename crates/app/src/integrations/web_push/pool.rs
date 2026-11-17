//! `WebPush::Pool` (reference/lib/web_push/pool.rb) with the invalid-subscription handler from
//! reference/config/initializers/web_push.rb: up to 50 deliveries at once and 10,000 waiting
//! (more are dropped, like `Concurrent::RejectedExecutionError`, and logged), and one worker that
//! destroys expired or unusable subscriptions in order.

use std::collections::{HashMap, hash_map::Entry};
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use campfire_db::{Connection, PushPayload, PushSubscription, Timestamp};
use tokio::sync::{Notify, Semaphore};

use super::{Notification, VapidConfig};
use crate::net::Network;

/// `Concurrent::ThreadPoolExecutor.new(max_threads: 50, max_queue: 10000)`
const MAX_THREADS: usize = 50;
const MAX_QUEUE: usize = 10_000;

type Handler = Box<dyn Fn(i64) -> Result<(), String> + Send>;

#[derive(Clone)]
pub struct Pool {
    inner: Arc<Inner>,
}

struct Inner {
    net: Network,
    vapid: VapidConfig,
    runtime: tokio::runtime::Handle,
    running: Semaphore,
    pending: AtomicUsize,
    /// Deliveries dropped since the queue was last accepting them.
    dropped: AtomicUsize,
    idle: Notify,
    shut_down: AtomicBool,
    invalidations: Mutex<Option<mpsc::Sender<i64>>>,
    invalidator: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Pool {
    /// Call from inside the Tokio runtime deliveries should run on. `invalid_subscription_handler`
    /// is `Push::Subscription.find_by(id:)&.destroy`; it runs on the pool's own thread, so it may
    /// block (e.g. `Database::write_blocking`).
    pub fn new<F, E>(net: Network, vapid: VapidConfig, invalid_subscription_handler: F) -> Self
    where
        F: Fn(i64) -> Result<(), E> + Send + 'static,
        E: Display,
    {
        let handler: Handler =
            Box::new(move |id| invalid_subscription_handler(id).map_err(|e| e.to_string()));
        let (sender, receiver) = mpsc::channel::<i64>();
        let invalidator = std::thread::Builder::new()
            .name("web_push-invalidation".into())
            .spawn(move || {
                for id in receiver {
                    tracing::info!("Destroying push subscription: {id}");
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(id))) {
                        Ok(Ok(())) => {}
                        Ok(Err(message)) => {
                            tracing::error!(
                                "Error in WebPush::Pool.invalid_subscription_handler: {message}"
                            )
                        }
                        Err(_) => tracing::error!(
                            "Error in WebPush::Pool.invalid_subscription_handler: panic"
                        ),
                    }
                }
            })
            .expect("spawn the web push invalidation thread");

        Self {
            inner: Arc::new(Inner {
                net,
                vapid,
                runtime: tokio::runtime::Handle::current(),
                running: Semaphore::new(MAX_THREADS),
                pending: AtomicUsize::new(0),
                dropped: AtomicUsize::new(0),
                idle: Notify::new(),
                shut_down: AtomicBool::new(false),
                invalidations: Mutex::new(Some(sender)),
                invalidator: Mutex::new(Some(invalidator)),
            }),
        }
    }

    /// `queue(payload, subscriptions)`: in id order (`find_each`), each subscription's
    /// notification is built here (counting the badge once per user) and delivered on the pool.
    pub fn queue(
        &self,
        conn: &Connection,
        payload: &PushPayload,
        mut subscriptions: Vec<PushSubscription>,
        now: Timestamp,
    ) -> campfire_db::Result<()> {
        subscriptions.sort_by_key(|s| s.id);
        let mut badges = HashMap::new();
        for subscription in &subscriptions {
            let badge = match badges.entry(subscription.user_id) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => *entry.insert(subscription.badge(conn, now)?),
            };
            self.deliver_later(Notification::with_badge(subscription, payload, badge));
        }
        Ok(())
    }

    pub fn deliver_later(&self, notification: Notification) {
        let inner = &self.inner;
        if inner.shut_down.load(Ordering::SeqCst) {
            tracing::warn!("WebPush::Pool is shut down, dropping a notification");
            return;
        }
        if inner.pending.fetch_add(1, Ordering::SeqCst) >= MAX_THREADS + MAX_QUEUE {
            inner.pending.fetch_sub(1, Ordering::SeqCst);
            if inner.dropped.fetch_add(1, Ordering::SeqCst) == 0 {
                tracing::error!("WebPush::Pool is full, dropping notifications");
            }
            return;
        }
        let dropped = inner.dropped.swap(0, Ordering::SeqCst);
        if dropped > 0 {
            tracing::error!("WebPush::Pool dropped {dropped} notifications while it was full");
        }
        let pool = self.inner.clone();
        inner.runtime.spawn(async move {
            if let Ok(_permit) = pool.running.acquire().await {
                pool.deliver(&notification).await;
            }
            if pool.pending.fetch_sub(1, Ordering::SeqCst) == 1 {
                pool.idle.notify_waiters();
            }
        });
    }

    /// Waits (up to a second, like `wait_for_termination(1)`) for queued deliveries, then stops
    /// the invalidation worker once it has drained.
    pub async fn shutdown(&self) {
        let inner = &self.inner;
        inner.shut_down.store(true, Ordering::SeqCst);
        let drained = async {
            loop {
                let idle = inner.idle.notified();
                if inner.pending.load(Ordering::SeqCst) == 0 {
                    break;
                }
                idle.await;
            }
        };
        let _ = tokio::time::timeout(Duration::from_secs(1), drained).await;
        inner.invalidations.lock().unwrap().take();
        let worker = inner.invalidator.lock().unwrap().take();
        if let Some(worker) = worker {
            let _ = tokio::task::spawn_blocking(move || worker.join()).await;
        }
    }

    /// Queued or running deliveries.
    #[cfg(any(test, feature = "test-support"))]
    pub fn pending(&self) -> usize {
        self.inner.pending.load(Ordering::SeqCst)
    }
}

impl Inner {
    async fn deliver(&self, notification: &Notification) {
        match notification.deliver(&self.net, &self.vapid).await {
            Ok(_) => {}
            Err(error) if error.invalidates_subscription() => {
                self.invalidate_subscription_later(notification.subscription.id)
            }
            Err(error) => tracing::error!(
                "Error in WebPush::Pool.deliver: {} {}",
                error.class_name(),
                error
            ),
        }
    }

    fn invalidate_subscription_later(&self, id: i64) {
        if let Some(sender) = self.invalidations.lock().unwrap().as_ref() {
            let _ = sender.send(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! { static BADGE_QUERIES: Cell<usize> = const { Cell::new(0) }; }

    #[tokio::test]
    async fn app_badge_is_counted_once_per_user_per_queue() {
        let mut conn = Connection::open_in_memory().unwrap();
        campfire_db::schema::prepare(&mut conn, "production", &campfire_db::SystemClock).unwrap();
        conn.execute_batch("INSERT INTO users(id,name,created_at,updated_at) VALUES(1,'One','2035-01-01','2035-01-01'),(2,'Two','2035-01-01','2035-01-01');").unwrap();
        let key = p256::ecdsa::SigningKey::from_slice(&[1; 32]).unwrap();
        let vapid = VapidConfig::new(
            "mailto:test@example.com",
            &super::super::encode64_nopad(key.verifying_key().to_sec1_point(false).as_bytes()),
            &super::super::encode64_nopad(&key.to_bytes()),
        )
        .unwrap();
        let pool = Pool::new(Network::system(), vapid, |_| Ok::<_, String>(()));
        let payload = PushPayload::new("Test".into(), "Message".into(), "/app/".into(), None);
        let subscriptions = [1, 2, 1, 2, 1]
            .map(|user| PushSubscription::new(user, None, None, None, None))
            .to_vec();
        BADGE_QUERIES.set(0);
        conn.trace_v2(
            rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
            Some(|event: rusqlite::trace::TraceEvent<'_>| {
                if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event
                    && sql.starts_with("WITH \"bounds\"")
                {
                    BADGE_QUERIES.set(BADGE_QUERIES.get() + 1);
                }
            }),
        )
        .unwrap();
        let now = Timestamp::parse_db("2035-01-01T12:00:00Z").unwrap();
        pool.queue(&conn, &payload, subscriptions.clone(), now)
            .unwrap();
        let first = BADGE_QUERIES.get();
        pool.queue(&conn, &payload, subscriptions, now).unwrap();
        let second = BADGE_QUERIES.get();
        pool.shutdown().await;
        assert_eq!(first, 2, "one count per user, regardless of device count");
        assert_eq!(second, 4, "a later push must count again");
    }
}
