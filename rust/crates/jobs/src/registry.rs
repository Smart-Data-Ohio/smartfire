//! Which handler performs which job class, found by class name as Active Job finds a class.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use futures_util::future::BoxFuture;
use serde::de::DeserializeOwned;

use crate::kind::{Execution, JobError, JobKind, JobResult, RetryPolicy};

/// A registered job class, its type erased.
pub(crate) struct Kind<C> {
    pub queue: &'static str,
    pub version: u32,
    pub policy: RetryPolicy,
    pub perform: Arc<dyn Fn(C, serde_json::Value, u32, Execution) -> BoxFuture<'static, JobResult> + Send + Sync>,
}

impl<C> Clone for Kind<C> {
    fn clone(&self) -> Self {
        Self { queue: self.queue, version: self.version, policy: self.policy, perform: self.perform.clone() }
    }
}

/// The job classes a runner performs, each with its handler, which gets the app's context `C`,
/// the job's arguments and its [`Execution`]. Each module registers its own classes.
pub struct Registry<C> {
    kinds: HashMap<&'static str, Kind<C>>,
}

impl<C> Default for Registry<C> {
    fn default() -> Self {
        Self { kinds: HashMap::new() }
    }
}

impl<C: Send + 'static> Registry<C> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `J`'s handler. Panics if `J`'s class already has one: two handlers for one class
    /// is a bug.
    pub fn register<J, F, Fut>(&mut self, handler: F) -> &mut Self
    where
        J: JobKind + DeserializeOwned,
        F: Fn(C, J, Execution) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = JobResult> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let perform = move |context: C, arguments: serde_json::Value, version: u32, execution: Execution| -> BoxFuture<'static, JobResult> {
            let job = decode::<J>(arguments, version);
            let handler = handler.clone();
            Box::pin(async move {
                match job {
                    Ok(job) => handler(context, job, execution).await,
                    Err(error) => Err(error),
                }
            })
        };
        let kind = Kind { queue: J::QUEUE, version: J::VERSION, policy: J::retry_policy(), perform: Arc::new(perform) };
        assert!(self.kinds.insert(J::CLASS, kind).is_none(), "{} is registered twice", J::CLASS);
        self
    }

    pub(crate) fn get(&self, class: &str) -> Option<&Kind<C>> {
        self.kinds.get(class)
    }

    pub(crate) fn kinds(&self) -> impl Iterator<Item = (&'static str, &Kind<C>)> {
        self.kinds.iter().map(|(class, kind)| (*class, kind))
    }
}

/// The stored arguments as `J`, upgraded from an older payload version first. Arguments that
/// don't deserialize discard the job (`discard_on ActiveJob::DeserializationError`).
fn decode<J: JobKind + DeserializeOwned>(arguments: serde_json::Value, version: u32) -> Result<J, JobError> {
    let arguments = match version {
        version if version == J::VERSION => arguments,
        version => J::upgrade(version, arguments).map_err(|error| JobError::discard(anyhow::anyhow!("{}: {error}", J::CLASS)))?,
    };
    serde_json::from_value(arguments).map_err(|error| JobError::discard(anyhow::anyhow!("{} arguments don't deserialize: {error}", J::CLASS)))
}
