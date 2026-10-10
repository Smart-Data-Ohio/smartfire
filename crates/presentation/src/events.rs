pub mod forms;
pub mod pages;
// Event fragments use plain, preloaded facts; app/helpers/rooms/events_helper.rb.
use crate::helpers::{self as h};

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CardView {
    pub id: i64,
    pub room_id: i64,
    pub title: String,
    pub organizer_name: String,
    pub starts_at: crate::time::CalendarTime,
    pub ends_at: Option<crate::time::CalendarTime>,
    pub time_zone: String,
    pub series: bool,
    pub cancelled: bool,
    pub venue_name: Option<String>,
    pub meet_link: Option<String>,
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
impl CardView {
    pub fn path(&self) -> String {
        format!("/rooms/{}/events/{}", self.room_id, self.id)
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
}
