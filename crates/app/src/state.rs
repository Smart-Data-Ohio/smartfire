//! The parts of the app state whose behaviour lives in the layers above it: mail, sudo and
//! re-authentication seams, and the agent payload adapter's slot. [`crate::app::AppState`] holds
//! them; the controllers, concerns and presenters that use them re-export them where they were.

pub mod agent_payload;
pub mod mail;
pub mod sudo;
pub mod two_factor;
