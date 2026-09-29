//! `Periodic::Runner` (`app/services/periodic/runner.rb`, run by `bin/periodic`): named tasks,
//! each with an interval, run by one loop. Each tick runs every task whose interval has elapsed
//! since its last successful run, in order; a task that fails (or panics) is logged without
//! stopping the others or the loop, and runs again on the next tick, since only a success counts
//! as a run. The loop sleeps the shortest interval between ticks.
//!
//! `Huddle::Reconciler` (`bin/huddle-reconcile`) is the same shape: its steps every
//! `HUDDLE_RECONCILE_INTERVAL` seconds, each isolated from the others' failures, so it's a
//! [`Periodic`] of its own.

use std::collections::HashMap;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use campfire_db::{Clock, Timestamp};
use futures_util::FutureExt as _;
use futures_util::future::BoxFuture;
use tokio::sync::watch;
use tokio::task::JoinHandle;

type Run<C> = Arc<dyn Fn(C) -> BoxFuture<'static, anyhow::Result<()>> + Send + Sync>;

/// `Periodic::Runner::Task`: a name, an interval, and an idempotent callable.
pub struct Task<C> {
    name: &'static str,
    interval: Duration,
    run: Run<C>,
}

impl<C: Send + 'static> Task<C> {
    pub fn new<F, Fut>(name: &'static str, interval: Duration, run: F) -> Self
    where
        F: Fn(C) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        Self { name, interval, run: Arc::new(move |context| Box::pin(run(context))) }
    }

    /// A task whose work is done once per process (`clear_bot_tokens_once`): once it has
    /// succeeded, its later runs do nothing.
    pub fn once<F, Fut>(name: &'static str, interval: Duration, run: F) -> Self
    where
        F: Fn(C) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let done = Arc::new(AtomicBool::new(false));
        let run = Arc::new(run);
        Self::new(name, interval, move |context| {
            let (done, run) = (done.clone(), run.clone());
            async move {
                if !done.load(Ordering::Acquire) {
                    run(context).await?;
                    done.store(true, Ordering::Release);
                }
                Ok(())
            }
        })
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }
}

/// The loop and its tasks.
pub struct Periodic<C> {
    /// For logs: `"Periodic"` (`"Periodic #{task.name} failed"`), `"Huddle reconciliation"`.
    label: &'static str,
    tasks: Vec<Task<C>>,
    last_run: HashMap<&'static str, Timestamp>,
}

impl<C: Clone + Send + 'static> Periodic<C> {
    pub fn new(label: &'static str) -> Self {
        Self { label, tasks: Vec::new(), last_run: HashMap::new() }
    }

    /// Appends a task. Panics on a second task with the same name: names key the last runs.
    pub fn task(&mut self, task: Task<C>) -> &mut Self {
        assert!(self.tasks.iter().all(|t| t.name != task.name), "periodic task {:?} is registered twice", task.name);
        self.tasks.push(task);
        self
    }

    pub fn tasks(&self) -> impl Iterator<Item = &Task<C>> {
        self.tasks.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// `tick_interval`: the shortest task interval.
    pub fn tick_interval(&self) -> Option<Duration> {
        self.tasks.iter().map(|task| task.interval).min()
    }

    /// One pass over the due tasks, in order (`tick`). Returns the names of the tasks that ran,
    /// whether or not they succeeded.
    pub async fn tick(&mut self, context: C, now: Timestamp) -> Vec<&'static str> {
        let mut ran = Vec::new();
        for task in &self.tasks {
            if !due(self.last_run.get(task.name).copied(), task.interval, now) {
                continue;
            }
            if run_task(self.label, task, context.clone()).await {
                self.last_run.insert(task.name, now);
            }
            ran.push(task.name);
        }
        ran
    }

    /// `run`: ticks, then sleeps the tick interval, until `stopping` (checked between ticks; a
    /// tick in progress finishes). Does nothing without tasks.
    pub fn spawn(mut self, context: C, clock: Arc<dyn Clock>, mut stopping: watch::Receiver<bool>) -> JoinHandle<()>
    where
        C: Sync,
    {
        tokio::spawn(async move {
            let Some(interval) = self.tick_interval() else { return };
            loop {
                if *stopping.borrow() {
                    return;
                }
                self.tick(context.clone(), clock.now()).await;
                tokio::select! {
                    () = tokio::time::sleep(interval) => {}
                    () = stopped(&mut stopping) => return,
                }
            }
        })
    }
}

/// Resolves once `stopping` turns true; never, if its sender is dropped.
async fn stopped(stopping: &mut watch::Receiver<bool>) {
    let closed = stopping.wait_for(|stopping| *stopping).await.is_err();
    if closed {
        std::future::pending::<()>().await;
    }
}

fn due(last_run: Option<Timestamp>, interval: Duration, now: Timestamp) -> bool {
    match last_run {
        None => true,
        Some(last) => last.since(jiff::SignedDuration::try_from(interval).unwrap_or(jiff::SignedDuration::MAX)) <= now,
    }
}

/// Runs one task, isolating the loop from its error or panic. Returns whether it succeeded.
async fn run_task<C>(label: &str, task: &Task<C>, context: C) -> bool {
    match AssertUnwindSafe((task.run)(context)).catch_unwind().await {
        Ok(Ok(())) => true,
        Ok(Err(error)) => {
            tracing::error!("{label} {} failed: {error:#}", task.name);
            false
        }
        Err(panic) => {
            let message = panic.downcast_ref::<&str>().copied().or_else(|| panic.downcast_ref::<String>().map(String::as_str)).unwrap_or("Box<dyn Any>");
            tracing::error!("{label} {} panicked: {message}", task.name);
            false
        }
    }
}

/// `Periodic::Runner.parse_interval!`: seconds as a positive, finite number.
pub fn parse_interval(value: &str, name: &str) -> anyhow::Result<Duration> {
    match value.trim().parse::<f64>() {
        Ok(seconds) if seconds.is_finite() && seconds > 0.0 => {
            Duration::try_from_secs_f64(seconds).map_err(|_| anyhow::anyhow!("{name} must be a positive finite number"))
        }
        _ => anyhow::bail!("{name} must be a positive finite number"),
    }
}

/// An interval from the environment variable `name` (`ENV.fetch(name, default)`), parsed by
/// [`parse_interval`]. `lookup` reads the environment.
pub fn interval_from_env(lookup: impl Fn(&str) -> Option<String>, name: &str, default: Duration) -> anyhow::Result<Duration> {
    match lookup(name) {
        Some(value) => parse_interval(&value, name),
        None => Ok(default),
    }
}
