//! Channel calendar pages and writes, using the classic event forms and domain callbacks.

use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

use crate::{AttendanceResponse, RoomKind, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum EventRecurrenceRule {
    Daily,
    Weekly,
    Biweekly,
    Monthly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EventScope {
    ThisEvent,
    ThisAndFollowing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventVenue {
    pub room_id: i64,
    pub name: String,
    pub kind: RoomKind,
    pub member: bool,
    pub live_user: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventCounts {
    pub going: i64,
    pub maybe: i64,
    pub declined: i64,
}

/// The facts shown by each classic calendar row. Instants are UTC; `zoneLabel` is in the
/// event's scheduled zone, including its start/end clock times and abbreviation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelEvent {
    pub id: i64,
    pub room_id: i64,
    pub title: String,
    pub starts_at: Timestamp,
    pub ends_at: Option<Timestamp>,
    pub time_zone: String,
    pub zone_label: String,
    pub organizer_name: String,
    pub venue: Option<EventVenue>,
    pub counts: EventCounts,
    pub recurrence_label: Option<String>,
    pub remaining_occurrences: Option<i64>,
    pub cancelled: bool,
    pub series: bool,
}

/// `GET /api/v1/rooms/:roomId/events`. Series are grouped exactly as on the classic list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventList {
    pub room_id: i64,
    pub room_name: String,
    pub room_kind: RoomKind,
    pub may_create: bool,
    pub upcoming: Vec<ChannelEvent>,
    pub past: Vec<ChannelEvent>,
    pub cancelled: Vec<ChannelEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventAttendee {
    pub name: String,
    pub response: AttendanceResponse,
}

/// The classic show page's facts. `descriptionHtml` uses its sanitized `simple_format`
/// renderer; `recurrenceUntil` is the classic display date, not the form's ISO date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventDetail {
    pub room_id: i64,
    pub room_name: String,
    pub event: ChannelEvent,
    pub description_html: Option<String>,
    pub manageable: bool,
    pub respondable: bool,
    pub current_response: Option<AttendanceResponse>,
    pub head: bool,
    pub can_apply_to_future: bool,
    pub previous_occurrence_id: Option<i64>,
    pub next_occurrence_id: Option<i64>,
    pub recurrence_phrase: Option<String>,
    pub recurrence_until: Option<String>,
    pub meet_link: Option<String>,
    pub calendar_copy: bool,
    pub attendees: Vec<EventAttendee>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventVenueOption {
    pub room_id: i64,
    pub name: String,
    pub kind: RoomKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventRepeatOption {
    pub value: Option<EventRecurrenceRule>,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventLimits {
    pub title_max_length: i64,
    pub max_occurrences: i64,
    pub max_recurrence_years: i64,
}

/// Editable values. Start/end are `datetime-local` strings in `timeZone`; until is YYYY-MM-DD.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventValues {
    pub title: String,
    pub description: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub time_zone: String,
    pub venue_room_id: Option<i64>,
    pub recurrence_rule: Option<EventRecurrenceRule>,
    pub recurrence_until: Option<String>,
    pub meet_link_requested: bool,
    pub meet_link: Option<String>,
}

/// `GET .../events/new` and `GET .../events/:id/edit`. New defaults to UTC; a screen may replace
/// it with the browser's zone before submitting, as the classic form's JavaScript does.
/// `meetAvailable` means the request checkbox is shown, even without a connected Google account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventForm {
    pub room_id: i64,
    pub room_name: String,
    pub event_id: Option<i64>,
    pub values: EventValues,
    pub venues: Vec<EventVenueOption>,
    pub meet_available: bool,
    pub repeat_options: Vec<EventRepeatOption>,
    pub limits: EventLimits,
    pub series: bool,
    pub head: bool,
    pub rule_editable: bool,
    pub scope_options: Vec<EventScope>,
}

/// `POST .../events`. Nullable times preserve classic validation for missing/invalid inputs.
/// Rule is a string so an unsupported value receives the classic field validation error.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct CreateEvent {
    pub title: String,
    pub description: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub time_zone: Option<String>,
    pub venue_room_id: Option<i64>,
    pub recurrence_rule: Option<String>,
    pub recurrence_until: Option<String>,
    pub meet_link_requested: bool,
}

/// `PATCH .../events/:id` submits classic form controls. Omit hidden controls to preserve their
/// values. Start/end are parsed even when absent, so screens must submit their displayed times.
/// Unknown/blank scope falls back to this event.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateEvent {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<Option<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<Option<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub venue_room_id: Option<Option<i64>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_rule: Option<Option<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_until: Option<Option<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meet_link_requested: Option<bool>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_scope: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[derive(Default)]
struct UpdateEventFields {
    title: Option<String>,
    #[serde(deserialize_with = "present")]
    description: Option<Option<String>>,
    #[serde(deserialize_with = "present")]
    starts_at: Option<Option<String>>,
    #[serde(deserialize_with = "present")]
    ends_at: Option<Option<String>>,
    time_zone: Option<String>,
    #[serde(deserialize_with = "present")]
    venue_room_id: Option<Option<i64>>,
    #[serde(deserialize_with = "present")]
    recurrence_rule: Option<Option<String>>,
    #[serde(deserialize_with = "present")]
    recurrence_until: Option<Option<String>>,
    meet_link_requested: Option<bool>,
    update_scope: Option<String>,
}

impl<'de> Deserialize<'de> for UpdateEvent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let fields = UpdateEventFields::deserialize(deserializer)?;
        Ok(Self {
            title: fields.title,
            description: fields.description,
            starts_at: fields.starts_at,
            ends_at: fields.ends_at,
            time_zone: fields.time_zone,
            venue_room_id: fields.venue_room_id,
            recurrence_rule: fields.recurrence_rule,
            recurrence_until: fields.recurrence_until,
            meet_link_requested: fields.meet_link_requested,
            update_scope: fields.update_scope,
        })
    }
}

fn present<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// `PATCH .../events/:id/cancel`; unknown/blank scope falls back to this event.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct CancelEvent {
    pub cancel_scope: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn event_update_preserves_omitted_null_and_supplied_controls() {
        let omitted: UpdateEvent = serde_json::from_value(json!({})).unwrap();
        assert_eq!(omitted, UpdateEvent::default());
        assert_eq!(serde_json::to_value(&omitted).unwrap(), json!({}));

        let cleared = json!({
            "description": null, "startsAt": null, "endsAt": null,
            "venueRoomId": null, "recurrenceRule": null, "recurrenceUntil": null
        });
        let update: UpdateEvent = serde_json::from_value(cleared.clone()).unwrap();
        assert_eq!(update.description, Some(None));
        assert_eq!(update.starts_at, Some(None));
        assert_eq!(update.ends_at, Some(None));
        assert_eq!(update.venue_room_id, Some(None));
        assert_eq!(update.recurrence_rule, Some(None));
        assert_eq!(update.recurrence_until, Some(None));
        assert_eq!(serde_json::to_value(update).unwrap(), cleared);

        let supplied = json!({
            "title": "Weekly planning", "description": "Bring notes",
            "startsAt": "2026-10-08T10:00", "endsAt": "2026-10-08T11:00",
            "timeZone": "America/New_York", "venueRoomId": 12,
            "recurrenceRule": "weekly", "recurrenceUntil": "2026-11-08",
            "meetLinkRequested": false, "updateScope": "this_and_following"
        });
        let update: UpdateEvent = serde_json::from_value(supplied.clone()).unwrap();
        assert_eq!(update.title.as_deref(), Some("Weekly planning"));
        assert_eq!(update.venue_room_id, Some(Some(12)));
        assert_eq!(update.meet_link_requested, Some(false));
        assert_eq!(serde_json::to_value(update).unwrap(), supplied);
    }

    #[test]
    fn event_write_defaults_leave_validation_to_the_domain() {
        let create: CreateEvent = serde_json::from_value(json!({})).unwrap();
        assert_eq!(create, CreateEvent::default());
        let create: CreateEvent = serde_json::from_value(json!({
            "recurrenceRule": "unsupported", "startsAt": "bad date"
        }))
        .unwrap();
        assert_eq!(create.recurrence_rule.as_deref(), Some("unsupported"));
        assert_eq!(create.starts_at.as_deref(), Some("bad date"));
        let cancel: CancelEvent = serde_json::from_value(json!({"cancelScope":"unknown"})).unwrap();
        assert_eq!(cancel.cancel_scope.as_deref(), Some("unknown"));
    }
}
