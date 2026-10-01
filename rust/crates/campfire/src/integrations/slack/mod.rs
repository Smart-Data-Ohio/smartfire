//! Slack importer domain, matching `app/models/slack/*`. No HTML dependencies.
pub mod client;
pub mod markdown;

pub mod runner;
pub mod jobs;
pub mod users;
pub mod conversations;
pub mod writer;
pub mod store;
pub mod undoer;

#[cfg(test)]
mod sequence_tests;
