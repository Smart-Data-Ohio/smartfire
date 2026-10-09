//! Ports of `reference/app/helpers` and the Rails built-in helpers Campfire's views use.
//!
//! Helpers return [`Html`] (askama's `Safe<String>`, the SafeBuffer equivalent), which templates
//! print unescaped; everything else is escaped by [`ErbEscaper`] exactly like ERB. Block helpers
//! (`link_to ... do`) live in [`filters`] and are used as askama filter blocks.
//!
//! The pieces the retained pages also need live in `campfire_view_kit` and are re-exported here,
//! so classic templates keep `h::name(...)` after `use crate::helpers as h;`.

pub mod application;
pub mod assets;
pub mod emoji;
pub mod filters;
pub mod icons;
pub mod rooms;
pub mod translations;
pub mod turbo;
pub mod users;

pub use campfire_view_kit::helpers::{forms, html, links, request_forgery, tag, text, url};

pub use application::*;
pub use assets::*;
pub use forms::*;
pub use html::*;
pub use icons::*;
pub use links::*;
pub use request_forgery::{csp_meta_tag, csrf_meta_tags, token_tag};
pub use rooms::*;
pub use tag::*;
pub use text::*;
pub use translations::*;
pub use turbo::*;
pub use url::*;
pub use users::*;

pub use crate::time::{distance_of_time_in_words, local_datetime_tag, time_ago_in_words};
/// Path helpers, re-exported so templates can write `h::routes::user_profile()`.
pub use campfire_routes as routes;
pub use campfire_view_kit::helpers::page_title_tag;
