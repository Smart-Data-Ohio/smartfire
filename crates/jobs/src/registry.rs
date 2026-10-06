//! Which handler performs which job class, found by class name as Active Job finds a class.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use futures_util::future::BoxFuture;
use serde::de::DeserializeOwned;

use crate::kind::{Execution, JobError, JobKind, JobResult, RetryPolicy};

type Exhausted<C> = Arc<dyn Fn(C, serde_json::Value, u32, &anyhow::Error) + Send + Sync>;
type ExhaustedIn<C> = Arc<dyn Fn(&mut campfire_db::Tx<'_>, C, serde_json::Value, u32) -> campfire_db::Result<()> + Send + Sync>;

/// A registered job class, its type erased.
pub(crate) struct Kind<C> {
    pub queue: &'static str,
    pub version: u32,
    pub policy: RetryPolicy,
    pub exhausted: Option<Exhausted<C>>,
    pub exhausted_in: Option<ExhaustedIn<C>>,
    pub perform: Arc<
        dyn Fn(C, serde_json::Value, u32, Execution) -> BoxFuture<'static, JobResult> + Send + Sync,
    >,
}

impl<C> Clone for Kind<C> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue,
            version: self.version,
            policy: self.policy,
            exhausted: self.exhausted.clone(),
            exhausted_in: self.exhausted_in.clone(),
            perform: self.perform.clone(),
        }
    }
}

/// The job classes a runner performs, each with its handler, which gets the app's context `C`,
/// the job's arguments and its [`Execution`]. Each module registers its own classes.
pub struct Registry<C> {
    kinds: HashMap<&'static str, Kind<C>>,
}

impl<C> Default for Registry<C> {
    fn default() -> Self {
        Self {
            kinds: HashMap::new(),
        }
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
        let perform = move |context: C,
                            arguments: serde_json::Value,
                            version: u32,
                            execution: Execution|
              -> BoxFuture<'static, JobResult> {
            let job = decode::<J>(arguments, version);
            let handler = handler.clone();
            Box::pin(async move {
                match job {
                    Ok(job) => handler(context, job, execution).await,
                    Err(error) => Err(error),
                }
            })
        };
        let kind = Kind {
            queue: J::QUEUE,
            version: J::VERSION,
            policy: J::retry_policy(),
            exhausted: None,
            exhausted_in: None,
            perform: Arc::new(perform),
        };
        assert!(
            self.kinds.insert(J::CLASS, kind).is_none(),
            "{} is registered twice",
            J::CLASS
        );
        self
    }

    /// A `retry_on` exhaustion block, delivered only after the runner commits the
    /// terminal outcome for its still-owned claim. Ordinary discard/failure does not call it.
    pub fn on_exhausted<J, F>(&mut self, callback: F) -> &mut Self
    where
        J: JobKind + DeserializeOwned,
        F: Fn(C, J, &anyhow::Error) + Send + Sync + 'static,
    {
        let kind = self
            .kinds
            .get_mut(J::CLASS)
            .expect("register the job before its exhaustion block");
        assert!(
            kind.exhausted.is_none(),
            "{} has two exhaustion blocks",
            J::CLASS
        );
        kind.exhausted = Some(Arc::new(move |context, arguments, version, error| {
            if let Ok(job) = decode::<J>(arguments, version) {
                callback(context, job, error);
            }
        }));
        self
    }

    /// Durable exhaustion cleanup, committed atomically with the still-owned terminal
    /// queue outcome. Use this for a failure marker that must survive a worker restart.
    pub fn on_exhausted_in<J, F>(&mut self, callback: F) -> &mut Self
    where
        J: JobKind + DeserializeOwned,
        F: Fn(&mut campfire_db::Tx<'_>, C, J) -> campfire_db::Result<()> + Send + Sync + 'static,
    {
        let kind = self.kinds.get_mut(J::CLASS).expect("register the job before its exhaustion block");
        assert!(kind.exhausted_in.is_none(), "{} has two durable exhaustion blocks", J::CLASS);
        kind.exhausted_in = Some(Arc::new(move |tx, context, arguments, version| {
            match decode::<J>(arguments, version) {
                Ok(job) => callback(tx, context, job),
                Err(_) => Ok(()),
            }
        }));
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
fn decode<J: JobKind + DeserializeOwned>(
    arguments: serde_json::Value,
    version: u32,
) -> Result<J, JobError> {
    let arguments = match version {
        version if version == J::VERSION => arguments,
        version => J::upgrade(version, arguments)
            .map_err(|error| JobError::discard(anyhow::anyhow!("{}: {error}", J::CLASS)))?,
    };
    serde_json::from_value(arguments).map_err(|error| {
        JobError::discard(anyhow::anyhow!(
            "{} arguments don't deserialize: {error}",
            J::CLASS
        ))
    })
}
