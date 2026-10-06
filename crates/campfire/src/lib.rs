//! The Campfire server: controllers, channels, jobs and integrations wired over the crates. The
//! `campfire` binary (main.rs) runs it; `campfire_tests` boots it in its tests.

pub mod admin;
pub mod controllers;
// Boot, the HTTP stack and the binary's commands.
pub mod server;

// The lower layers' modules, at the paths they had in this crate.
use campfire_app::{app, config, errors, integrations, net, queue, security};
#[cfg(any(test, feature = "test-support"))]
use campfire_app::test_support;
use campfire_channels::{channels, jobs};
use campfire_web::{active_storage, concerns, mail, rich_text};
