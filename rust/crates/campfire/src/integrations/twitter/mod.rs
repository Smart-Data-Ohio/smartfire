//! X cards' domain policy. HTML formatting lives in campfire_views::twitter.
pub mod fetcher;
#[allow(dead_code, reason = "X message-reference and card-render consumers are the next slice")]
pub mod post;
#[allow(dead_code, reason = "X message-reference consumers remain staged")]
pub mod urls;
