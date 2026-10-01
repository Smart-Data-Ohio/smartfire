//! `rooms/show/_header_identity`: explicit recipient facts, safe for detached broadcasts.
use crate::{ViewContext, helpers as h};
use askama::Template;

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct HeaderIdentity {
    pub id: i64,
    pub param_key: String,
    pub direct: bool,
    pub kind_label: String,
    pub display_name: String,
    pub icon: Option<h::AvatarIcon>,
}

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
