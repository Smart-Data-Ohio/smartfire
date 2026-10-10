//! Event fragments use plain, preloaded facts; app/helpers/rooms/events_helper.rb.
use crate::{
    helpers::{self as h, filters},
    time::Zone,
};
use askama::Template;
pub trait CardViewRendering {
    fn datetime(&self, viewer_zone: &Zone, time: crate::time::CalendarTime) -> h::Html;
    fn start_tag(&self, viewer_zone: &Zone) -> h::Html;
    fn end_tag(&self, viewer_zone: &Zone) -> h::Html;
}
impl CardViewRendering for CardView {
    fn datetime(&self, viewer_zone: &Zone, time: crate::time::CalendarTime) -> h::Html {
        let zone = Zone::for_user(Some(&self.time_zone));
        crate::time::local_datetime_tag_iso(
            &time.iso8601(viewer_zone),
            "datetime",
            h::attrs(),
            &h::escape(&time.format(&zone, "%B %-d, %Y at %-I:%M %p")),
        )
    }
    fn start_tag(&self, viewer_zone: &Zone) -> h::Html {
        self.datetime(viewer_zone, self.starts_at)
    }
    fn end_tag(&self, viewer_zone: &Zone) -> h::Html {
        self.ends_at
            .map(|time| self.datetime(viewer_zone, time))
            .unwrap_or_else(h::empty)
    }
}

#[derive(Template)]
#[template(path = "rooms/events/_card.html")]
pub struct Card<'a> {
    pub event: &'a CardView,
    pub message_id: &'a str,
    pub viewer_zone: &'a Zone,
}

#[derive(Template)]
#[template(path = "rooms/events/_cards.html")]
pub struct Cards<'a> {
    pub message_key: &'a str,
    /// Each string is this partial's rendered loop body, including leading indentation.
    pub entries: &'a [String],
}
pub trait AttendanceViewRendering {
    fn future_checkbox(&self) -> h::Html;
    fn response_button(&self, label: &str, value: &str, class: &str) -> h::Html;
}
impl AttendanceViewRendering for AttendanceView {
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
    fn response_button(&self, label: &str, value: &str, class: &str) -> h::Html {
        h::button_tag(h::attrs().name("response").value(value).class(class), label)
    }
}

#[derive(Template)]
#[template(path = "rooms/events/attendances/show.html")]
pub struct Attendance<'a> {
    pub view: &'a AttendanceView,
}

/// ERB's collection loop adds its own indentation and newline around each partial.
pub fn card_entries(events: &[CardView], message_id: &str, zone: &Zone) -> Vec<String> {
    events
        .iter()
        .map(|event| {
            format!(
                "      {}\n",
                Card {
                    event,
                    message_id,
                    viewer_zone: zone
                }
                .render()
                .expect("event card renders")
            )
        })
        .collect()
}

pub mod pages;

pub mod forms;
pub use campfire_presentation::events::*;
