//! `rooms/show/_header_identity`: explicit recipient facts, safe for detached broadcasts.
use crate::{ViewContext, helpers as h};
use askama::Template;

#[derive(Template)]
#[template(path = "rooms/show/_header_identity.html")]
struct HeaderIdentityPartial<'a> {
    ctx: &'a ViewContext<'a>,
    header: &'a HeaderIdentity,
}

pub fn header_identity(ctx: &ViewContext, header: &HeaderIdentity) -> h::Html {
    h::raw(
        HeaderIdentityPartial { ctx, header }
            .render()
            .expect("header identity renders"),
    )
}
pub use campfire_presentation::rooms::header::*;
