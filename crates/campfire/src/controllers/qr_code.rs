//! `controllers::qr_code` lives in the campfire_people crate (plans/crate-split-plan.md). This
//! module re-exports it under its old path and mounts the tests that still need the whole app.

pub use campfire_people::controllers::qr_code::*;

#[cfg(test)]
mod tests;
