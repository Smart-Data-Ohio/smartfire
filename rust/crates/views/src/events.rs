//! Event fragments use plain, preloaded facts; app/helpers/rooms/events_helper.rb.
use crate::{
    helpers::{self as h, filters},
    time::Zone,
};
use askama::Template;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CardView {
    pub id: i64,
    pub room_id: i64,
    pub title: String,
    pub organizer_name: String,
    pub starts_at: jiff::Timestamp,
    pub ends_at: Option<jiff::Timestamp>,
    pub time_zone: String,
    pub series: bool,
    pub cancelled: bool,
    pub venue_name: Option<String>,
    pub meet_link: Option<String>,
}
impl CardView {
    pub fn path(&self) -> String {
        format!("/rooms/{}/events/{}", self.room_id, self.id)
    }
    fn datetime(&self, viewer_zone: &Zone, time: jiff::Timestamp) -> h::Html {
        let zone = Zone::for_user(Some(&self.time_zone));
        h::local_datetime_tag(
            viewer_zone,
            time,
            "datetime",
            h::attrs(),
            &h::escape(&zone.format(time, "%B %-d, %Y at %-I:%M %p")),
        )
    }
    pub fn start_tag(&self, viewer_zone: &Zone) -> h::Html {
        self.datetime(viewer_zone, self.starts_at)
    }
    pub fn end_tag(&self, viewer_zone: &Zone) -> h::Html {
        self.ends_at
            .map(|time| self.datetime(viewer_zone, time))
            .unwrap_or_else(h::empty)
    }
    pub fn frame_id(&self, message_id: &str) -> String {
        format!("response_for_message_{message_id}_event_{}", self.id)
    }
    pub fn attendance_path(&self, message_id: &str) -> String {
        format!(
            "{}/attendance?message_id={}",
            self.path(),
            h::url::cgi_escape(message_id)
        )
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

#[derive(Clone, Debug, serde::Deserialize)]
pub struct AttendanceView {
    pub event_id: i64,
    pub room_id: i64,
    pub message_id: Option<String>,
    #[serde(default)]
    pub message_id_input: Option<String>,
    pub current_response: Option<String>,
    pub going: i64,
    pub maybe: i64,
    pub respondable: bool,
    pub cancelled: bool,
    pub apply_to_future: bool,
    pub alert: Option<String>,
}
impl AttendanceView {
    pub fn message_input_value(&self) -> Option<&str> {
        self.message_id_input
            .as_deref()
            .or(self.message_id.as_deref())
    }
    pub fn frame_id(&self) -> String {
        format!(
            "response_for_message_{}_event_{}",
            self.message_id.as_deref().unwrap_or(""),
            self.event_id
        )
    }
    pub fn path(&self) -> String {
        format!(
            "/rooms/{}/events/{}/attendance",
            self.room_id, self.event_id
        )
    }
    pub fn going_count(&self) -> String {
        pages::plural(self.going, "going")
    }
    pub fn maybe_count(&self) -> String {
        format!(
            "{} {}",
            self.maybe,
            if self.maybe == 1 { "maybe" } else { "maybes" }
        )
    }
    pub fn response_label(&self) -> String {
        self.current_response
            .as_ref()
            .map(|s| {
                let mut chars = s.chars();
                chars
                    .next()
                    .into_iter()
                    .flat_map(char::to_uppercase)
                    .chain(chars)
                    .collect()
            })
            .unwrap_or_else(|| "No response yet".into())
    }
    pub fn future_checkbox(&self) -> h::Html {
        h::legacy_tag(
            "input",
            h::attrs()
                .type_("checkbox")
                .name("apply_to_future")
                .id("apply_to_future")
                .value("1"),
        )
    }
    pub fn response_button(&self, label: &str, value: &str, class: &str) -> h::Html {
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
