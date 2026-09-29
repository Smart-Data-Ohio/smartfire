//! The enqueueing side: where a class's jobs go, and waking their queue once they're committed.

use std::collections::HashMap;
use std::sync::Arc;

use campfire_db::{Database, JobRequest, Tx};
use tokio::sync::Notify;

use crate::registry::Registry;
use crate::runner::RunnerConfig;
use crate::store;

/// Where a class's jobs go, from its [`JobKind`](crate::JobKind).
#[derive(Debug, Clone, Copy)]
struct Route {
    queue: &'static str,
    version: u32,
}

/// Enqueues jobs, and wakes the runner's queues. Cheap to clone. The app's
/// [`EventSink`](campfire_db::EventSink) calls [`JobQueue::enqueue`] from `persist` and
/// [`JobQueue::wake`] from `emit`.
#[derive(Clone)]
pub struct JobQueue {
    routes: Arc<HashMap<&'static str, Route>>,
    wakers: Arc<HashMap<String, Arc<Notify>>>,
}

/// Where a job of a class nobody registered goes. It fails when a runner claims it ("no handler").
const UNREGISTERED_QUEUE: &str = "default";

impl JobQueue {
    /// The queue for `registry`'s classes, running on `config`'s queues. Fails when a class's
    /// queue isn't one of them: its jobs would never run.
    pub fn new<C: Send + 'static>(registry: &Registry<C>, config: &RunnerConfig) -> anyhow::Result<Self> {
        let wakers: HashMap<String, Arc<Notify>> = config.queues.iter().map(|queue| (queue.name.clone(), Arc::new(Notify::new()))).collect();
        let mut routes = HashMap::new();
        for (class, kind) in registry.kinds() {
            anyhow::ensure!(wakers.contains_key(kind.queue), "{class} runs on the {:?} queue, which has no workers", kind.queue);
            routes.insert(class, Route { queue: kind.queue, version: kind.version });
        }
        Ok(Self { routes: Arc::new(routes), wakers: Arc::new(wakers) })
    }

    /// `perform_later`: inserts the job's row in `tx`'s transaction, due after its `wait`. It
    /// becomes visible to the runner when the transaction commits; call [`JobQueue::wake`] then.
    pub fn enqueue(&self, tx: &Tx<'_>, request: &JobRequest) -> campfire_db::Result<i64> {
        let route = self.route(request.class);
        let now = tx.now();
        let run_at = match request.wait {
            Some(wait) => now.since(jiff::SignedDuration::try_from(wait).unwrap_or(jiff::SignedDuration::MAX)),
            None => now,
        };
        let job = store::NewJob { queue: route.queue, class: request.class, arguments: &request.arguments, version: route.version, run_at };
        store::insert(tx.conn(), job, now)
    }

    /// Enqueues a job in a write of its own, and wakes its queue.
    pub async fn perform_later(&self, db: &Database, request: JobRequest) -> campfire_db::Result<i64> {
        let queue = self.clone();
        let class = request.class;
        let id = db.write(move |tx| queue.enqueue(tx, &request)).await?;
        self.wake(class);
        Ok(id)
    }

    /// Tells `class`'s queue a job was committed, so it claims it now rather than at its next poll.
    pub fn wake(&self, class: &str) {
        self.wake_queue(self.route(class).queue);
    }

    pub(crate) fn wake_queue(&self, queue: &str) {
        if let Some(waker) = self.wakers.get(queue) {
            waker.notify_one();
        }
    }

    pub(crate) fn waker(&self, queue: &str) -> Arc<Notify> {
        self.wakers.get(queue).cloned().unwrap_or_default()
    }

    fn route(&self, class: &str) -> Route {
        match self.routes.get(class) {
            Some(route) => *route,
            None => {
                tracing::error!(job = class, "enqueueing a job class that has no handler");
                Route { queue: UNREGISTERED_QUEUE, version: 1 }
            }
        }
    }
}
