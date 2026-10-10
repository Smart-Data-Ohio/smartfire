//! Full event pages consume preloaded facts, matching rooms/events/*.erb.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
pub trait PageEventRendering {
    fn row(&self, ctx: &ViewContext) -> h::Html;
    fn live_dot(&self, venue: &VenueView, row: bool) -> h::Html;
    fn cancel_button(&self) -> h::Html;
    fn future_checkbox(&self) -> h::Html;
    fn response_button(&self, name: &str, value: &str, class: &str) -> h::Html;
}
impl PageEventRendering for PageEvent {
    fn row(&self, ctx: &ViewContext) -> h::Html {
        h::raw(format!(
            "{}\n",
            Row { ctx, event: self }.render().expect("event row")
        ))
    }
    fn live_dot(&self, venue: &VenueView, row: bool) -> h::Html {
        let target = if row {
            format!("venue_live_dot_event_{}", self.card.id)
        } else {
            format!("event_stage_live_rooms_stage_{}", venue.id)
        };
        h::raw(format!(
            "{}\n",
            LiveDot {
                venue,
                target: &target
            }
            .render()
            .expect("event venue dot")
        ))
    }
    fn cancel_button(&self) -> h::Html {
        h::button_to_form(
            &self.cancel_path(),
            h::attrs()
                .attr("method", "patch")
                .class("btn btn--negative"),
            h::attrs().data(
                "turbo_confirm",
                "Cancel this event? Attendees will be notified.",
            ),
            "Cancel event",
        )
    }
    fn future_checkbox(&self) -> h::Html {
        h::legacy_tag(
            "input",
            h::attrs()
                .type_("checkbox")
                .name("apply_to_future")
                .id("apply_to_future")
                .value("1"),
        )
    }
    fn response_button(&self, name: &str, value: &str, class: &str) -> h::Html {
        h::button_tag(h::attrs().name("response").value(value).class(class), name)
    }
}

#[derive(Template)]
#[template(path = "rooms/events/_event.html")]
pub struct Row<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub event: &'a PageEvent,
}
#[derive(Template)]
#[template(path = "rooms/events/_venue_live_dot.html")]
pub struct LiveDot<'a> {
    pub venue: &'a VenueView,
    pub target: &'a str,
}
#[derive(Template)]
#[template(path="rooms/events/index.html",blocks=["head","content"])]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub view: &'a IndexView,
}
impl Page for Index<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!("Events in {}", self.view.room_name))
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar work-workspace")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
#[derive(Template)]
#[template(path="rooms/events/show.html",blocks=["head","content"])]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub view: &'a ShowView,
}
impl Page for Show<'_> {
    fn page_title(&self) -> Option<String> {
        Some(self.view.event.card.title.clone())
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar work-workspace")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
pub use campfire_presentation::events::pages::*;

use crate::rendering::*;
