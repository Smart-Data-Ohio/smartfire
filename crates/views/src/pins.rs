//! Plain view models for `app/views/rooms/pins`, including request-free broadcast partials.
use crate::{ViewContext, helpers as h};
use askama::Template;
pub trait ListRendering {
    fn unpin(&self, pin: &Pin) -> h::Html;
    fn datetime(&self, ctx: &ViewContext, pin: &Pin) -> h::Html;
}
impl ListRendering for List {
    fn unpin(&self, pin: &Pin) -> h::Html {
        h::button_to_form(
            &campfire_routes::message_pin(pin.message_id),
            h::attrs().method("delete").class("btn btn--plain"),
            h::attrs().data("turbo_frame", self.dom_id("pins_frame")),
            "Unpin",
        )
    }
    fn datetime(&self, ctx: &ViewContext, pin: &Pin) -> h::Html {
        crate::time::local_datetime_tag(&ctx.time_zone, pin.created_at, "time", h::attrs(), "")
    }
}

#[derive(Template)]
#[template(path = "rooms/pins/_list.html")]
pub struct ListPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub list: &'a List,
}
#[derive(Template)]
#[template(path = "rooms/pins/_count.html")]
pub struct CountPartial {
    pub room_id: i64,
    pub room_param_key: String,
    pub count: i64,
}
#[derive(Template)]
#[template(path = "rooms/pins/_panel.html")]
pub struct PanelPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub room_id: i64,
    pub room_param_key: &'a str,
    pub count: i64,
}
impl PanelPartial<'_> {
    pub fn dom_id(&self, prefix: &str) -> String {
        format!("{prefix}_{}_{}", self.room_param_key, self.room_id)
    }
    pub fn count_html(&self) -> h::Html {
        h::raw(CountPartial { room_id:self.room_id,room_param_key:self.room_param_key.into(),count:self.count }
            .render().expect("pin count renders"))
    }
}
impl CountPartial {
    pub fn dom_id(&self) -> String {
        format!("pins_count_{}_{}", self.room_param_key, self.room_id)
    }
}
#[derive(Template)]
#[template(path = "rooms/pins/index.html")]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub list: &'a List,
}
#[derive(Template)]
#[template(path = "messages/_pin_badge.html")]
pub struct BadgePartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub message: &'a Badge,
}

impl Index<'_> {
    pub fn list_html(&self) -> h::Html {
        h::raw(
            ListPartial {
                ctx: self.ctx,
                list: self.list,
            }
            .render()
            .expect("pins list renders"),
        )
    }
}
pub use campfire_presentation::pins::*;
