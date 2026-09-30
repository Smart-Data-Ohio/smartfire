//! Smartfire mail, ported from app/mailboxes and app/mailers in our Rails reference.
pub mod config;
pub mod parse;
pub mod ruby;

pub mod outbound;

pub mod inbound;
pub mod jobs;
