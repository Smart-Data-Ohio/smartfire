//! The channels layer of the Campfire server: the Action Cable channels the browser subscribes
//! to, the broadcast sink that renders what they stream, and the background jobs (periodic and
//! ad hoc) that broadcast through it. The crates above mount the channels and enqueue the jobs
//! (plans/crate-split-plan.md). Modules keep their paths from the campfire crate, so
//! `channels::sink` lives at `campfire_channels::channels::sink`.

pub mod channels;
pub mod jobs;

// The app and web layers, under the paths this code used inside the campfire crate.
use campfire_app::{app, cable, huddle, integrations, net, queue};
use campfire_runtime::{active_storage, concerns, messaging};
use campfire_runtime::{controllers, mail};
