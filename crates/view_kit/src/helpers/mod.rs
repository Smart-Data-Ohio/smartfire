//! Tag, form, CSRF, escape, route, and translation helpers shared by both shells.

pub mod filters;
pub mod forms;
pub mod head;
pub mod html;
pub mod links;
pub mod request_forgery;
pub mod tag;
pub mod text;
pub mod translations;
mod translations_table;
pub mod url;

pub use forms::*;
pub use head::{
    auth_script_tag, auth_stylesheet_tag, page_title_tag, turbo_page_requires_reload_tag,
};
pub use html::*;
pub use links::*;
pub use request_forgery::{csp_meta_tag, csrf_meta_tags, token_tag};
pub use tag::*;
pub use text::*;
pub use translations::*;
pub use url::*;

/// Path helpers, so a template can write `h::routes::session()`.
pub use campfire_routes as routes;
