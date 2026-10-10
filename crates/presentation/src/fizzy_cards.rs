// `fizzy/cards/{card,chip}` and `rooms/fizzy/cards/show`: view facts, no SQL/client state.
use crate::{helpers as h, time::Zone};
use serde_json::Value;

pub struct Frame<'a> {
    pub account: &'a str,
    pub number: i64,
    pub web_url: &'a str,
    pub id: &'a str,
    pub connect: bool,
    pub payload: Option<&'a Value>,
    pub error: Option<&'a str>,
    pub zone: &'a Zone,
}
pub fn blank(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => h::is_blank(s),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}
pub fn truthy(value: &Value) -> bool {
    !matches!(value, Value::Null | Value::Bool(false))
}
pub fn text(value: &Value) -> String {
    campfire_richtext::ruby::json_value_to_s(value)
}
/// Time.zone.parse accepts civil dates/times in the viewer's zone as well as offset timestamps.
pub fn last_active_at(zone: &Zone, value: &Value) -> Option<jiff::Timestamp> {
    let value = text(value);
    let value = value.trim();
    if let Ok(at) = value.parse::<jiff::Timestamp>() {
        return Some(at);
    }
    let local = value.parse::<jiff::civil::DateTime>().ok().or_else(|| {
        value
            .parse::<jiff::civil::Date>()
            .ok()
            .map(|date| date.at(0, 0, 0, 0))
    });
    local?
        .to_zoned(zone.tz().clone())
        .ok()
        .map(|time| time.timestamp())
}

pub fn escaped(value: &Value) -> String {
    h::escape(&text(value))
}
pub fn array(value: &Value) -> Vec<&Value> {
    match value {
        Value::Null => vec![],
        Value::Array(a) => a.iter().collect(),
        _ => vec![value],
    }
}
pub fn https(url: &str) -> bool {
    // URI::HTTPS normalizes its scheme; an absolute HTTPS URL may use any nonblank host.
    campfire_richtext::uri::parse(url).ok().is_some_and(|uri| {
        uri.scheme
            .as_deref()
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https"))
            && uri.host.is_some_and(|host| !h::is_blank(&host))
    })
}
