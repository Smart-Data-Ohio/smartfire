//! Rails.error's subscriber boundary. Default logs expose classification/context, never tokens.
use serde::Serialize;
use serde_json::Value;
use std::sync::{Arc, RwLock};
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Options {
    pub error_class: &'static str,
    pub handled: bool,
    pub severity: &'static str,
    pub source: &'static str,
    pub context: Value,
}
pub trait Subscriber: Send + Sync {
    fn report(&self, error: &anyhow::Error, options: &Options);
}
#[derive(Clone)]
pub struct Reporter(Arc<RwLock<Vec<Arc<dyn Subscriber>>>>);
struct Logger;
impl Subscriber for Logger {
    fn report(&self, _error: &anyhow::Error, options: &Options) {
        tracing::warn!(error_class=options.error_class,handled=options.handled,severity=options.severity,source=options.source,context=%options.context,"error service report");
    }
}
impl Default for Reporter {
    fn default() -> Self {
        let reporter = Self(Arc::new(RwLock::new(vec![])));
        reporter.subscribe(Arc::new(Logger));
        reporter
    }
}
impl Reporter {
    pub fn subscribe(&self, subscriber: Arc<dyn Subscriber>) {
        self.0
            .write()
            .unwrap_or_else(|p| p.into_inner())
            .push(subscriber);
    }
    pub fn report(&self, error: &anyhow::Error, error_class: &'static str, context: Value) {
        let options = Options {
            error_class,
            handled: true,
            severity: "warning",
            source: "application",
            context,
        };
        let subscribers = self.0.read().unwrap_or_else(|p| p.into_inner()).clone();
        for subscriber in subscribers {
            // A broken reporting service must not prevent the runner releasing its claim.
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                subscriber.report(error, &options)
            }))
            .is_err()
            {
                tracing::error!(error_class, "error subscriber panicked");
            }
        }
    }
}
