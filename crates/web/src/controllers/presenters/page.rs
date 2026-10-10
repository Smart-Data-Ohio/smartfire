//! Retained page helper and shared request facts.

/// Render a retained page or its frame on the independent retained shell.
#[macro_export]
macro_rules! retained_page {
    ($c:expr, $status:expr, |$ctx:ident| $page:expr) => {
        $crate::controllers::presenters::view_context::retained_page_or_frame(
            $c,
            $status,
            |$ctx| askama::Template::render(&$page),
            |$ctx| {
                let page = $page;
                campfire_retained::layouts::frame($ctx, page.as_head(), page.as_content())
            },
        )
    };
}
pub use crate::retained_page;


pub use campfire_runtime::context::*;
